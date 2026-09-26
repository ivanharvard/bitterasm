// Phase 5 (see `docs/sections-and-linking/PROGRESS.md`): `from file import
// label_name` resolves a `pub` label in `file` as a deferred cross-unit
// reference instead of splicing it — `label_name`'s real position isn't
// known within this compilation at all, only that it exists and is `pub`.
// Exercised through the real `bitterasm compile` CLI, same standard
// `tests/sections.rs`/`tests/label_passes.rs` hold themselves to.

use std::path::Path;
use std::process::Command;

use bitterasm::emit::EmittedValue;

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
    let file = bitterasm::emit::EmFile::parse(&json, bitterasm::emit::features::ALL).expect("a valid .em file");
    assert_eq!(file.requires, ["extern-labels"]);
    let entries = file.entries;

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
fn em_exports_every_pub_labels_position_and_omits_private_ones() {
    // `bitter build`'s multi-file linking needs to know, from each input's
    // own `.em`, where its `pub` labels resolved to: the header's
    // `exports` (Phase D3 of `docs/1.0/PROGRESS.md`).
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let source = Path::new(manifest_dir).join("tests/fixtures/emit/extern_label_dep.basm");
    let em_out = std::env::temp_dir().join(format!("bitterasm-exports-{}.em", std::process::id()));

    let output = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &em_out.display().to_string()])
        .output()
        .expect("bitterasm compile should run");
    assert!(
        output.status.success(),
        "bitterasm compile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = std::fs::read_to_string(&em_out).expect(".em file should exist");
    let file = bitterasm::emit::EmFile::parse(&json, bitterasm::emit::features::ALL).expect("a valid .em file");

    // `extern_label_dep.basm` declares `pub target:` then `private_marker:`
    // with nothing emitted at all — both at position 0 (trailing: nothing
    // precedes either), but only the `pub` one should appear.
    assert_eq!(file.module, "tests.fixtures.emit.extern_label_dep");
    assert_eq!(file.exports.len(), 1);
    assert_eq!(file.exports.get("target"), Some(&0));
    assert!(!file.exports.contains_key("private_marker"));
}

#[test]
fn a_librarys_macro_can_use_a_label_the_library_imported() {
    // The library's own `from .labels import marker` travels with its
    // declarations, and the entry file importing the same label too is
    // still one extern label.
    let out = std::env::temp_dir().join(format!("bitterasm-extern-transitive-{}.em", std::process::id()));
    let output = bitterasm_compile("tests/fixtures/extern_transitive/main.basm", &out);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let json = std::fs::read_to_string(&out).expect(".em file should exist");
    let file = bitterasm::emit::EmFile::parse(&json, bitterasm::emit::features::ALL).expect("a valid .em file");
    let deferred = EmittedValue::Deferred {
        module: "tests.fixtures.extern_transitive.labels".to_string(),
        symbol: "marker".to_string(),
    };
    assert_eq!(file.entries.iter().map(|entry| entry.value.clone()).collect::<Vec<_>>(), [deferred.clone(), deferred]);
}
