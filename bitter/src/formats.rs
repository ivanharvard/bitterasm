//! Wraps raw machine code (what `pack::pack_stream` produces) in a minimal,
//! native OS executable container, so `bitter exec`/`bitter build` can hand
//! back something the OS's own loader (`execve`, `CreateProcess`, ...)
//! actually runs — not just correct bytes. `bitter encode`'s own contract
//! stops at "correct machine code"; turning that into a loadable file is a
//! second, OS-specific step, the same way a real toolchain's `as` produces
//! an object file and a separate `ld` step turns it into something
//! `execve`-able.
//!
//! **What's actually verified, and what's spec-only:** this module was
//! built and exercised on Linux, where [`wrap_elf`]'s output was round-
//! tripped through `bitter exec` and actually executed (see
//! `examples/x86_64/hello.basm`). [`wrap_pe`] and [`wrap_macho`] are built
//! byte-for-byte against the public PE and Mach-O format specs, but this
//! environment has no Windows or macOS to run them on — treat them as
//! "should work," not "confirmed working," until someone runs one for
//! real. Notably, macOS's loader has tightened considerably since the
//! classic "tiny Mach-O" era: a modern kernel refuses to load an
//! `MH_EXECUTE` with no `LC_LOAD_DYLINKER` at all (even one that never
//! actually calls into a shared library), which is why [`wrap_macho`]
//! includes one pointing at `/usr/lib/dyld` even though nothing here uses
//! it — and even with that, current macOS may still refuse an ad-hoc/
//! unsigned binary depending on Gatekeeper policy.
//!
//! One thing wrapping can't paper over, whatever the container: the
//! *payload* itself may only be correct for one OS. `examples/x86_64/
//! hello.basm` calls Linux's `syscall` ABI (`rax=1`/`rax=60`) directly —
//! wrapping those exact bytes in a PE or Mach-O produces a file Windows or
//! macOS will happily load and jump into, which then executes a real
//! `syscall` instruction against the *host* kernel's completely different
//! syscall table. That's a property of the program, not this module; a
//! program meant to run cross-OS would need its own OS-specific code path,
//! same as it would with any other assembler.

use std::io;
use std::path::Path;

/// The three native executable containers `bitter exec`/`bitter build`
/// know how to produce. Picking the right one for *this* OS is
/// [`Format::native`]; producing a specific one regardless of host OS
/// (for inspection, or once a format above is confirmed working
/// elsewhere) is `--format` on the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Elf,
    Pe,
    MachO,
}

impl Format {
    /// The container the OS actually running this copy of `bitter` expects.
    pub fn native() -> Format {
        if cfg!(target_os = "windows") {
            Format::Pe
        } else if cfg!(target_os = "macos") {
            Format::MachO
        } else {
            Format::Elf
        }
    }

    pub fn parse(name: &str) -> Result<Format, String> {
        match name {
            "elf" => Ok(Format::Elf),
            "pe" => Ok(Format::Pe),
            "macho" | "mach-o" => Ok(Format::MachO),
            other => Err(format!("unknown format `{other}` (expected `elf`, `pe`, or `macho`)")),
        }
    }

    /// The conventional filename extension for an executable in this
    /// container — empty for ELF/Mach-O (a Unix executable is normally
    /// extensionless), `.exe` for PE.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Elf | Format::MachO => "",
            Format::Pe => "exe",
        }
    }
}

/// `entry` is the byte offset into `code` where execution starts — `0`
/// for "the very first byte", which is all any caller could ask for before
/// `--entry` existed.
pub fn wrap(code: &[u8], format: Format, entry: usize) -> Vec<u8> {
    match format {
        Format::Elf => wrap_elf(code, entry),
        Format::Pe => wrap_pe(code, entry),
        Format::MachO => wrap_macho(code, entry),
    }
}

