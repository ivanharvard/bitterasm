// Hand-verified ground truth for `std/x86_64/impl.basm`'s register-direct
// (Phase 2: `mov`, the six ALU ops, `test`, ModRM `mod=11` only),
// memory-operand (Phase 3: the same instructions' `MemOperand` overloads,
// full ModRM/SIB addressing), control-flow (Phase 4: `jmp`/`jcc`/`call`/
// `ret`, `rel32` relative to the next instruction), and the remaining core
// subset (Phase 5: `shl`/`shr`/`sar`, `lea`, `push`/`pop`) forms: every
// case's expected bytes are computed by hand against the Intel SDM's own
// opcode/ModRM/SIB/REX encoding tables, the same "no independent oracle
// exists yet at this phase" standard `tests/pdp10_encoding.rs` already
// holds itself to (Phase 7 adds a real GNU binutils cross-check). Driven
// through the actual `bitterasm compile` + `bitter encode` CLI binaries,
// not the library directly — same reasoning as `tests/pdp10_encoding.rs`.
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
        // mov rax, 0x12345678, 0 — no REX, opcode=0xB8+rax(0)=0xB8, imm32 LE
        0xB8, 0x78, 0x56, 0x34, 0x12,
        // mov r15, 0x1, 1 — REX.W + REX.B: REX(W=1,R=0,X=0,B=ext(r15)=1)=0x49,
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
        // add rax, 0x11223344, 0 — opcode=0x81, ModRM(mod=11, digit=0, rm=rax=0)=0xC0, imm32 LE
        0x81, 0xC0, 0x44, 0x33, 0x22, 0x11,
        // or rbx, 0x11223344, 0 — ModRM(mod=11, digit=1, rm=rbx=3)=0xCB
        0x81, 0xCB, 0x44, 0x33, 0x22, 0x11,
        // and rcx, 0x11223344, 0 — ModRM(mod=11, digit=4, rm=rcx=1)=0xE1
        0x81, 0xE1, 0x44, 0x33, 0x22, 0x11,
        // sub rdx, 0x11223344, 0 — ModRM(mod=11, digit=5, rm=rdx=2)=0xEA
        0x81, 0xEA, 0x44, 0x33, 0x22, 0x11,
        // xor rsi, 0x11223344, 0 — ModRM(mod=11, digit=6, rm=rsi=6)=0xF6
        0x81, 0xF6, 0x44, 0x33, 0x22, 0x11,
        // cmp rdi, 0x11223344, 0 — ModRM(mod=11, digit=7, rm=rdi=7)=0xFF
        0x81, 0xFF, 0x44, 0x33, 0x22, 0x11,
        // sub r10, 0x5, 1 — REX.W + REX.B: REX(W=1,R=0,X=0,B=ext(r10)=1)=0x49,
        // opcode=0x81, ModRM(mod=11, digit=5, rm=r10&7=2)=0xEA, imm32 LE
        0x49, 0x81, 0xEA, 0x05, 0x00, 0x00, 0x00,
        // test rax, rcx, 0 — opcode=0x85, ModRM(mod=11, reg=rcx=1, rm=rax=0)=0xC8
        0x85, 0xC8,
        // test rax, 0xFF, 0 — opcode=0xF7, ModRM(mod=11, digit=0, rm=rax=0)=0xC0, imm32 LE
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
        // add Mem(rax, 0), 0x7F, 0 — opcode=0x81, ModRM(mod=00, digit=0, rm=0)=0x00, imm32 LE
        0x81, 0x00, 0x7F, 0x00, 0x00, 0x00,
        // mov Mem(rbx, 4), 0x2A, 0 — 0xC7 /0 (distinct from the register form's
        // 0xB8+reg — no register to fold into the opcode when the destination is
        // memory): ModRM(mod=01, digit=0, rm=rbx=3)=0x43, disp8=4, imm32 LE of 0x2A
        0xC7, 0x43, 0x04, 0x2A, 0x00, 0x00, 0x00,
        // test Mem(rax, 0), 0xFF, 0 — opcode=0xF7, ModRM(mod=00, digit=0, rm=0)=0x00, imm32 LE
        0xF7, 0x00, 0xFF, 0x00, 0x00, 0x00,
        // mov rax, Mem(rbx, 8), 0 — load direction (0x8B), added in Phase 6:
        // ModRM(mod=01, reg=rax=0, rm=rbx=3)=0x43, disp8=8
        0x8B, 0x43, 0x08,
        // mov r9, MemIndexed(rax, rcx, 4, 0x100), 1 — w=1 + extended reg field:
        // REX(W=1,R=ext(r9)=1,X=0,B=0)=0x4C, opcode=0x8B,
        // ModRM(mod=10, reg=r9&7=1, rm=100)=0x8C, SIB(scale=10, index=rcx&7=1, base=rax&7=0)=0x88,
        // disp32 LE of 0x100 (256 doesn't fit in disp8)
        0x4C, 0x8B, 0x8C, 0x88, 0x00, 0x01, 0x00, 0x00,
    ];

    assert_case_encodes_to("regmem", expected);
}

