// Spaces and sections as values: `space` declarations, a section's `in`,
// and user-defined facets built from `@extend`.

mod common;

use common::{compile_error, compile_ints};

fn error(name: &str) -> String {
    compile_error(&format!("tests/fixtures/spaces/err_{name}.basm"))
}

#[test]
fn facets_build_spaces_and_sections_field_by_field() {
    assert_eq!(compile_ints("tests/fixtures/spaces/facets.basm"), vec![32768, 1024, 32768, 16, 1, 0, 0, 7, 1, 0]);
}

#[test]
fn a_space_can_come_from_another_module() {
    assert_eq!(compile_ints("tests/fixtures/spaces/imported.basm"), vec![0x100]);
}

#[test]
fn a_facet_must_return_what_it_was_given() {
    let stderr = error("wrong_subject");
    assert!(stderr.contains("facet `swap` must return the space `flash` it was given"), "{stderr}");
    let stderr = error("no_return");
    assert!(stderr.contains("facet `nothing` must return the space `flash` it was given"), "{stderr}");
}

#[test]
fn a_field_is_added_once() {
    let stderr = error("duplicate_field");
    assert!(stderr.contains("space `flash` already has a field `size`"), "{stderr}");
}

#[test]
fn a_facet_can_raise_its_own_error() {
    let stderr = error("error");
    assert!(stderr.contains("this space already has a start address"), "{stderr}");
}

#[test]
fn a_space_cant_be_emitted() {
    let stderr = error("emit");
    assert!(stderr.contains("can't `@emit` a value of type `space`"), "{stderr}");
}

#[test]
fn a_type_field_isnt_a_value() {
    let stderr = error("type_field");
    assert!(stderr.contains("space `flash`'s field `unit` holds a type, not a value"), "{stderr}");
}

#[test]
fn a_section_is_declared_once() {
    let stderr = error("redeclared");
    assert!(stderr.contains("section `.text` is already declared"), "{stderr}");
}

#[test]
fn a_space_declaration_needs_a_space() {
    let stderr = error("not_a_space");
    assert!(stderr.contains("expected a space, found `int`"), "{stderr}");
}

#[test]
fn only_spaces_and_sections_can_be_extended() {
    let stderr = error("extend_struct");
    assert!(stderr.contains("`@extend` takes a space or section, not `bits<8>`"), "{stderr}");
}

// ---- sections ----

#[test]
fn a_macro_can_declare_a_section_from_its_arguments() {
    // Called twice with the same space: the same declaration, no conflict.
    assert_eq!(compile_ints("tests/fixtures/spaces/macro_sections.basm"), vec![1, 1]);
    let stderr = error("macro_section_conflict");
    assert!(stderr.contains("section `.text` is already declared"), "{stderr}");
}

#[test]
fn sections_are_imported_with_their_space_and_facets() {
    assert_eq!(compile_ints("tests/fixtures/spaces/imported_sections.basm"), vec![1, 2, 0]);
    let stderr = error("imported_section_facet");
    assert!(stderr.contains("this section's facet ran"), "{stderr}");
}

#[test]
fn a_files_own_section_replaces_an_imported_one() {
    assert_eq!(compile_ints("tests/fixtures/spaces/local_replaces_import.basm"), vec![1]);
}

#[test]
fn a_section_is_declared_before_it_is_used() {
    let stderr = error("used_before_declared");
    assert!(stderr.contains("section `.x` is declared after it's already been used"), "{stderr}");
}

// ---- struct facets ----

#[test]
fn a_structs_facets_check_every_value_it_builds() {
    assert_eq!(compile_ints("tests/fixtures/spaces/struct_facets.basm"), vec![4, 6]);
    let stderr = error("struct_facet_check");
    assert!(stderr.contains("register out of range"), "{stderr}");
}

#[test]
fn a_struct_facet_returns_the_value_unchanged() {
    let stderr = error("struct_facet_changes");
    assert!(stderr.contains("facet `bump` must return the `Reg` value it was given"), "{stderr}");
}

#[test]
fn only_structs_spaces_and_sections_take_user_facets() {
    let stderr = error("user_facet_on_macro");
    assert!(stderr.contains("`nope` isn't a built-in facet"), "{stderr}");
}
