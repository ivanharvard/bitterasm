// Cross-checks `std/x86_64/intel.basm` (Phase 6's Intel-syntax dialect) against
// `std/x86_64/impl.basm`'s own default positional syntax: two fixtures invoking
// the same instructions, in the same order, with the same operands — one spelled
// with real Intel mnemonic/bracket syntax, the other with impl.basm's own
// `name arg, arg, ...` calls (explicit `w=1`, `Mem(...)`/`MemIndexed(...)`/
// `MemRipRelative(...)` instead of `[...]`). Real `bitterasm compile`, through
// the actual CLI binary, on both — if the dialect ever produces different bytes
// than what its own sugar is supposed to be shorthand for, this is a byte-for-byte
// diff, not just a "did it parse" check. Mirrors `tests/riscv_dialects.rs`'s own
// standard, adapted from a two-dialect comparison (RISC-V has two, x86-64 only
// has one) to a dialect-vs-default one, matching `std/x86_64/PROGRESS.md`'s own
// stated Phase 6 test convention.

use std::path::Path;
use std::process::Command;

#[test]
fn intel_dialect_and_default_syntax_compile_to_identical_output() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let intel = Path::new(manifest_dir).join("tests/fixtures/x86_64/dialect_intel.basm");
    let default = Path::new(manifest_dir).join("tests/fixtures/x86_64/dialect_default.basm");

    let intel_out = std::env::temp_dir().join("bitterasm-x86_64-dialect-intel.em");
    let default_out = std::env::temp_dir().join("bitterasm-x86_64-dialect-default.em");

    for (source, out) in [(&intel, &intel_out), (&default, &default_out)] {
        let status = Command::new(bitterasm)
            .current_dir(manifest_dir)
            .args(["compile", &source.display().to_string(), "-o", &out.display().to_string()])
            .status()
            .expect("bitterasm compile should run");

        assert!(status.success(), "bitterasm compile failed for {}", source.display());
    }

    let intel_bytes = std::fs::read(&intel_out).expect("intel .em should exist");
    let default_bytes = std::fs::read(&default_out).expect("default .em should exist");

    assert_eq!(
        intel_bytes, default_bytes,
        "intel dialect and default syntax produced different emitted output for the same instructions"
    );

    std::fs::remove_file(&intel_out).ok();
    std::fs::remove_file(&default_out).ok();
}
