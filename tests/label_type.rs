// `label`: a compile-time type, like a macro's, for positions in the output.

mod common;

use common::{compile_error, compile_ints};

fn error(name: &str) -> String {
    compile_error(&format!("tests/fixtures/labels/err_{name}.basm"))
}

#[test]
fn labels_overload_against_ints_and_compare() {
    assert_eq!(compile_ints("tests/fixtures/labels/overloads.basm"), vec![7, 1000, 42, 1001, 1, 0, 1001]);
}

#[test]
fn a_labels_position_counts_the_output_grouped_by_section() {
    assert_eq!(compile_ints("tests/fixtures/labels/as_int_after_sections.basm"), vec![10, 2, 20]);
}

#[test]
fn a_label_cant_be_emitted() {
    let stderr = error("emit");
    assert!(stderr.contains("can't `@emit` a value of type `label<stream>`"), "{stderr}");
}

#[test]
fn a_label_isnt_a_number() {
    let stderr = error("arithmetic");
    assert!(stderr.contains("a label is a position, not a number"), "{stderr}");
}

#[test]
fn a_number_isnt_a_label() {
    let stderr = error("number_as_label");
    assert!(stderr.contains("expected `label`, found `int`"), "{stderr}");
}

#[test]
fn a_type_naming_label_cant_be_emitted_even_when_empty() {
    let stderr = error("empty_array");
    assert!(stderr.contains("can't `@emit` a value of type `Array<label, 0>`"), "{stderr}");
}

#[test]
fn a_label_is_typed_by_its_sections_space() {
    assert_eq!(compile_ints("tests/fixtures/labels/spaces.basm"), vec![1, 5, 5, 5, 5, 2]);
}

#[test]
fn label_s_takes_labels_from_one_space() {
    let stderr = error("mixed_spaces");
    assert!(stderr.contains("type mismatch for `S`: expected `label<flash>`, found `label<ram>`"), "{stderr}");
    let stderr = error("wrong_space");
    assert!(stderr.contains("expected `label<flash>`, found `label<ram>`"), "{stderr}");
}
