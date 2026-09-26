// `@fold` / `@next` (Part B of `docs/1.0/PROGRESS.md`), through the real
// `bitterasm compile` CLI.

mod common;

use common::{compile_error, compile_ints};

fn ints(name: &str) -> Vec<i128> {
    compile_ints(&format!("tests/fixtures/fold/{name}.basm"))
}

fn error(name: &str) -> String {
    compile_error(&format!("tests/fixtures/fold/err_{name}.basm"))
}

// ---------------------------------------------------------------------
// macro bodies (Phase B2)
// ---------------------------------------------------------------------

#[test]
fn sums_with_one_accumulator_and_no_depth_limit() {
    // 100,000 iterations: far past both the 32-level call depth and the
    // 4,096-iteration tail-call cap.
    assert_eq!(ints("sum"), vec![10, 4_999_950_000]);
}

#[test]
fn emits_offsets_then_binds_the_total() {
    assert_eq!(ints("table"), vec![0, 0, 1, 3, 6]);
}

#[test]
fn several_accumulators_update_by_name_and_otherwise_stay_put() {
    assert_eq!(ints("multiple"), vec![4, 9]);
}

#[test]
fn a_struct_accumulator_fits_in_the_result_struct() {
    assert_eq!(ints("struct_accumulator"), vec![3, 30, 3]);
}

#[test]
fn next_ends_the_iteration_and_a_bare_next_changes_nothing() {
    assert_eq!(ints("next_ends_iteration"), vec![0, 1, 3, 3]);
}

#[test]
fn return_inside_a_fold_returns_from_the_macro() {
    assert_eq!(ints("return_inside"), vec![4, -1]);
}

#[test]
fn statement_folds_keep_emits_and_nest() {
    assert_eq!(ints("statement_and_nested"), vec![0, 0, 1]);
}

#[test]
fn labels_after_an_emitting_fold_count_its_values() {
    assert_eq!(ints("labels_after"), vec![0, 1, 2, 3]);
}

#[test]
fn a_macro_call_as_a_const_value_keeps_its_emits_and_label_positions() {
    assert_eq!(ints("kept_call_emits"), vec![111, 222, 5, 3]);
}

#[test]
fn rejects_next_outside_a_fold() {
    assert!(error("next_outside").contains("`@next` can only be used inside a `@fold` body"));
}

#[test]
fn rejects_next_inside_a_plain_for_in_a_fold() {
    assert!(error("next_in_plain_for").contains("inside a plain `@for` nested in a `@fold`"));
}

#[test]
fn rejects_next_in_a_macro_called_from_a_fold() {
    assert!(error("next_in_called_macro").contains("`@next` can only be used inside a `@fold` body"));
}

#[test]
fn rejects_positional_next_with_several_accumulators() {
    assert!(error("positional_with_two").contains("positional `@next` needs exactly one accumulator"));
}

#[test]
fn rejects_an_unknown_accumulator() {
    assert!(error("unknown_accumulator").contains("`b` isn't an accumulator of this `@fold`"));
}

#[test]
fn rejects_a_duplicate_accumulator() {
    assert!(error("duplicate_accumulator").contains("accumulator `a` is declared twice"));
}

#[test]
fn rejects_an_accumulator_named_like_the_loop_variable() {
    assert!(error("var_is_accumulator").contains("both the loop variable and an accumulator"));
}

#[test]
fn rejects_updating_an_accumulator_twice_in_one_next() {
    assert!(error("duplicate_update").contains("`a` is updated twice in one `@next`"));
}

#[test]
fn rejects_an_emitting_fold_inside_a_larger_expression() {
    assert!(error("emitting_fold_in_expression").contains("this `@fold` emits values"));
}

#[test]
fn rejects_an_emitting_macro_call_inside_a_larger_expression() {
    assert!(error("emitting_call_in_expression").contains("`side` emits values"));
}

#[test]
fn rejects_return_inside_a_fold_in_a_larger_expression() {
    assert!(error("return_in_expression_fold").contains("nothing to return from"));
}

// ---------------------------------------------------------------------
// construction literals and struct bodies (Phase B3)
// ---------------------------------------------------------------------

#[test]
fn builds_an_offsets_table_in_a_construction() {
    assert_eq!(ints("construct_table"), vec![0, 3, 8, 10]);
}

#[test]
fn next_in_a_construction_skips_the_rest_of_the_iteration() {
    assert_eq!(ints("construct_next_skips"), vec![100, 101, 202]);
}

#[test]
fn computes_struct_field_names_with_a_fold() {
    assert_eq!(ints("struct_body"), vec![10, 20, 30, 3]);
}

#[test]
fn a_construction_next_cannot_reach_a_macro_body_fold() {
    assert!(error("construct_next_escapes").contains("`@next` can only be used inside a `@fold` body"));
}

#[test]
fn rejects_next_in_a_struct_body_without_a_fold() {
    assert!(error("struct_next_outside").contains("`@next` can only be used inside a `@fold` body"));
}

// ---------------------------------------------------------------------
// top level (Phase B4)
// ---------------------------------------------------------------------

#[test]
fn unrolls_folds_at_top_level() {
    assert_eq!(
        ints("top_level"),
        vec![
            0, 1, 3, 6, // one `show` per iteration
            6,          // `start3`, a constant the fold generated
            0, 1,       // `@for j in 0..total - 8`, with `total` = 10
            4, 6,       // `r.n`, `r.s`
            0, -5, -10, // a negative accumulator
        ]
    );
}

#[test]
fn a_top_level_fold_needs_a_literal_range() {
    assert!(error("top_level_not_range").contains("range"));
}

#[test]
fn rejects_next_at_top_level_outside_a_fold() {
    assert!(error("top_level_next_outside").contains("`@next` can only be used inside a `@fold` body"));
}

#[test]
fn top_level_accumulators_must_be_integers() {
    assert!(error("top_level_struct_accumulator").contains("accumulators must be integer constants"));
}
