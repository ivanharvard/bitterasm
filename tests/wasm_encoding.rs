// Hand-verified ground truth for `std/wasm/impl.basm` and
// `std/wasm/leb128.basm`: expected bytes below are computed by hand against
// the WebAssembly binary format's own opcode table (stable since the MVP)
// and the standard LEB128 encoding algorithm, then compared against real
// `bitterasm compile` + `bitter encode` output through the actual CLI
// binaries — the same "drive the real binary, not the library" standard
// `tests/riscv_dialects.rs` and `tests/pdp10_encoding.rs` already hold
// themselves to.
//
// See those files' own comments for why `bitter`'s binary path has to be
// derived from `bitterasm`'s rather than read from `CARGO_BIN_EXE_bitter`
// (it lives in a separate workspace package), and why it's built on demand
// rather than assumed pre-built.

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

/// Compiles `tests/wasm/cases/{name}.basm`, encodes it with `bitter`, and
/// asserts the resulting bytes match `expected` exactly.
fn assert_case_encodes_to(name: &str, expected: &[u8]) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let bitter = bitter_bin();

    let source = Path::new(manifest_dir).join(format!("tests/wasm/cases/{name}.basm"));
    let work_dir = std::env::temp_dir().join("bitterasm-wasm-cases");
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

// =================
// control instructions
// =================

#[test]
fn unreachable_encodes_correctly() {
    assert_case_encodes_to("unreachable", &[0x00]);
}

#[test]
fn nop_encodes_correctly() {
    assert_case_encodes_to("nop", &[0x01]);
}

#[test]
fn block_encodes_correctly() {
    assert_case_encodes_to("block", &[0x02, 0x40]);
}

#[test]
fn block_with_a_value_type_encodes_correctly() {
    // blocktype = I32 (0x7F), not the empty blocktype (0x40).
    assert_case_encodes_to("block_i32", &[0x02, 0x7F]);
}

#[test]
fn loop_encodes_correctly() {
    assert_case_encodes_to("loop", &[0x03, 0x40]);
}

#[test]
fn if_encodes_correctly() {
    assert_case_encodes_to("if", &[0x04, 0x40]);
}

#[test]
fn else_encodes_correctly() {
    assert_case_encodes_to("else", &[0x05]);
}

#[test]
fn end_encodes_correctly() {
    assert_case_encodes_to("end", &[0x0B]);
}

#[test]
fn br_encodes_correctly() {
    assert_case_encodes_to("br", &[0x0C, 0x00]);
}

#[test]
fn br_if_encodes_correctly() {
    assert_case_encodes_to("br_if", &[0x0D, 0x01]);
}

#[test]
fn return_encodes_correctly() {
    assert_case_encodes_to("return", &[0x0F]);
}

#[test]
fn call_encodes_correctly() {
    assert_case_encodes_to("call", &[0x10, 0x03]);
}

#[test]
fn call_with_a_large_index_needs_two_leb128_bytes() {
    // 300 = 0b1_0010_1100 -> low 7 bits 0x2C (continuation set: 0xAC),
    // remaining 2 -> 0x02.
    assert_case_encodes_to("call_large_index", &[0x10, 0xAC, 0x02]);
}

// =================
// parametric instructions
// =================

#[test]
fn drop_encodes_correctly() {
    assert_case_encodes_to("drop", &[0x1A]);
}

// =================
// variable instructions
// =================

#[test]
fn local_get_encodes_correctly() {
    assert_case_encodes_to("local_get", &[0x20, 0x02]);
}

#[test]
fn local_set_encodes_correctly() {
    assert_case_encodes_to("local_set", &[0x21, 0x01]);
}

#[test]
fn local_tee_encodes_correctly() {
    assert_case_encodes_to("local_tee", &[0x22, 0x00]);
}

// =================
// numeric instructions
// =================

#[test]
fn i32_const_zero_encodes_correctly() {
    assert_case_encodes_to("i32_const_zero", &[0x41, 0x00]);
}

#[test]
fn i32_const_small_encodes_correctly() {
    // 63 fits in a single SLEB128 byte (its sign bit, 0x40, is already 0).
    assert_case_encodes_to("i32_const_small", &[0x41, 0x3F]);
}

#[test]
fn i32_const_negative_encodes_correctly() {
    // -5 as SLEB128: -5 & 0x7F = 0x7B, and -5 >> 7 == -1 with 0x7B's sign
    // bit (0x40) already set, so encoding terminates in a single byte.
    assert_case_encodes_to("i32_const_negative", &[0x41, 0x7B]);
}

#[test]
fn i32_const_multibyte_encodes_correctly() {
    // 300 needs two SLEB128 bytes for the same reason it needs two ULEB128
    // bytes (its low 7-bit group alone can't hold it), but note this is a
    // *different* encoding from `call_large_index`'s ULEB128 0xAC 0x02 only
    // by coincidence of this particular value being positive and small
    // enough that signed/unsigned grouping happen to agree here.
    assert_case_encodes_to("i32_const_multibyte", &[0x41, 0xAC, 0x02]);
}

#[test]
fn i32_eqz_encodes_correctly() {
    assert_case_encodes_to("i32_eqz", &[0x45]);
}

#[test]
fn i32_lt_s_encodes_correctly() {
    assert_case_encodes_to("i32_lt_s", &[0x48]);
}

#[test]
fn i32_add_encodes_correctly() {
    assert_case_encodes_to("i32_add", &[0x6A]);
}

#[test]
fn i32_sub_encodes_correctly() {
    assert_case_encodes_to("i32_sub", &[0x6B]);
}

#[test]
fn i32_mul_encodes_correctly() {
    assert_case_encodes_to("i32_mul", &[0x6C]);
}

// =================
// composite: a real function body
// =================

#[test]
fn sum_loop_encodes_as_a_flat_concatenation_of_its_instructions() {
    // `tests/wasm/cases/sum_loop.basm`: sums 1..n-1 into local 1, using
    // local 2 as the loop counter and local 0 as the implicit parameter.
    // Exercises structured control flow (`block`+`loop`+`br`/`br_if` by
    // explicit relative depth, no address-based labels at all) and
    // multi-byte LEB128 immediates interleaved with single-byte opcodes,
    // all through the same flat top-level `@emit` stream RISC-V/PDP-10
    // already use — `bitter`'s `pack_stream` concatenates every one of
    // this file's 24 separate instruction invocations in order with no
    // WASM-specific evaluator support at all.
    assert_case_encodes_to(
        "sum_loop",
        &[
            0x41, 0x00, // i32.const 0
            0x21, 0x01, // local.set 1      (sum = 0)
            0x41, 0x01, // i32.const 1
            0x21, 0x02, // local.set 2      (i = 1)
            0x02, 0x40, // block
            0x03, 0x40, //   loop
            0x20, 0x02, //     local.get 2
            0x20, 0x00, //     local.get 0
            0x48, //     i32.lt_s
            0x45, //     i32.eqz
            0x0D, 0x01, //     br_if 1        (break if i >= n)
            0x20, 0x01, //     local.get 1
            0x20, 0x02, //     local.get 2
            0x6A, //     i32.add
            0x21, 0x01, //     local.set 1    (sum += i)
            0x20, 0x02, //     local.get 2
            0x41, 0x01, //     i32.const 1
            0x6A, //     i32.add
            0x21, 0x02, //     local.set 2    (i += 1)
            0x0C, 0x00, //     br 0           (continue loop)
            0x0B, //   end (loop)
            0x0B, // end (block)
            0x20, 0x01, // local.get 1
            0x0F, // return
        ],
    );
}
