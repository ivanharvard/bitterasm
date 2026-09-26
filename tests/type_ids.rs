// Phase D1 of `docs/1.0/PROGRESS.md`: `.em` identifies every struct, enum
// and type by its module path + declared name.

use std::path::{Path, PathBuf};
use std::process::Command;

use bitterasm::emit::{EmittedEntry, EmittedValue};

// Copies `tests/fixtures/ids` to a fresh directory under `parent` and
// compiles `main.basm` there, with that directory as the working directory.
fn compile_copy(parent: &Path) -> (String, Vec<EmittedEntry>) {
    let root = parent.join("project");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("lib")).unwrap();

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ids");
    std::fs::copy(fixtures.join("main.basm"), root.join("main.basm")).unwrap();
    std::fs::copy(fixtures.join("lib/shapes.basm"), root.join("lib/shapes.basm")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_bitterasm"))
        .current_dir(&root)
        .env_remove("BITTERASM_PATH")
        .args(["compile", "main.basm", "-o", "main.em"])
        .output()
        .expect("bitterasm compile should run");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let json = std::fs::read_to_string(root.join("main.em")).unwrap();
    let entries = bitterasm::emit::EmFile::parse(&json, bitterasm::emit::features::ALL).unwrap().entries;
    (json, entries)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bitterasm-type-ids-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn struct_ids(entries: &[EmittedEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| match &entry.value {
            EmittedValue::Struct { id, .. } => id.clone(),
            other => panic!("expected a struct, found {other:?}"),
        })
        .collect()
}

#[test]
fn ids_are_module_path_plus_name_and_hide_private_renaming() {
    let (_, entries) = compile_copy(&scratch("names"));
    let ids = struct_ids(&entries);

    assert_eq!(ids[0], "main.Local");
    assert_eq!(ids[1], "lib.shapes.Pair");
    // A synthesized struct keeps its counter, which keeps it unique.
    assert!(ids[2].starts_with("main.__range$"), "{}", ids[2]);
}

#[test]
fn output_does_not_depend_on_where_the_project_lives() {
    let (first, _) = compile_copy(&scratch("here"));
    let (second, _) = compile_copy(&scratch("somewhere-else").join("deeper"));
    assert_eq!(first, second);
}

#[test]
fn a_file_under_no_search_root_is_named_relative_to_the_working_directory() {
    let dir = scratch("outside");
    let (cwd, elsewhere) = (dir.join("cwd"), dir.join("elsewhere"));
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(
        elsewhere.join("thing.basm"),
        "struct Thing {\n    pub v: int,\n}\n\nmacro f() {\n    @emit Thing(v = 1)\n}\n\nf\n",
    )
    .unwrap();

    let out = dir.join("thing.em");
    let output = Command::new(env!("CARGO_BIN_EXE_bitterasm"))
        .current_dir(&cwd)
        .env("HOME", &dir)
        .env_remove("BITTERASM_PATH")
        .args(["compile", &elsewhere.join("thing.basm").display().to_string(), "-o", &out.display().to_string()])
        .output()
        .expect("bitterasm compile should run");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let entries = bitterasm::emit::EmFile::parse(&std::fs::read_to_string(&out).unwrap(), bitterasm::emit::features::ALL)
        .unwrap()
        .entries;
    assert_eq!(struct_ids(&entries), ["..elsewhere.thing.Thing"]);
}
