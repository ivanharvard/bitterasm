// `std.avr` against GNU `avr-as`, without needing Docker: each
// `tests/avr/cases/*.golden` is the bytes `tests/avr/run_tests.py`'s
// oracle produced for the `.s` file beside it. Regenerate one with
// `docker run --rm -i bitterasm-avr-oracle < case.s > case.golden` after
// building the image in `tests/avr/docker`.
//
// `bitter` is a separate package, so it's built on demand next to
// `bitterasm`'s binary, as in `tests/pdp10_encoding.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

fn bitter_bin() -> PathBuf {
    static BUILD_ONCE: Once = Once::new();

    let bitterasm = PathBuf::from(env!("CARGO_BIN_EXE_bitterasm"));
    let bitter = bitterasm.with_file_name(if cfg!(windows) { "bitter.exe" } else { "bitter" });

    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "--package", "bitter", "--quiet"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo build --package bitter should run");
        assert!(status.success(), "failed to build the `bitter` binary");
    });

    bitter
}

fn work_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bitterasm-avr-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Compiles `source` and encodes it, returning the bytes, or the
/// compiler's error output.
fn build(name: &str, source: &str) -> Result<Vec<u8>, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dir = work_dir();
    let basm = dir.join(format!("{name}.basm"));
    let em = dir.join(format!("{name}.em"));
    let bin = dir.join(format!("{name}.bin"));
    std::fs::write(&basm, source).unwrap();

    let compiled = Command::new(env!("CARGO_BIN_EXE_bitterasm"))
        .current_dir(manifest_dir)
        .args(["compile", &basm.display().to_string(), "-o", &em.display().to_string()])
        .output()
        .unwrap();
    if !compiled.status.success() {
        return Err(String::from_utf8_lossy(&compiled.stderr).into_owned());
    }

    let encoded = Command::new(bitter_bin())
        .args(["encode", &em.display().to_string(), "-o", &bin.display().to_string()])
        .output()
        .unwrap();
    assert!(encoded.status.success(), "bitter encode failed for {name}:\n{}", String::from_utf8_lossy(&encoded.stderr));

    Ok(std::fs::read(&bin).unwrap())
}

/// A case's `.s` text as BitterASM.
fn as_basm(text: &str) -> String {
    format!("from std.avr.native import *\n\n{text}")
}

#[test]
fn every_case_matches_gnu_as() {
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/avr/cases");
    let mut checked = 0;

    for entry in std::fs::read_dir(&cases).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "s") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path).unwrap();
        let expected = std::fs::read(path.with_extension("golden")).unwrap();

        let actual = build(&name, &as_basm(&text)).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(actual, expected, "{name}: bitter's bytes differ from GNU as's");
        checked += 1;
    }

    assert!(checked >= 5, "expected the AVR cases, found {checked}");
}

#[test]
fn blink_example_builds() {
    let source = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/avr/blink.basm")).unwrap();
    let bytes = build("blink", &source).unwrap();

    // Checked against `avr-objdump -D -b binary -m avr5`.
    let expected = [
        0x08, 0xe0, 0x0e, 0xbf, 0x0f, 0xef, 0x0d, 0xbf, 0x25, 0x9a, 0x1d, 0x9a, 0x01, 0xd0, 0xfd, 0xcf,
        0x21, 0xe1, 0x3f, 0xef, 0x4f, 0xef, 0x4a, 0x95, 0xf1, 0xf7, 0x3a, 0x95, 0xd9, 0xf7, 0x2a, 0x95,
        0xc1, 0xf7, 0x08, 0x95,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn operands_out_of_range_are_compile_errors() {
    for (name, line, message) in [
        ("ldi_low_reg", "ldi r0, 1", "this instruction takes r16 to r31"),
        ("imm_too_big", "ldi r16, 256", "an 8-bit immediate is -128 to 255"),
        ("adiw_pair", "adiw r25, 1", "this instruction takes r24, r26, r28 or r30"),
        ("sbi_io", "sbi 0x20, 0", "this instruction takes I/O address 0 to 31"),
        ("ldd_x", "ldd r0, X+1", "a displacement takes `Y` or `Z`"),
        ("lpm_y", "lpm r0, Y", "`lpm` takes `Z` or `Z+`"),
        ("odd_jmp", "jmp 3", "a flash address is even"),
    ] {
        let error = build(name, &format!("from std.avr.native import *\n\n{line}\n")).expect_err(name);
        assert!(error.contains(message), "{name}: expected `{message}` in:\n{error}");
    }
}
