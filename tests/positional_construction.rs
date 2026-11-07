// `Name<args>(a, b, ...)`: a struct's fields by position, including the
// ones its body generates with `@for`.

mod common;

use common::{compile_error, compile_ints};

#[test]
fn builds_structs_with_generated_fields_by_position() {
    assert_eq!(compile_ints("tests/fixtures/positional/arrays.basm"), vec![10, 30, 3, 2, 10]);
}

#[test]
fn a_derived_field_takes_no_positional_argument() {
    let stderr = compile_error("tests/fixtures/positional/err_too_many.basm");
    assert!(stderr.contains("`Array` expects 2 argument(s), but 3 were supplied"), "{stderr}");
}

#[test]
fn a_generic_struct_needs_its_generic_arguments() {
    let stderr = compile_error("tests/fixtures/positional/err_no_generic_args.basm");
    assert!(stderr.contains("`Array` expects 2 generic argument(s), but 0 were supplied"), "{stderr}");
}
