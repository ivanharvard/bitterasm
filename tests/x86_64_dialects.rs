// Cross-checks x86-64 dialects against `std/x86_64/impl.basm`'s own default
// positional syntax: each fixture invokes the same instructions, in the same
// order, with the same operands. Real `bitterasm compile`, through the actual CLI
// binary, on each — if a dialect ever produces different bytes than what its own
// sugar is supposed to be shorthand for, this is a byte-for-byte diff, not just a
// "did it parse" check.

use std::path::Path;
use std::process::Command;

#[test]
fn x86_64_dialects_and_default_syntax_compile_to_identical_output() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let intel = Path::new(manifest_dir).join("tests/fixtures/x86_64/dialect_intel.basm");
    let att = Path::new(manifest_dir).join("tests/fixtures/x86_64/dialect_att.basm");
    let default = Path::new(manifest_dir).join("tests/fixtures/x86_64/dialect_default.basm");

    let intel_out = std::env::temp_dir().join("bitterasm-x86_64-dialect-intel.em");
    let att_out = std::env::temp_dir().join("bitterasm-x86_64-dialect-att.em");
    let default_out = std::env::temp_dir().join("bitterasm-x86_64-dialect-default.em");

    for (source, out) in [
        (&intel, &intel_out),
        (&att, &att_out),
        (&default, &default_out),
    ] {
        let status = Command::new(bitterasm)
            .current_dir(manifest_dir)
            .args([
                "compile",
                &source.display().to_string(),
                "-o",
                &out.display().to_string(),
            ])
            .status()
            .expect("bitterasm compile should run");

        assert!(
            status.success(),
            "bitterasm compile failed for {}",
            source.display()
        );
    }

    // Entries, not whole files: the header names each file's own module.
    let intel_bytes = bitterasm::emit::EmFile::parse(&std::fs::read_to_string(&intel_out).expect(".em should exist"), bitterasm::emit::features::ALL)
        .expect("a valid .em file")
        .entries;
    let att_bytes = bitterasm::emit::EmFile::parse(&std::fs::read_to_string(&att_out).expect(".em should exist"), bitterasm::emit::features::ALL)
        .expect("a valid .em file")
        .entries;
    let default_bytes = bitterasm::emit::EmFile::parse(&std::fs::read_to_string(&default_out).expect(".em should exist"), bitterasm::emit::features::ALL)
        .expect("a valid .em file")
        .entries;

    assert_eq!(
        intel_bytes, default_bytes,
        "intel dialect and default syntax produced different emitted output for the same instructions"
    );
    assert_eq!(
        att_bytes, default_bytes,
        "att dialect and default syntax produced different emitted output for the same instructions"
    );

    std::fs::remove_file(&intel_out).ok();
    std::fs::remove_file(&att_out).ok();
    std::fs::remove_file(&default_out).ok();
}
