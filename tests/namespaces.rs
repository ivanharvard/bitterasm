// Each file sees its own declarations and exactly what it imports: a
// module's own imports aren't passed on unless it re-exports them with
// `pub from`.

mod common;

use bitterasm::emit::EmittedValue;
use common::{compile_entries, compile_error, compile_ints};

#[test]
fn an_imported_module_does_not_pass_on_its_own_imports() {
    let stderr = compile_error("tests/fixtures/namespaces/transitive_hidden.basm");
    assert!(stderr.contains("unknown constant `BASE`"), "{stderr}");
}

#[test]
fn an_imported_macro_still_sees_its_own_modules_imports() {
    assert_eq!(compile_ints("tests/fixtures/namespaces/hygiene.basm"), vec![1]);
}

#[test]
fn a_files_own_declaration_shadows_an_imported_one() {
    // `go` reads this file's `BASE`; `emit_base` still reads its own.
    assert_eq!(compile_ints("tests/fixtures/namespaces/own_shadows_import.basm"), vec![7, 1]);
}

#[test]
fn pub_from_re_exports_a_name() {
    assert_eq!(compile_ints("tests/fixtures/namespaces/reexport.basm"), vec![1]);
}

#[test]
fn a_private_import_is_not_importable_from_the_importer() {
    let stderr = compile_error("tests/fixtures/namespaces/private_not_reexported.basm");
    assert!(stderr.contains("has no `emit_base`"), "{stderr}");
}

#[test]
fn using_a_name_two_imports_both_bring_in_is_an_error() {
    let stderr = compile_error("tests/fixtures/namespaces/ambiguous.basm");
    assert!(stderr.contains("`BASE` is imported from more than one module"), "{stderr}");
    assert!(stderr.contains("lib.base") && stderr.contains("lib.other"), "{stderr}");
}

#[test]
fn a_name_two_imports_bring_in_is_fine_until_used() {
    assert_eq!(compile_ints("tests/fixtures/namespaces/ambiguous_unused.basm"), vec![1]);
}

#[test]
fn a_name_imported_by_name_beats_one_a_star_brings_in() {
    assert_eq!(compile_ints("tests/fixtures/namespaces/explicit_beats_glob.basm"), vec![2]);
}

#[test]
fn a_spliced_name_reads_an_imported_declaration() {
    assert_eq!(compile_ints("tests/fixtures/namespaces/spliced_read.basm"), vec![20]);
}

#[test]
fn macro_overloads_from_different_modules_merge() {
    assert_eq!(compile_ints("tests/fixtures/namespaces/overloads_merge.basm"), vec![1, 5]);
}

#[test]
fn a_file_can_reuse_a_name_std_uses_internally() {
    let entries = compile_entries("tests/fixtures/namespaces/std_name_reuse.basm");
    let [entry] = entries.as_slice() else { panic!("expected one entry, got {entries:?}") };
    let EmittedValue::Struct { id, .. } = &entry.value else { panic!("expected a struct, got {entry:?}") };
    assert_eq!(id, "tests.fixtures.namespaces.std_name_reuse.bits");
}

#[test]
fn a_file_still_cannot_declare_one_name_twice() {
    let stderr = compile_error("tests/fixtures/emit/label_collides_with_const.basm");
    assert!(stderr.contains("duplicate symbol `dup`"), "{stderr}");
}
