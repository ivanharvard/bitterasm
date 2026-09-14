// Hand-verified ground truth for `std/pdp10/impl.basm`: no independent
// PDP-10 assembler exists to cross-check against the way `tests/riscv`
// cross-checks against real GNU binutils, so each case's expected word is
// computed by hand here (`opcode << 27 | ac << 23 | i << 22 | x << 18 |
// y`, the field layout confirmed against the DEC PDP-10 System Reference
// Manual's own "BASIC INSTRUCTIONS" word-format diagram) and compared
// against real `bitterasm compile` + `bitter encode` output, through the
// actual CLI binaries — the same "drive the real binary, not the library"
// standard `tests/riscv_dialects.rs` already holds itself to.
//
// `bitter` lives in a separate workspace package, so unlike `bitterasm`'s
// own binary, `CARGO_BIN_EXE_bitter` isn't set by Cargo for a test in this
// package (confirmed empirically — Cargo only exposes `CARGO_BIN_EXE_*` for
// binaries in the *same* package as the test). Its path is derived from
// `bitterasm`'s own binary path instead (same target directory, whatever
// profile `cargo test` actually used), and it's built on demand exactly
// once per test-binary process rather than assumed pre-built, so this
// passes under plain `cargo test` and not just `cargo test --workspace`.

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

/// Compiles `tests/pdp10/cases/{name}.basm`, encodes it with `bitter`, and
/// asserts the resulting bytes match `expected` exactly.
fn assert_case_encodes_to(name: &str, expected: &[u8]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let bitter = bitter_bin();

    let source = Path::new(manifest_dir).join(format!("tests/pdp10/cases/{name}.basm"));
    let work_dir = std::env::temp_dir().join("bitterasm-pdp10-cases");
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
    assert_eq!(
        bytes, expected,
        "{name}: expected {expected:02X?}, got {bytes:02X?}"
    );

    std::fs::remove_file(&em_path).ok();
    std::fs::remove_file(&bin_path).ok();
}

// Every instruction's word packs to exactly 5 bytes (36 bits, zero-padded
// at the high end to 40 — `bitter`'s generic non-byte-multiple handling,
// not anything PDP-10-specific), matching `bitter/src/pack.rs`'s own
// `pads_a_36_bit_word_up_to_five_bytes` unit test.

#[test]
fn halt_encodes_correctly() {
    // opcode=0o254, ac=4 (HALT is documented as `JRST 4,`), i=0, x=0, y=0
    assert_case_encodes_to("halt", &[0x05, 0x62, 0x00, 0x00, 0x00]);
}

#[test]
fn jrst_encodes_correctly() {
    // opcode=0o254, ac=0, i=1, x=3, y=100
    assert_case_encodes_to("jrst", &[0x05, 0x60, 0x4C, 0x00, 0x64]);
}

#[test]
fn jumpa_encodes_correctly() {
    // opcode=0o324, ac=0, i=0, x=0, y=200
    assert_case_encodes_to("jumpa", &[0x06, 0xA0, 0x00, 0x00, 0xC8]);
}

#[test]
fn movei_encodes_correctly() {
    // opcode=0o201, ac=1, i=0, x=0, y=42
    assert_case_encodes_to("movei", &[0x04, 0x08, 0x80, 0x00, 0x2A]);
}

#[test]
fn move_encodes_correctly() {
    // opcode=0o200, ac=2, i=0, x=0, y=1000
    assert_case_encodes_to("move", &[0x04, 0x01, 0x00, 0x03, 0xE8]);
}

#[test]
fn movem_encodes_correctly() {
    // opcode=0o202, ac=2, i=0, x=0, y=1000
    assert_case_encodes_to("movem", &[0x04, 0x11, 0x00, 0x03, 0xE8]);
}

#[test]
fn add_encodes_correctly() {
    // opcode=0o270, ac=1, i=0, x=0, y=2
    assert_case_encodes_to("add", &[0x05, 0xC0, 0x80, 0x00, 0x02]);
}

#[test]
fn addi_encodes_correctly() {
    // opcode=0o271, ac=1, i=0, x=0, y=7
    assert_case_encodes_to("addi", &[0x05, 0xC8, 0x80, 0x00, 0x07]);
}

#[test]
fn sub_encodes_correctly() {
    // opcode=0o274, ac=3, i=0, x=0, y=4
    assert_case_encodes_to("sub", &[0x05, 0xE1, 0x80, 0x00, 0x04]);
}

#[test]
fn and_encodes_correctly() {
    // opcode=0o404, ac=5, i=0, x=0, y=0o777
    assert_case_encodes_to("and", &[0x08, 0x22, 0x80, 0x01, 0xFF]);
}

#[test]
fn cain_encodes_correctly() {
    // opcode=0o306, ac=1, i=0, x=0, y=42
    assert_case_encodes_to("cain", &[0x06, 0x30, 0x80, 0x00, 0x2A]);
}

#[test]
fn exch_encodes_correctly() {
    // opcode=0o250, ac=6, i=0, x=0, y=1000
    assert_case_encodes_to("exch", &[0x05, 0x43, 0x00, 0x03, 0xE8]);
}

#[test]
fn all_fields_simultaneously_nonzero_packs_with_no_bleed_across_field_boundaries() {
    // opcode=0o200 (move), ac=15, i=1, x=15, y=0x3FFFF — the case that most
    // directly stresses "even bits are a library concept": ac/i straddle
    // one byte boundary and x/y straddle another once padded into 5 bytes,
    // and nothing in the packing path may assume byte alignment anywhere.
    assert_case_encodes_to("all_fields_nonzero", &[0x04, 0x07, 0xFF, 0xFF, 0xFF]);
}
