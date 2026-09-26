// Phase A1 of `docs/1.0/PROGRESS.md`: `` name`expr` `` in expression
// position reads whatever the spliced name resolves to.

mod common;

use common::{compile_error, compile_ints};

#[test]
fn reads_a_binding_from_the_same_iteration() {
    assert_eq!(compile_ints("tests/fixtures/spliced/scope_binding.basm"), vec![1, 11, 21]);
}

#[test]
fn reads_constants_another_macro_generated() {
    assert_eq!(compile_ints("tests/fixtures/spliced/generated_const.basm"), vec![100, 101, 102]);
}

#[test]
fn reads_top_level_constants_from_a_const_and_a_macro() {
    assert_eq!(compile_ints("tests/fixtures/spliced/top_level.basm"), vec![1, 13]);
}

#[test]
fn finds_private_constants_of_the_reading_module() {
    assert_eq!(compile_ints("tests/fixtures/spliced/private_const.basm"), vec![8, 40]);
}

#[test]
fn a_space_before_the_backtick_is_not_a_spliced_name() {
    let stderr = compile_error("tests/fixtures/spliced/whitespace.basm");
    assert!(!stderr.contains("x0"), "should not have read `x0`:\n{stderr}");
}

#[test]
fn an_unknown_spliced_name_reports_the_resolved_name() {
    let stderr = compile_error("tests/fixtures/spliced/unknown_name.basm");
    assert!(stderr.contains("nothing_5"), "{stderr}");
}
