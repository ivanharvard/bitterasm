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

#[test]
fn ihex_writes_what_objcopy_does() {
    // Both golden files are `avr-objcopy -I binary -O ihex`'s output.
    assert_eq!(common::encode(&fixture("ihex_blink.basm")), file("blink.hex"));
}

#[test]
fn ihex_starts_a_new_block_at_64_kib() {
    // `objcopy --change-addresses 0xfff8`, without the x86 start address
    // record it adds from the entry point.
    assert_eq!(common::encode(&fixture("ihex_shifted.basm")), file("shifted.hex"));
}

#[test]
fn a_file_past_the_limit_is_a_compile_error() {
    // Written here rather than kept as a fixture: it's a megabyte.
    let dir = std::env::temp_dir().join(format!("bitterasm-embed-limit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("huge.bin"), vec![0u8; 1_000_001]).unwrap();
    let source = dir.join("huge.basm");
    std::fs::write(&source, "macro show(v: int) {\n    @emit v\n}\n\nshow (@embed \"huge.bin\").len\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_bitterasm"))
        .args(["check", &source.display().to_string()])
        .output()
        .unwrap();
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(error.contains("holds 1000001 units") && error.contains("at most 1000000"), "{error}");

    std::fs::remove_dir_all(&dir).ok();
}
