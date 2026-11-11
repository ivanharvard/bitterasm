// `std.array`, `std.option` and `std.list`'s macros, through the real CLI.

mod common;

#[test]
fn array_helpers() {
    let values = common::compile_ints("tests/fixtures/collections/array.basm");
    assert_eq!(
        values,
        [
            7, 7, 7, // filled
            3, 1, 2, 9, 8, // concatenated
            6, 0, // sum, of an empty array too
            1, 0, // any, all
            -1, 0, 3, 3, 5, 9, // sorted, and sorting nothing
        ]
    );
}

#[test]
fn option_helpers() {
    let values = common::compile_ints("tests/fixtures/collections/option.basm");
    // is_some twice, is_none, unwrap, unwrap_or, mapped, mapped of `None`,
    // and the array `mapped` beside the option one.
    assert_eq!(values, [1, 0, 1, 21, 5, 42, 0, 6]);
}

#[test]
fn list_helpers() {
    let values = common::compile_ints("tests/fixtures/collections/list.basm");
    // get, concatenated both ways, mapped.
    assert_eq!(values, [1, 3, 1, 2, 3, 4, 1, 2, 3, 2, 4, 6]);
}

#[test]
fn unwrapping_none_is_a_compile_error() {
    let error = common::compile_error("tests/fixtures/collections/err_unwrap_none.basm");
    assert!(error.contains("unwrap of `None`"), "{error}");
}

#[test]
fn a_list_index_past_the_end_is_a_compile_error() {
    let error = common::compile_error("tests/fixtures/collections/err_list_index.basm");
    assert!(error.contains("list index past the end"), "{error}");
}
