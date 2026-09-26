// A module's private declarations stay usable by its own `pub` macros
// wherever those are called from.

mod common;

use common::compile_ints;

#[test]
fn a_pub_macro_can_invoke_a_private_macro_with_a_private_constant() {
    // `show 4` invokes the library's private `emit_scaled` twice, once with
    // its private `SCALE`: 4 * 10, then 10 * 10.
    assert_eq!(compile_ints("tests/fixtures/private_invocation/main.basm"), vec![40, 100]);
}
