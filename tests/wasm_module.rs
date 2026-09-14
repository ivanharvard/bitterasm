// End-to-end test for `std/wasm/module.basm`'s `span`/`deferred_uleb128`
// machinery: `tests/wasm/cases/module.basm` builds a complete, real `.wasm`
// module (Type/Function/Export/Code sections, one exported zero-argument
// function computing sum(1..9)) using labels + `std.bitter.deferred`'s
// `span(start, end)` for every section's length prefix and the Code
// section's per-function body length prefix — nothing hand-computed.
//
// The fixed structural bytes (magic/version, the type/function/export
// section contents, the instruction stream) are hand-verified byte-for-
// byte against the WASM spec, the same standard `tests/wasm_encoding.rs`
// holds itself to. The four length-prefix bytes those hand-computed values
// depend on are exactly what this test exists to check — this repo has no
// independent WASM assembler to cross-check against (the way
// `tests/riscv_dialects.rs` cross-checks against real GNU binutils), but
// unlike `std/pdp10` (which has no independent implementation to check
// against at all), a real, independent WebAssembly engine is one `node -e`
// away: it both validates the module's binary structure and actually runs
// the exported function, so a wrong length prefix or a wrong instruction
// encoding fails this test either as a validation error or as a wrong
// return value, not just as a byte mismatch against another hand
// computation that could share the same mistake.
//
// See `tests/wasm_encoding.rs`/`tests/pdp10_encoding.rs` for why `bitter`'s
// binary path is derived from `bitterasm`'s rather than read from
// `CARGO_BIN_EXE_bitter`, and why it's built on demand.

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

/// Compiles and encodes `tests/wasm/cases/module.basm`, returning the raw
/// `.wasm` bytes. `unique_name` only needs to differ between the tests in
/// this file, which (like every `cargo test` binary) run concurrently and
/// would otherwise race over the same `.em`/`.wasm` scratch files.
fn compile_module(unique_name: &str) -> Vec<u8> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let bitter = bitter_bin();

    let source = Path::new(manifest_dir).join("tests/wasm/cases/module.basm");
    let work_dir = std::env::temp_dir().join("bitterasm-wasm-module");
    std::fs::create_dir_all(&work_dir).unwrap();
    let em_path = work_dir.join(format!("{unique_name}.em"));
    let wasm_path = work_dir.join(format!("{unique_name}.wasm"));

    let compile_status = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &em_path.display().to_string()])
        .status()
        .expect("bitterasm compile should run");
    assert!(compile_status.success(), "bitterasm compile failed for module.basm");

    let encode_output = Command::new(&bitter)
        .args(["encode", &em_path.display().to_string(), "-o", &wasm_path.display().to_string()])
        .output()
        .expect("bitter encode should run");
    assert!(
        encode_output.status.success(),
        "bitter encode failed for module.em:\n{}",
        String::from_utf8_lossy(&encode_output.stderr)
    );

    let bytes = std::fs::read(&wasm_path).expect("encoded .wasm should exist");
    std::fs::remove_file(&em_path).ok();
    std::fs::remove_file(&wasm_path).ok();
    bytes
}

#[test]
fn module_encodes_to_the_hand_verified_byte_sequence() {
    let bytes = compile_module("module");

    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        // magic + version
        0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
        // Type section: id=1, len=5, [count=1, functype () -> i32]
        0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
        // Function section: id=3, len=2, [count=1, type index 0]
        0x03, 0x02, 0x01, 0x00,
        // Export section: id=7, len=14, [count=1, "sum_to_ten", kind=func, index=0]
        0x07, 0x0E,
        0x01, 0x0A, 0x73, 0x75, 0x6D, 0x5F, 0x74, 0x6F, 0x5F, 0x74, 0x65, 0x6E, 0x00, 0x00,
        // Code section: id=10, len=51, [count=1, body_size=49, locals, instructions]
        0x0A, 0x33,
        0x01, 0x31,
        0x01, 0x03, 0x7F, // 1 local-decl group: 3 locals of type i32
        0x41, 0x0A, // i32.const 10
        0x21, 0x00, // local.set 0     (n = 10)
        0x41, 0x00, // i32.const 0
        0x21, 0x01, // local.set 1     (sum = 0)
        0x41, 0x01, // i32.const 1
        0x21, 0x02, // local.set 2     (i = 1)
        0x02, 0x40, // block
        0x03, 0x40, //   loop
        0x20, 0x02, //     local.get 2
        0x20, 0x00, //     local.get 0
        0x48, //     i32.lt_s
        0x45, //     i32.eqz
        0x0D, 0x01, //     br_if 1
        0x20, 0x01, //     local.get 1
        0x20, 0x02, //     local.get 2
        0x6A, //     i32.add
        0x21, 0x01, //     local.set 1
        0x20, 0x02, //     local.get 2
        0x41, 0x01, //     i32.const 1
        0x6A, //     i32.add
        0x21, 0x02, //     local.set 2
        0x0C, 0x00, //     br 0
        0x0B, //   end (loop)
        0x0B, // end (block)
        0x20, 0x01, // local.get 1
        0x0F, // return
        0x0B, // end (function)
    ];

    assert_eq!(bytes, expected, "expected {expected:02X?}, got {bytes:02X?}");
}

/// Independent cross-check: an unrelated, real WebAssembly implementation
/// (Node's) both accepts the module as structurally valid *and* runs the
/// exported function to the value the instruction sequence should compute
/// (sum of 1..9 == 45) — catching anything a byte-for-byte comparison
/// against another hand computation could share a mistake with (a wrong
/// length prefix, a swapped opcode that still happens to be a byte,
/// mismatched branch depths).
#[test]
fn module_validates_and_runs_under_an_independent_wasm_engine() {
    let node = match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => "node",
        _ => {
            eprintln!("skipping: `node` isn't available on PATH");
            return;
        }
    };

    let bytes = compile_module("module_for_node");
    let work_dir = std::env::temp_dir().join("bitterasm-wasm-module");
    std::fs::create_dir_all(&work_dir).unwrap();
    let wasm_path = work_dir.join("module_for_node.wasm");
    std::fs::write(&wasm_path, &bytes).unwrap();

    let script = format!(
        r#"
        const fs = require("fs");
        const bytes = fs.readFileSync("{}");
        if (!WebAssembly.validate(bytes)) {{
            console.error("INVALID");
            process.exit(1);
        }}
        const instance = new WebAssembly.Instance(new WebAssembly.Module(bytes), {{}});
        console.log(instance.exports.sum_to_ten());
        "#,
        wasm_path.display()
    );

    let output = Command::new(node).args(["-e", &script]).output().expect("node should run");
    std::fs::remove_file(&wasm_path).ok();

    assert!(
        output.status.success(),
        "node rejected the module:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "45", "sum_to_ten() should return 45 (sum of 1..9)");
}
