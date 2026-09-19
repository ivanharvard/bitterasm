// Hand-verified ground truth for `std/x86_64/impl.basm`'s Phase 2
// register-direct instructions (ModRM `mod=11` only — no memory operands
// yet, see Phase 3): every case's expected bytes are computed by hand
// against the Intel SDM's own opcode/ModRM/REX encoding tables, the same
// "no independent oracle exists yet at this phase" standard
// `tests/pdp10_encoding.rs` already holds itself to (Phase 7 adds a real
// GNU binutils cross-check). Driven through the actual `bitterasm
// compile` + `bitter encode` CLI binaries, not the library directly —
// same reasoning as `tests/pdp10_encoding.rs`.
//
// All 21 cases in `tests/fixtures/x86_64/regdirect.basm` are compiled and
// packed as a single program, so this is one test comparing the whole
// concatenated byte stream against one big hand-computed expected vector,
// in the fixture's own instruction order — there's no independent-oracle
// harness yet to check individual instructions against, and hand-slicing
// a shared buffer by instruction would just duplicate the same math this
// file already needs to write out per case.

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
