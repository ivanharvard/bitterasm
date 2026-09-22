// Phase 2 (see `docs/sections-and-linking/PROGRESS.md`): the resolver
// tracks "current section" and tags every `.em` entry with which section
// was active when it was `@emit`'d. Exercised through the real
// `bitterasm compile` CLI, same standard `tests/label_passes.rs` holds
// itself to, so this also covers `main.rs`'s `walk_top_level`/`Expansion`
// wiring, not just the resolver internals.

use std::path::Path;
use std::process::Command;

use bitterasm::emit::EmittedEntry;

fn compile_to_raw_json(fixture: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let source = Path::new(manifest_dir).join(fixture);
    let stem = Path::new(fixture).file_stem().unwrap().to_string_lossy();
    let out = std::env::temp_dir().join(format!("bitterasm-sections-{stem}.em"));

    let status = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &out.display().to_string()])
        .status()
        .expect("bitterasm compile should run");
    assert!(status.success(), "bitterasm compile failed for {fixture}");

    std::fs::read_to_string(&out).expect(".em file should exist")
}

fn compile_to_entries(fixture: &str) -> Vec<EmittedEntry> {
    let json = compile_to_raw_json(fixture);
    serde_json::from_str(&json).expect(".em file should be valid EmittedEntry JSON")
}

fn int(value: &str, section: Option<&str>) -> EmittedEntry {
    EmittedEntry {
        value: bitterasm::emit::EmittedValue::Int { value: value.to_string() },
        section: section.map(str::to_string),
    }
}

#[test]
fn no_section_statement_produces_em_with_no_section_key_at_all() {
    // The hard backward-compatibility bar from "Decided scope": a program
    // that never declares a `section` must produce `.em` byte-for-byte
    // identical to before this field existed — checked here at the raw
    // JSON text level (not just "deserializes to the same values"), since
    // `#[serde(skip_serializing_if)]` only omits the key when `None`, and
    // a typo there would still deserialize fine while silently changing
    // the actual bytes on disk.
    let json = compile_to_raw_json("tests/fixtures/emit/sections_none.basm");
    assert!(!json.contains("section"), "no `section` key should appear anywhere: {json}");

    let entries = compile_to_entries("tests/fixtures/emit/sections_none.basm");
    assert_eq!(entries, vec![int("1", None), int("1", None)]);
}

#[test]
fn one_section_tags_every_entry_with_it() {
    let entries = compile_to_entries("tests/fixtures/emit/sections_one.basm");
    assert_eq!(entries, vec![int("1", Some(".text")), int("1", Some(".text"))]);
}

#[test]
fn reopening_a_section_is_the_same_group_not_a_new_one() {
    let entries = compile_to_entries("tests/fixtures/emit/sections_reopened.basm");
    assert_eq!(
        entries,
        vec![int("1", Some(".text")), int("2", Some(".data")), int("3", Some(".text"))],
    );
}

#[test]
fn a_macros_section_change_is_scoped_to_its_own_call_not_leaked_to_the_caller() {
    let entries = compile_to_entries("tests/fixtures/emit/sections_macro_scoped.basm");
    assert_eq!(
        entries,
        vec![int("1", Some(".text")), int("99", Some(".data")), int("2", Some(".text"))],
    );
}

#[test]
fn a_macro_declared_leaks_section_leaves_its_section_change_active_after_return() {
    // Phase 3: `| leaks_section` opts a macro out of the Phase 2 push/pop
    // restore, on purpose - `mark 2` lands in `.data`, the section
    // `leaks_on_purpose` switched to, not back in `.text` where it was
    // called from.
    let entries = compile_to_entries("tests/fixtures/emit/sections_leaks_facet.basm");
    assert_eq!(
        entries,
        vec![int("1", Some(".text")), int("99", Some(".data")), int("2", Some(".data"))],
    );
}
