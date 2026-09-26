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
    let EmittedValue::Deferred { module, symbol } = &entries[0].value else {
        panic!("expected a Deferred entry, got {:?}", entries[0].value);
    };
    assert_eq!(symbol, "target");
    // The declaring file's module path — not an absolute path, so `.em`
    // doesn't depend on where the checkout lives (Phase D2 of
    // `docs/1.0/PROGRESS.md`).
    assert_eq!(module, "tests.fixtures.emit.extern_label_dep");
    assert!(!json.contains(env!("CARGO_MANIFEST_DIR")), "{json}");
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

#[test]
fn compile_labels_flag_writes_every_pub_labels_position_and_omits_private_ones() {
    // Phase 6 (`docs/sections-and-linking/PROGRESS.md`): `bitter build`'s
    // multi-file linking needs a way to ask, standalone, "where did this
    // file's own `pub` labels resolve to" — `compile --labels` is that
    // opt-in escape hatch. `.em`'s own shape is untouched either way (see
    // `tests/sections.rs`'s byte-identity bar, still passing unchanged).
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let source = Path::new(manifest_dir).join("tests/fixtures/emit/extern_label_dep.basm");
    let em_out = std::env::temp_dir().join("bitterasm-labels-flag.em");
    let labels_out = std::env::temp_dir().join("bitterasm-labels-flag.labels.json");

    let output = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args([
            "compile",
            &source.display().to_string(),
            "-o", &em_out.display().to_string(),
            "--labels", &labels_out.display().to_string(),
        ])
        .output()
        .expect("bitterasm compile should run");
    assert!(
        output.status.success(),
        "bitterasm compile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = std::fs::read_to_string(&labels_out).expect(".labels.json file should exist");
    let labels: std::collections::HashMap<String, String> =
        serde_json::from_str(&json).expect(".labels.json should be valid JSON");

    // `extern_label_dep.basm` declares `pub target:` then `private_marker:`
    // with nothing emitted at all — both at position 0 (trailing: nothing
    // precedes either), but only the `pub` one should appear.
    assert_eq!(labels.len(), 1);
    assert_eq!(labels.get("target"), Some(&"0".to_string()));
    assert!(!labels.contains_key("private_marker"));
}