#[test]
fn control_flow_encodes_correctly() {
    // Byte offsets of each entry in `control_flow.basm`, computed from each
    // instruction's own encoded length (used below to hand-verify every
    // rel32): 0 (mov, 2B) -> 2 (mov imm, 5B) -> 7 (jmp, 5B) -> 12 (add, 3B) ->
    // 15 (mov, 3B, this is `target:`) -> 18 (je, 6B) -> 24 (call, 5B) -> 29 (ret, 1B) ->
    // 30 (syscall, 2B) -> 32 (end). `ret`/`syscall` both trail every label and
    // branch target, so appending `syscall` doesn't disturb any rel32 above.
    #[rustfmt::skip]
    let expected: &[u8] = &[
        // mov rax, rbx, 0 — 89 D8 (see regdirect_encodes_correctly)
        0x89, 0xD8,
        // mov rax, 0x12345678, 0 — B8 78 56 34 12 (see regdirect_encodes_correctly)
        0xB8, 0x78, 0x56, 0x34, 0x12,
        // jmp target — opcode=0xE9, rel32 = target_addr(15) - next_instr_addr(7+5=12) = 3
        0xE9, 0x03, 0x00, 0x00, 0x00,
        // add rcx, rdx, 1 — w=1: REX(W=1,R=0,X=0,B=0)=0x48, opcode=0x01,
        // ModRM(mod=11, reg=rdx=2, rm=rcx=1)=0xD1
        0x48, 0x01, 0xD1,
        // mov r8, r9, 0 — 45 89 C8 (see regdirect_encodes_correctly; this is `target:`, byte offset 15)
        0x45, 0x89, 0xC8,
        // je start — opcode=0x0F 0x84 (tttn=0100=E/Z), rel32 = target_addr(0) -
        // next_instr_addr(18+6=24) = -24 = 0xFFFFFFE8 LE
        0x0F, 0x84, 0xE8, 0xFF, 0xFF, 0xFF,
        // call target — opcode=0xE8, rel32 = target_addr(15) - next_instr_addr(24+5=29) = -14 = 0xFFFFFFF2 LE
        0xE8, 0xF2, 0xFF, 0xFF, 0xFF,
        // ret — 0xC3
        0xC3,
        // syscall — 0F 05
        0x0F, 0x05,
    ];

    assert_case_encodes_to("control_flow", expected);
}

#[test]
fn shift_lea_stack_encodes_correctly() {
    #[rustfmt::skip]
    let expected: &[u8] = &[
        // shl rax, 4, 0 — opcode=0xC1, ModRM(mod=11, digit=4 "SHL", rm=rax=0)=0xE0, imm8=4
        0xC1, 0xE0, 0x04,
        // shr rcx, 1, 0 — ModRM(mod=11, digit=5 "SHR", rm=rcx=1)=0xE9, imm8=1
        0xC1, 0xE9, 0x01,
        // sar r10, 3, 1 — w=1 + extended rm: REX(W=1,R=0,X=0,B=ext(r10)=1)=0x49,
        // opcode=0xC1, ModRM(mod=11, digit=7 "SAR", rm=r10&7=2)=0xFA, imm8=3
        0x49, 0xC1, 0xFA, 0x03,
        // shl_cl rbx, 0 — opcode=0xD3 (no immediate — count comes from CL),
        // ModRM(mod=11, digit=4, rm=rbx=3)=0xE3
        0xD3, 0xE3,
        // shr_cl r8, 1 — w=1 + extended rm: REX(W=1,R=0,X=0,B=ext(r8)=1)=0x49,
        // opcode=0xD3, ModRM(mod=11, digit=5, rm=r8&7=0)=0xE8
        0x49, 0xD3, 0xE8,
        // sar_cl rdx, 0 — opcode=0xD3, ModRM(mod=11, digit=7, rm=rdx=2)=0xFA
        0xD3, 0xFA,
        // lea rax, Mem(rbx, 8), 0 — opcode=0x8D, ModRM(mod=01, reg=rax=0, rm=rbx=3)=0x43, disp8=8
        0x8D, 0x43, 0x08,
        // lea r9, MemRipRelative(0x20), 1 — w=1 + extended reg field:
        // REX(W=1,R=ext(r9)=1,X=0,B=0)=0x4C, opcode=0x8D,
        // ModRM(mod=00, reg=r9&7=1, rm=101 "RIP-relative")=0x0D, disp32 LE of 0x20
        0x4C, 0x8D, 0x0D, 0x20, 0x00, 0x00, 0x00,
        // push rax — opcode=0x50+rax(0)=0x50, no REX
        0x50,
        // push r15 — REX.B needed: REX(W=0,R=0,X=0,B=ext(r15)=1)=0x41, opcode=0x50+(r15&7=7)=0x57
        0x41, 0x57,
        // pop rbx — opcode=0x58+rbx(3)=0x5B, no REX
        0x5B,
        // pop r12 — REX.B needed: REX=0x41, opcode=0x58+(r12&7=4)=0x5C
        0x41, 0x5C,
    ];

    assert_case_encodes_to("shift_lea_stack", expected);
}