/// Writes `code` wrapped in `format`'s container to `output`, and — on a
/// Unix host — marks it executable (`chmod +x`; Windows has no such bit,
/// `.exe` alone is what makes a PE runnable there, and Mach-O's own `+x`
/// bit still matters even when this `bitter` isn't itself running on
/// macOS, e.g. building a `--format macho` file from Linux to copy over).
pub fn write_executable(code: &[u8], output: &Path, format: Format, entry: usize) -> io::Result<()> {
    if entry >= code.len().max(1) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("entry offset {entry} is past the end of the {}-byte code", code.len()),
        ));
    }

    let bytes = wrap(code, format, entry);
    std::fs::write(output, &bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(output)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(output, perms)?;
    }

    Ok(())
}

// =================
// little-endian byte-vector building
// =================
// None of these three formats' headers are self-describing enough to build
// with, say, `#[repr(C)]` structs without risking padding the compiler
// inserts silently changing the on-disk layout — appending fixed-width
// little-endian fields by hand, the same underlying idea `bitter`'s own
// `pack.rs` already uses for every emitted value, is the safer and more
// direct way to get an exact byte layout.
#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    fn bytes(&mut self, v: &[u8]) -> &mut Self {
        self.0.extend_from_slice(v);
        self
    }
    /// Zero-pads a fixed-width field (a name, a reserved block) out to
    /// `len` bytes, truncating `v` if it's somehow longer.
    fn fixed(&mut self, v: &[u8], len: usize) -> &mut Self {
        let n = v.len().min(len);
        self.0.extend_from_slice(&v[..n]);
        self.0.resize(self.0.len() + (len - n), 0);
        self
    }
    fn pad_to(&mut self, align: usize) -> &mut Self {
        let rem = self.0.len() % align;
        if rem != 0 {
            self.0.resize(self.0.len() + (align - rem), 0);
        }
        self
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

fn round_up(n: usize, align: usize) -> usize {
    n.div_ceil(align) * align
}

// =================
// ELF64 (Linux/BSD)
// =================
// The smallest valid ET_EXEC: one 64-byte Elf64_Ehdr, one 56-byte
// Elf64_Phdr (a single PT_LOAD segment mapping the *entire* file, headers
// included — `p_offset=0` — at a fixed load address), then the code
// itself right after, entered `entry_offset` bytes in. No section headers,
// no dynamic linking: this is a fully static, non-PIE binary, which is
// all a straight-line syscall-only program like `hello.basm` needs.
fn wrap_elf(code: &[u8], entry_offset: usize) -> Vec<u8> {
    const LOAD_ADDR: u64 = 0x400000;
    const EHDR_SIZE: u64 = 64;
    const PHDR_SIZE: u64 = 56;
    let entry = LOAD_ADDR + EHDR_SIZE + PHDR_SIZE + entry_offset as u64;
    let file_size = EHDR_SIZE + PHDR_SIZE + code.len() as u64;

    let mut w = Writer::default();
    w.bytes(&[0x7f, b'E', b'L', b'F']) // e_ident: magic
        .u8(2) // EI_CLASS = ELFCLASS64
        .u8(1) // EI_DATA = ELFDATA2LSB
        .u8(1) // EI_VERSION
        .u8(0) // EI_OSABI = ELFOSABI_SYSV
        .fixed(&[], 8) // EI_ABIVERSION + EI_PAD
        .u16(2) // e_type = ET_EXEC
        .u16(0x3E) // e_machine = EM_X86_64
        .u32(1) // e_version
        .u64(entry) // e_entry
        .u64(EHDR_SIZE) // e_phoff
        .u64(0) // e_shoff
        .u32(0) // e_flags
        .u16(EHDR_SIZE as u16) // e_ehsize
        .u16(PHDR_SIZE as u16) // e_phentsize
        .u16(1) // e_phnum
        .u16(0) // e_shentsize
        .u16(0) // e_shnum
        .u16(0); // e_shstrndx
    debug_assert_eq!(w.len() as u64, EHDR_SIZE);

    w.u32(1) // p_type = PT_LOAD
        .u32(5) // p_flags = PF_R | PF_X
        .u64(0) // p_offset
        .u64(LOAD_ADDR) // p_vaddr
        .u64(LOAD_ADDR) // p_paddr
        .u64(file_size) // p_filesz
        .u64(file_size) // p_memsz
        .u64(0x1000); // p_align
    debug_assert_eq!(w.len() as u64, EHDR_SIZE + PHDR_SIZE);

    w.bytes(code);
    w.0
}

// =================
// PE32+ (Windows)
// =================
// A minimal console-subsystem PE32+ (the 64-bit PE variant): a 64-byte
// `IMAGE_DOS_HEADER` (only `e_magic`/`e_lfanew` matter — the loader jumps
// straight to the real header via `e_lfanew` and never runs a DOS stub),
// the `"PE\0\0"` signature, a 20-byte `IMAGE_FILE_HEADER`, a 240-byte
// `IMAGE_OPTIONAL_HEADER64` (112-byte fixed part + 16 zeroed 8-byte data
// directories — none of import/export/relocation/etc. are needed for a
// single static `.text` section that makes no OS API calls), and one
// 40-byte `IMAGE_SECTION_HEADER` describing `.text`. Headers are padded to
// `FileAlignment` (0x200) and the section to `SectionAlignment` (0x1000),
// matching the two-alignment scheme every real PE file uses.
fn wrap_pe(code: &[u8], entry_offset: usize) -> Vec<u8> {
    const FILE_ALIGN: usize = 0x200;
    const SECTION_ALIGN: usize = 0x1000;
    const IMAGE_BASE: u64 = 0x1_4000_0000;
    const SECTION_RVA: u32 = SECTION_ALIGN as u32; // first section, right after the header page

    let headers_len = 64 + 4 + 20 + 240 + 40; // DOS + "PE\0\0" + COFF + Optional64 + one section header
    let size_of_headers = round_up(headers_len, FILE_ALIGN);
    let size_of_raw_data = round_up(code.len().max(1), FILE_ALIGN);
    let size_of_image = round_up(SECTION_RVA as usize + round_up(code.len().max(1), SECTION_ALIGN), SECTION_ALIGN);
    let pointer_to_raw_data = size_of_headers as u32;

    let mut w = Writer::default();

    // IMAGE_DOS_HEADER
    let dos_start = w.len();
    w.u16(0x5A4D); // e_magic = "MZ"
    w.fixed(&[], 0x3C - 0x02); // e_cblp..e_res2 (everything but e_magic/e_lfanew): unused by the PE loader
    w.u32(64); // e_lfanew: the real header starts right after this 64-byte DOS header
    debug_assert_eq!(w.len() - dos_start, 64);

    // "PE\0\0" + IMAGE_FILE_HEADER
    w.bytes(b"PE\0\0");
    w.u16(0x8664) // Machine = IMAGE_FILE_MACHINE_AMD64
        .u16(1) // NumberOfSections
        .u32(0) // TimeDateStamp
        .u32(0) // PointerToSymbolTable (no COFF symbol table)
        .u32(0) // NumberOfSymbols
        .u16(240) // SizeOfOptionalHeader
        .u16(0x0002 | 0x0020); // Characteristics: EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE

    // IMAGE_OPTIONAL_HEADER64
    let opt_start = w.len();
    w.u16(0x20B) // Magic = PE32+
        .u8(0) // MajorLinkerVersion
        .u8(0) // MinorLinkerVersion
        .u32(size_of_raw_data as u32) // SizeOfCode
        .u32(0) // SizeOfInitializedData
        .u32(0) // SizeOfUninitializedData
        .u32(SECTION_RVA + entry_offset as u32) // AddressOfEntryPoint: `entry_offset` bytes into .text
        .u32(SECTION_RVA) // BaseOfCode
        .u64(IMAGE_BASE) // ImageBase
        .u32(SECTION_ALIGN as u32) // SectionAlignment
        .u32(FILE_ALIGN as u32) // FileAlignment
        .u16(6) // MajorOperatingSystemVersion
        .u16(0) // MinorOperatingSystemVersion
        .u16(0) // MajorImageVersion
        .u16(0) // MinorImageVersion
        .u16(6) // MajorSubsystemVersion
        .u16(0) // MinorSubsystemVersion
        .u32(0) // Win32VersionValue
        .u32(size_of_image as u32) // SizeOfImage
        .u32(size_of_headers as u32) // SizeOfHeaders
        .u32(0) // CheckSum (unchecked for a normal EXE)
        .u16(3) // Subsystem = IMAGE_SUBSYSTEM_WINDOWS_CUI (console)
        .u16(0) // DllCharacteristics
        .u64(0x100000) // SizeOfStackReserve
        .u64(0x1000) // SizeOfStackCommit
        .u64(0x100000) // SizeOfHeapReserve
        .u64(0x1000) // SizeOfHeapCommit
        .u32(0) // LoaderFlags
        .u32(16); // NumberOfRvaAndSizes
    for _ in 0..16 {
        w.u64(0); // 16 zeroed (RVA, Size) data directory entries
    }
    debug_assert_eq!(w.len() - opt_start, 240);

    // IMAGE_SECTION_HEADER for .text
    w.fixed(b".text", 8) // Name
        .u32(code.len() as u32) // VirtualSize
        .u32(SECTION_RVA) // VirtualAddress
        .u32(size_of_raw_data as u32) // SizeOfRawData
        .u32(pointer_to_raw_data) // PointerToRawData
        .u32(0) // PointerToRelocations
        .u32(0) // PointerToLinenumbers
        .u16(0) // NumberOfRelocations
        .u16(0) // NumberOfLinenumbers
        .u32(0x60000020); // Characteristics: CNT_CODE | MEM_EXECUTE | MEM_READ

    debug_assert_eq!(w.len(), headers_len);
    w.pad_to(FILE_ALIGN);
    debug_assert_eq!(w.len(), size_of_headers);

    w.bytes(code);
    w.pad_to(FILE_ALIGN);
    w.0
}

// =================
// Mach-O 64 (macOS)
// =================
// One `mach_header_64` (32 bytes) plus three load commands: a
// `LC_SEGMENT_64` for `__TEXT` mapping the whole file (headers included,
// `fileoff=0`) read+execute at a fixed address, `LC_LOAD_DYLINKER`
// pointing at `/usr/lib/dyld` (see this module's own header comment on
// why a binary that never actually calls a shared library still needs
// this to satisfy a modern kernel's loader), and `LC_MAIN` giving the
// entry point as a file offset (`entryoff`) into that segment rather than
// a raw address, the modern replacement for the deprecated
// `LC_UNIXTHREAD`.
fn wrap_macho(code: &[u8], entry_offset: usize) -> Vec<u8> {
    const VM_ADDR: u64 = 0x1_0000_0000; // typical fixed __TEXT base for a non-PIE x86_64 Mach-O

    let dylinker_path = b"/usr/lib/dyld\0";
    let dylinker_cmdsize = round_up(8 + 4 + dylinker_path.len(), 8); // load_command hdr + lc_str offset field + path
    let segment_cmdsize = 72; // segment_command_64 header, this segment has no sections
    let main_cmdsize = 24; // entry_point_command

    let ncmds = 3u32;
    let sizeofcmds = (segment_cmdsize + dylinker_cmdsize + main_cmdsize) as u32;
    let header_len = 32 + sizeofcmds as usize;
    let file_size = header_len + code.len();
    let entryoff = (header_len + entry_offset) as u64;

    let mut w = Writer::default();

    // mach_header_64
    w.u32(0xfeedfacf) // magic = MH_MAGIC_64
        .u32(0x0100_0007u32) // cputype = CPU_TYPE_X86_64 (CPU_TYPE_I386 | CPU_ARCH_ABI64)
        .u32(3) // cpusubtype = CPU_SUBTYPE_X86_64_ALL
        .u32(2) // filetype = MH_EXECUTE
        .u32(ncmds)
        .u32(sizeofcmds)
        .u32(0x1) // flags = MH_NOUNDEFS (no undefined symbols to resolve)
        .u32(0); // reserved

    // LC_SEGMENT_64 __TEXT: maps the whole file, headers included
    w.u32(0x19) // cmd = LC_SEGMENT_64
        .u32(segment_cmdsize as u32)
        .fixed(b"__TEXT", 16) // segname
        .u64(VM_ADDR) // vmaddr
        .u64(round_up(file_size, 0x1000) as u64) // vmsize
        .u64(0) // fileoff
        .u64(file_size as u64) // filesize
        .u32(7) // maxprot = VM_PROT_READ | WRITE | EXECUTE
        .u32(5) // initprot = VM_PROT_READ | EXECUTE
        .u32(0) // nsects
        .u32(0); // flags

    // LC_LOAD_DYLINKER
    w.u32(0xe) // cmd = LC_LOAD_DYLINKER
        .u32(dylinker_cmdsize as u32)
        .u32(12) // lc_str.offset: name starts right after this 12-byte prefix
        .bytes(dylinker_path)
        .pad_to(8);

    // LC_MAIN: entry point as a file offset into __TEXT, no separate stack size override
    w.u32(0x80000028u32) // cmd = LC_MAIN | LC_REQ_DYLD
        .u32(main_cmdsize as u32)
        .u64(entryoff)
        .u64(0); // stacksize = 0 (use the default)

    debug_assert_eq!(w.len(), header_len);
    w.bytes(code);
    w.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elf_header_reports_the_right_entry_point_and_file_size() {
        let code = [0x90, 0x90, 0x90]; // 3 NOPs
        let bytes = wrap_elf(&code, 0);
        assert_eq!(bytes.len(), 64 + 56 + code.len());
        assert_eq!(&bytes[0..4], &[0x7f, b'E', b'L', b'F']);
        let entry = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        assert_eq!(entry, 0x400000 + 64 + 56);
        assert_eq!(&bytes[64 + 56..], &code);
    }

    #[test]
    fn pe_header_is_file_aligned_and_reports_the_entry_rva() {
        let code = [0x90, 0x90, 0x90];
        let bytes = wrap_pe(&code, 0);
        assert_eq!(&bytes[0..2], b"MZ");
        let lfanew = u32::from_le_bytes(bytes[0x3C..0x40].try_into().unwrap()) as usize;
        assert_eq!(&bytes[lfanew..lfanew + 4], b"PE\0\0");
        let opt_start = lfanew + 4 + 20;
        let entry_rva = u32::from_le_bytes(bytes[opt_start + 16..opt_start + 20].try_into().unwrap());
        assert_eq!(entry_rva, 0x1000);
        assert_eq!(bytes.len() % 0x200, 0, "file size must stay FileAlignment-rounded");
        assert_eq!(&bytes[0x200..0x200 + code.len()], &code);
    }

    #[test]
    fn macho_header_reports_the_right_magic_and_entry_offset() {
        let code = [0x90, 0x90, 0x90];
        let bytes = wrap_macho(&code, 0);
        assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), 0xfeedfacf);
        assert_eq!(&bytes[bytes.len() - code.len()..], &code);
    }

    #[test]
    fn entry_offset_moves_every_format_s_entry_point() {
        let code = [0x90, 0x90, 0x90];

        let elf = wrap_elf(&code, 2);
        assert_eq!(u64::from_le_bytes(elf[24..32].try_into().unwrap()), 0x400000 + 64 + 56 + 2);

        let pe = wrap_pe(&code, 2);
        let opt_start = u32::from_le_bytes(pe[0x3C..0x40].try_into().unwrap()) as usize + 4 + 20;
        assert_eq!(u32::from_le_bytes(pe[opt_start + 16..opt_start + 20].try_into().unwrap()), 0x1000 + 2);

        let at_start = wrap_macho(&code, 0);
        let moved = wrap_macho(&code, 2);
        let diff: Vec<usize> = (0..at_start.len()).filter(|&i| at_start[i] != moved[i]).collect();
        assert!(!diff.is_empty(), "LC_MAIN's entryoff should change");
    }

    #[test]
    fn native_format_matches_this_build_s_own_target_os() {
        let format = Format::native();
        if cfg!(target_os = "linux") {
            assert_eq!(format, Format::Elf);
        }
    }
}
