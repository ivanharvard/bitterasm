// Hand-verified ground truth for `std/x86_64/impl.basm`'s register-direct
// (Phase 2: `mov`, the six ALU ops, `test`, ModRM `mod=11` only) and
// memory-operand (Phase 3: the same instructions' `MemOperand` overloads,
// full ModRM/SIB addressing) forms: every case's expected bytes are
// computed by hand against the Intel SDM's own opcode/ModRM/SIB/REX
// encoding tables, the same "no independent oracle exists yet at this
// phase" standard `tests/pdp10_encoding.rs` already holds itself to
// (Phase 7 adds a real GNU binutils cross-check). Driven through the
// actual `bitterasm compile` + `bitter encode` CLI binaries, not the
// library directly — same reasoning as `tests/pdp10_encoding.rs`.
//
// Each fixture's cases are compiled and packed as a single program, so
// each test below compares the whole concatenated byte stream against one
// big hand-computed expected vector, in the fixture's own instruction
// order — there's no independent-oracle harness yet to check individual
// instructions against, and hand-slicing a shared buffer by instruction
// would just duplicate the same math this file already needs to write out
// per case.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn bitter_bin() -> PathBuf {
    static BUILD_ONCE: Once = Once::new();

    let bitterasm = PathBuf::from(env!("CARGO_BIN_EXE_bitterasm"));
    let bin_dir = bitterasm.parent().expect("bitterasm binary should have a parent directory");
    let exe_name = if cfg!(windows) { "bitter.exe" } else { "bitter" };
    let bitter = bin_dir.join(exe_name);

    BUILD_ONCE.call_once(|| {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let status = Command::new("cargo")
            .args(["build", "--package", "bitter", "--quiet"])
            .current_dir(manifest_dir)
            .status()
            .expect("cargo build --package bitter should run");
        assert!(status.success(), "failed to build the `bitter` binary");
    });

    assert!(bitter.is_file(), "expected a `bitter` binary at {}", bitter.display());
    bitter
}

/// Compiles `tests/fixtures/x86_64/{name}.basm`, encodes it with `bitter`,
/// and asserts the resulting bytes match `expected` exactly.
fn assert_case_encodes_to(name: &str, expected: &[u8]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let bitter = bitter_bin();

    let source = Path::new(manifest_dir).join(format!("tests/fixtures/x86_64/{name}.basm"));
    let work_dir = std::env::temp_dir().join("bitterasm-x86_64-cases");
    std::fs::create_dir_all(&work_dir).unwrap();
    let em_path = work_dir.join(format!("{name}.em"));
    let bin_path = work_dir.join(format!("{name}.bin"));

    let compile_status = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &em_path.display().to_string()])
        .status()
        .expect("bitterasm compile should run");
    assert!(compile_status.success(), "bitterasm compile failed for {name}.basm");

    let encode_output = Command::new(&bitter)
        .args(["encode", &em_path.display().to_string(), "-o", &bin_path.display().to_string()])
        .output()
        .expect("bitter encode should run");
    assert!(
        encode_output.status.success(),
        "bitter encode failed for {name}.em:\n{}",
        String::from_utf8_lossy(&encode_output.stderr)
    );

    let bytes = std::fs::read(&bin_path).expect("encoded .bin should exist");
    assert_eq!(bytes, expected, "{name}: expected {expected:02X?}, got {bytes:02X?}");

    std::fs::remove_file(&em_path).ok();
    std::fs::remove_file(&bin_path).ok();
}

