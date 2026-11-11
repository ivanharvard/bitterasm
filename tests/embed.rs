// `@embed`, `std.embed` and `std.formats.ihex`, through the real CLI.

mod common;

const DIR: &str = "tests/fixtures/embed";

fn fixture(name: &str) -> String {
    format!("{DIR}/{name}")
}

fn file(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR).join(name)).unwrap()
}

#[test]
fn embed_gives_the_files_units_as_they_are_stored() {
    // `len`, each unit, then one read by name. `é` is UTF-8's two bytes,
    // not one character: nothing is decoded.
    let values = common::compile_ints(&fixture("units.basm"));
    assert_eq!(values, [5, 65, 195, 169, 0, 255, 195]);
}

#[test]
fn an_embed_path_starts_from_the_file_its_written_in() {
    // The macros are in `lib/`, beside `inner.bin`; the caller isn't.
    let values = common::compile_ints(&fixture("uses_lib.basm"));
    assert_eq!(values, [2, 0x2a]);
}

#[test]
fn a_missing_embed_file_is_a_compile_error() {
    let error = common::compile_error(&fixture("err_missing.basm"));
    assert!(error.contains("can't embed") && error.contains("no_such_file.bin"), "{error}");
}

#[test]
fn emit_bytes_pastes_a_file_untouched() {
    assert_eq!(common::encode(&fixture("paste.basm")), file("data.bin"));
}
