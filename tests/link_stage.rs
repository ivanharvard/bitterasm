// `@link`: a program's link macro runs once everything else has, and what
// it emits goes in the link section.

mod common;

use bitterasm::emit::LINK_SECTION;
use common::{compile_error, compile_file};

#[test]
fn the_link_macro_runs_last_into_the_link_section() {
    let file = compile_file("tests/fixtures/link/runs_last.basm");
    let sections: Vec<Option<&str>> = file.entries.iter().map(|entry| entry.section.as_deref()).collect();
    assert_eq!(sections, [None, None, Some(LINK_SECTION)]);
    assert!(file.requires.iter().any(|feature| feature == "link"), "{:?}", file.requires);
}

#[test]
fn only_the_programs_own_link_runs() {
    let file = compile_file("tests/fixtures/link/imported_link.basm");
    assert!(file.entries.is_empty());
}

#[test]
fn link_goes_once_at_the_top_of_the_program() {
    let stderr = compile_error("tests/fixtures/link/err_in_macro.basm");
    assert!(stderr.contains("`@link` goes at the top of the program's own file, not in a macro"), "{stderr}");
    let stderr = compile_error("tests/fixtures/link/err_twice.basm");
    assert!(stderr.contains("a program has one `@link`"), "{stderr}");
    let stderr = compile_error("tests/fixtures/link/err_not_a_call.basm");
    assert!(stderr.contains("`@link` takes one macro call"), "{stderr}");
}
