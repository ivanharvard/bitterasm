// Phase 5 (see `docs/sections-and-linking/PROGRESS.md`): `from file import
// label_name` resolves a `pub` label in `file` as a deferred cross-unit
// reference instead of splicing it — `label_name`'s real position isn't
// known within this compilation at all, only that it exists and is `pub`.
// Exercised through the real `bitterasm compile` CLI, same standard
// `tests/sections.rs`/`tests/label_passes.rs` hold themselves to.

use std::path::Path;
use std::process::Command;

use bitterasm::emit::{EmittedEntry, EmittedValue};

fn bitterasm_compile(fixture: &str, out: &Path) -> std::process::Output {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let source = Path::new(manifest_dir).join(fixture);

    Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &out.display().to_string()])
        .output()
        .expect("bitterasm compile should run")
}

#[test]
fn importing_a_pub_label_compiles_to_an_em_with_a_visibly_unresolved_deferred_entry() {
    let out = std::env::temp_dir().join("bitterasm-extern-label-importer.em");
    let output = bitterasm_compile("tests/fixtures/emit/extern_label_importer.basm", &out);
    assert!(
        output.status.success(),
        "bitterasm compile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = std::fs::read_to_string(&out).expect(".em file should exist");
    let entries: Vec<EmittedEntry> =
        serde_json::from_str(&json).expect(".em file should be valid EmittedEntry JSON");

    assert_eq!(entries.len(), 1);
    let EmittedValue::Deferred { file, symbol } = &entries[0].value else {
        panic!("expected a Deferred entry, got {:?}", entries[0].value);
    };
    assert_eq!(symbol, "target");
    assert!(
        file.ends_with("extern_label_dep.basm"),
        "expected the declaring file's own path, got {file}"
    );
}

#[test]
fn importing_a_non_pub_label_fails_immediately_with_a_clear_error() {
    let out = std::env::temp_dir().join("bitterasm-extern-label-private.em");
    let output = bitterasm_compile("tests/fixtures/emit/extern_label_private_import.basm", &out);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("private_marker"),
        "expected the error to name the rejected import: {stderr}"
    );
}

#[test]
fn importing_a_name_that_doesnt_exist_at_all_fails_the_same_way_a_typod_macro_import_would() {
    let out = std::env::temp_dir().join("bitterasm-extern-label-typo.em");
    let output = bitterasm_compile("tests/fixtures/emit/extern_label_typo_import.basm", &out);
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("totally_made_up_name"),
        "expected the error to name the rejected import: {stderr}"
    );
}
