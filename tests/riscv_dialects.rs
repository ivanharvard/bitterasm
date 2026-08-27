// Cross-checks `std/riscv/c_like.basm` against `std/riscv/native.basm`: two
// fixtures that invoke the same `std/riscv/impl.basm` instructions, in the
// same order, with the same operands — one spelled the conventional
// mnemonic way, the other with no instruction name anywhere at all (see
// `std/riscv/c_like.basm`'s own doc). Real `bitterasm compile`, through the
// actual CLI binary (`CARGO_BIN_EXE_bitterasm`, not a hand-rolled call into
// the library), on both — if the two dialects ever disagree about which
// instruction a pattern means, this is a byte-for-byte diff, not just a
// "did it parse" check.

use std::path::Path;
use std::process::Command;

#[test]
fn c_like_and_native_dialects_compile_to_identical_output() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let native = Path::new(manifest_dir).join("tests/fixtures/riscv/dialect_native.basm");
    let c_like = Path::new(manifest_dir).join("tests/fixtures/riscv/dialect_c_like.basm");

    let native_out = std::env::temp_dir().join("bitterasm-dialect-native.em");
    let c_like_out = std::env::temp_dir().join("bitterasm-dialect-c_like.em");

    for (source, out) in [(&native, &native_out), (&c_like, &c_like_out)] {
        let status = Command::new(bitterasm)
            .current_dir(manifest_dir)
            .args(["compile", &source.display().to_string(), "-o", &out.display().to_string()])
            .status()
            .expect("bitterasm compile should run");

        assert!(status.success(), "bitterasm compile failed for {}", source.display());
    }

    let native_bytes = std::fs::read(&native_out).expect("native .em should exist");
    let c_like_bytes = std::fs::read(&c_like_out).expect("c_like .em should exist");

    assert_eq!(
        native_bytes, c_like_bytes,
        "native and c_like dialects produced different emitted output for the same instructions"
    );

    std::fs::remove_file(&native_out).ok();
    std::fs::remove_file(&c_like_out).ok();
}

#[test]
fn importing_both_riscv_dialects_together_is_a_conflict_error() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let dir = std::env::temp_dir().join("bitterasm-dialect-conflict-test");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("both.basm");
    std::fs::write(
        &source,
        "from std.riscv.native import *\nfrom std.riscv.c_like import *\n\nadd x1, x2, x3\n",
    )
    .unwrap();

    let output = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["check", &source.display().to_string()])
        .output()
        .expect("bitterasm check should run");

    assert!(!output.status.success(), "importing both dialects together should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("multiple imports assign different syntax"), "unexpected stderr: {stderr}");

    std::fs::remove_dir_all(&dir).ok();
}