#[test]
fn regdirect_encodes_correctly() {
    #[rustfmt::skip]
    let expected: &[u8] = &[
        // mov rax, rbx, 0 — no REX (both operands are low registers, w=0):
        // opcode=0x89, ModRM(mod=11, reg=rbx=3, rm=rax=0)=0xD8
        0x89, 0xD8,
        // mov r8, r9, 0 — REX needed (both r8-r15): REX(W=0,R=ext(r9)=1,X=0,B=ext(r8)=1)=0x45,
        // opcode=0x89, ModRM(mod=11, reg=r9&7=1, rm=r8&7=0)=0xC8
        0x45, 0x89, 0xC8,
        // mov rax, rbx, 1 — REX.W needed: REX(W=1,R=0,X=0,B=0)=0x48,
        // opcode=0x89, ModRM(mod=11, reg=3, rm=0)=0xD8
        0x48, 0x89, 0xD8,
        // movi rax, 0x12345678, 0 — no REX, opcode=0xB8+rax(0)=0xB8, imm32 LE
        0xB8, 0x78, 0x56, 0x34, 0x12,
        // movi r15, 0x1, 1 — REX.W + REX.B: REX(W=1,R=0,X=0,B=ext(r15)=1)=0x49,
        // opcode=0xB8+(r15&7=7)=0xBF, imm64 LE (REX.W widens the immediate itself, not
        // just sign-extension — 0xB8+reg takes a full imm64 under REX.W)
        0x49, 0xBF, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // add rcx, rdx, 0 — opcode=0x01, ModRM(mod=11, reg=rdx=2, rm=rcx=1)=0xD1
        0x01, 0xD1,
        // or rax, rbx, 0 — opcode=0x09, ModRM(mod=11, reg=rbx=3, rm=rax=0)=0xD8
        0x09, 0xD8,
        // and rbx, rcx, 0 — opcode=0x21, ModRM(mod=11, reg=rcx=1, rm=rbx=3)=0xCB
        0x21, 0xCB,
        // sub rdx, rax, 0 — opcode=0x29, ModRM(mod=11, reg=rax=0, rm=rdx=2)=0xC2
        0x29, 0xC2,
        // xor rsi, rdi, 0 — opcode=0x31, ModRM(mod=11, reg=rdi=7, rm=rsi=6)=0xFE
        0x31, 0xFE,
        // cmp rbp, rsp, 0 — opcode=0x39, ModRM(mod=11, reg=rsp=4, rm=rbp=5)=0xE5
        0x39, 0xE5,
        // add r12, r13, 1 — REX.W + both extended: REX(W=1,R=ext(r13)=1,X=0,B=ext(r12)=1)=0x4D,
        // opcode=0x01, ModRM(mod=11, reg=r13&7=5, rm=r12&7=4)=0xEC
        0x4D, 0x01, 0xEC,
        // addi rax, 0x11223344, 0 — opcode=0x81, ModRM(mod=11, digit=0, rm=rax=0)=0xC0, imm32 LE
        0x81, 0xC0, 0x44, 0x33, 0x22, 0x11,
        // ori rbx, 0x11223344, 0 — ModRM(mod=11, digit=1, rm=rbx=3)=0xCB
        0x81, 0xCB, 0x44, 0x33, 0x22, 0x11,
        // andi rcx, 0x11223344, 0 — ModRM(mod=11, digit=4, rm=rcx=1)=0xE1
        0x81, 0xE1, 0x44, 0x33, 0x22, 0x11,
        // subi rdx, 0x11223344, 0 — ModRM(mod=11, digit=5, rm=rdx=2)=0xEA
        0x81, 0xEA, 0x44, 0x33, 0x22, 0x11,
        // xori rsi, 0x11223344, 0 — ModRM(mod=11, digit=6, rm=rsi=6)=0xF6
        0x81, 0xF6, 0x44, 0x33, 0x22, 0x11,
        // cmpi rdi, 0x11223344, 0 — ModRM(mod=11, digit=7, rm=rdi=7)=0xFF
        0x81, 0xFF, 0x44, 0x33, 0x22, 0x11,
        // subi r10, 0x5, 1 — REX.W + REX.B: REX(W=1,R=0,X=0,B=ext(r10)=1)=0x49,
        // opcode=0x81, ModRM(mod=11, digit=5, rm=r10&7=2)=0xEA, imm32 LE
        0x49, 0x81, 0xEA, 0x05, 0x00, 0x00, 0x00,
        // test rax, rcx, 0 — opcode=0x85, ModRM(mod=11, reg=rcx=1, rm=rax=0)=0xC8
        0x85, 0xC8,
        // testi rax, 0xFF, 0 — opcode=0xF7, ModRM(mod=11, digit=0, rm=rax=0)=0xC0, imm32 LE
        0xF7, 0xC0, 0xFF, 0x00, 0x00, 0x00,
    ];

    assert_case_encodes_to("regdirect", expected);
}

