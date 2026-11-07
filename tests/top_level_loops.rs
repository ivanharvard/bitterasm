// Top-level `@for`: a loop of invocations runs as a loop, and means the
// same as the unrolled copies a loop with declarations becomes.

mod common;

use common::{compile_error, compile_ints};

fn ints(name: &str) -> Vec<i128> {
    compile_ints(&format!("tests/fixtures/loops/{name}.basm"))
}

#[test]
fn a_kept_loop_emits_what_its_unrolled_copies_would() {
    let expected = vec![0, 0, 1, 2, 10, 1, 2, 20, 2, 0, 2, -1, 4];
    assert_eq!(ints("kept"), expected);
    assert_eq!(ints("unrolled"), expected);
}

#[test]
fn a_kept_loop_runs_in_its_own_file_but_not_in_an_importer() {
    assert_eq!(ints("dep"), vec![0, 1, 2]);
    assert_eq!(ints("imports_dep"), vec![42]);
}

#[test]
fn an_error_inside_a_kept_loop_is_reported_where_it_happens() {
    let stderr = compile_error("tests/fixtures/loops/err_in_loop.basm");
    assert!(stderr.contains("too big"), "{stderr}");
    assert!(stderr.contains("err_in_loop.basm:2:5"), "{stderr}");
}