#[test]
fn byte_mul_div_encodes_correctly() {
    // Cross-checked against GNU objdump (`-b binary -mi386:x86-64 -M intel`),
    // which decodes each line back to the fixture's own instruction. The
    // `_intel` and `_att` fixtures write the same instructions in each dialect.
    #[rustfmt::skip]
    let expected: &[u8] = &[
        // mov al, bl — 0x88 /r, no REX for registers 0-3
        0x88, 0xD8,
        // mov sil, dl — bare REX 0x40 selects sil over dh
        0x40, 0x88, 0xD6,
        // mov r9b, dil — REX.B for r9, which also covers dil
        0x41, 0x88, 0xF9,
        // mov al, 0x41 — 0xB0+reg ib
        0xB0, 0x41,
        // mov dil, 7 — forced REX, 0xB0+7
        0x40, 0xB7, 0x07,
        // mov r10b, 1 — REX.B, 0xB0+2
        0x41, 0xB2, 0x01,
        // mov dl, [rax] — 0x8A /r
        0x8A, 0x10,
        // mov sil, [rbx+8] — forced REX, disp8
        0x40, 0x8A, 0x73, 0x08,
        // mov [rdi], r8b — 0x88 /r, REX.R
        0x44, 0x88, 0x07,
        // mov [rsp], bpl — forced REX, SIB for rsp
        0x40, 0x88, 0x2C, 0x24,
        // mov byte [rax+3], 0x7F — 0xC6 /0 ib
        0xC6, 0x40, 0x03, 0x7F,
        // movzx edx, byte [rax] — 0F B6 /r
        0x0F, 0xB6, 0x10,
        // movzx rax, sil — REX.W before the 0F escape
        0x48, 0x0F, 0xB6, 0xC6,
        // movzx r11, r12b — REX.WRB
        0x4D, 0x0F, 0xB6, 0xDC,
        // movzx rcx, word [rbx+2] — 0F B7 /r
        0x48, 0x0F, 0xB7, 0x4B, 0x02,
        // inc rax — 0xFF /0
        0x48, 0xFF, 0xC0,
        // dec ecx — 0xFF /1
        0xFF, 0xC9,
        // inc r9
        0x49, 0xFF, 0xC1,
        // inc qword [rax]
        0x48, 0xFF, 0x00,
        // dec dword [rbx+rcx*8+16]
        0xFF, 0x4C, 0xCB, 0x10,
        // mul rcx — 0xF7 /4
        0x48, 0xF7, 0xE1,
        // imul r8 — 0xF7 /5
        0x49, 0xF7, 0xE8,
        // div rbx — 0xF7 /6
        0x48, 0xF7, 0xF3,
        // idiv qword [rsp+8] — 0xF7 /7
        0x48, 0xF7, 0x7C, 0x24, 0x08,
        // imul rcx, rdx — 0F AF /r, ModRM.reg = rcx
        0x48, 0x0F, 0xAF, 0xCA,
        // imul r9, [rax] — REX.WR
        0x4C, 0x0F, 0xAF, 0x08,
        // imul rcx, rcx, 10 — 0x6B /r ib
        0x48, 0x6B, 0xC9, 0x0A,
        // imul eax, r15d, 1000 — 0x69 /r id, REX.B only
        0x41, 0x69, 0xC7, 0xE8, 0x03, 0x00, 0x00,
        // cqo, cdq
        0x48, 0x99,
        0x99,
    ];

    assert_case_encodes_to("byte_mul_div", expected);
    assert_case_encodes_to("byte_mul_div_intel", expected);
    assert_case_encodes_to("byte_mul_div_att", expected);
}