#[test]
fn regmem_encodes_correctly() {
    #[rustfmt::skip]
    let expected: &[u8] = &[
        // mov Mem(rax, 0), rbx, 0 — mod=00, no disp, no REX:
        // opcode=0x89, ModRM(mod=00, reg=rbx=3, rm=rax=0)=0x18
        0x89, 0x18,
        // mov Mem(rax, 8), rbx, 0 — mod=01, disp8:
        // ModRM(mod=01, reg=3, rm=0)=0x58, disp8=8
        0x89, 0x58, 0x08,
        // mov Mem(rax, 1000), rbx, 0 — mod=10, disp32 (1000 doesn't fit in disp8):
        // ModRM(mod=10, reg=3, rm=0)=0x98, disp32 LE of 1000 (0x3E8)
        0x89, 0x98, 0xE8, 0x03, 0x00, 0x00,
        // mov Mem(rsp, 0), rbx, 0 — rsp's low3=100 always forces a SIB byte
        // (ModRM.rm=100 means "read SIB" and can't mean a real base register):
        // ModRM(mod=00, reg=3, rm=100)=0x1C, SIB(scale=00, index=100 "none", base=rsp&7=100)=0x24
        0x89, 0x1C, 0x24,
        // mov Mem(r12, 0), rbx, 0 — same SIB forcing, extended base needs REX.B:
        // REX(W=0,R=0,X=0,B=ext(r12)=1)=0x41, ModRM=0x1C (same rm=100), SIB=0x24 (same low3 as rsp)
        0x41, 0x89, 0x1C, 0x24,
        // mov Mem(rbp, 0), rbx, 0 — rbp's low3=101 with mod=00 would mean
        // RIP-relative, so a literal zero displacement is forced to mod=01 disp8=0:
        // ModRM(mod=01, reg=3, rm=101)=0x5D, disp8=0
        0x89, 0x5D, 0x00,
        // mov Mem(r13, 0), rbx, 0 — same forced disp8=0, extended base needs REX.B:
        // REX(W=0,R=0,X=0,B=ext(r13)=1)=0x41, ModRM=0x5D (same rm=101), disp8=0
        0x41, 0x89, 0x5D, 0x00,
        // mov MemIndexed(rax, rcx, 4, 0), rbx, 0 — base+index*scale, mod=00, no disp:
        // ModRM(mod=00, reg=3, rm=100)=0x1C, SIB(scale=10 "*4", index=rcx&7=001, base=rax&7=000)=0x88
        0x89, 0x1C, 0x88,
        // mov MemIndexed(rbp, rcx, 4, 0), rbx, 0 — base=rbp forces disp8=0 even
        // through a SIB byte (SIB.base=101 with mod=00 means "no base, disp32 only"):
        // ModRM(mod=01, reg=3, rm=100)=0x5C, SIB(scale=10, index=001, base=rbp&7=101)=0x8D, disp8=0
        0x89, 0x5C, 0x8D, 0x00,
        // mov MemIndexed(rax, rcx, 4, 100), rbx, 0 — mod=01, disp8=100:
        // ModRM=0x5C, SIB(scale=10, index=001, base=000)=0x88, disp8=100 (0x64)
        0x89, 0x5C, 0x88, 0x64,
        // mov MemIndexed(rax, rcx, 4, 100000), rbx, 0 — mod=10, disp32 (100000
        // doesn't fit in disp8): ModRM(mod=10, reg=3, rm=100)=0x9C, SIB=0x88,
        // disp32 LE of 100000 (0x186A0)
        0x89, 0x9C, 0x88, 0xA0, 0x86, 0x01, 0x00,
        // mov MemRipRelative(0x100), rbx, 0 — mod=00, rm=101, no SIB, disp32 always
        // (RIP-relative never uses disp8): ModRM(mod=00, reg=3, rm=101)=0x1D,
        // disp32 LE of 0x100
        0x89, 0x1D, 0x00, 0x01, 0x00, 0x00,
        // add Mem(rax, 0), rcx, 1 — w=1 forces REX.W even with no extended
        // registers: REX(W=1,R=0,X=0,B=0)=0x48, opcode=0x01, ModRM(mod=00, reg=rcx=1, rm=0)=0x08
        0x48, 0x01, 0x08,
        // addi Mem(rax, 0), 0x7F, 0 — opcode=0x81, ModRM(mod=00, digit=0, rm=0)=0x00, imm32 LE
        0x81, 0x00, 0x7F, 0x00, 0x00, 0x00,
        // movi Mem(rbx, 4), 0x2A, 0 — 0xC7 /0 (distinct from the register form's
        // 0xB8+reg — no register to fold into the opcode when the destination is
        // memory): ModRM(mod=01, digit=0, rm=rbx=3)=0x43, disp8=4, imm32 LE of 0x2A
        0xC7, 0x43, 0x04, 0x2A, 0x00, 0x00, 0x00,
        // testi Mem(rax, 0), 0xFF, 0 — opcode=0xF7, ModRM(mod=00, digit=0, rm=0)=0x00, imm32 LE
        0xF7, 0x00, 0xFF, 0x00, 0x00, 0x00,
    ];

    assert_case_encodes_to("regmem", expected);
}
