// Part D of `docs/1.0/PROGRESS.md`: `bitter` recognizes std types by
// `.em` id, reads only versioned `.em` files, and lays sections out the
// same way whether it encodes one file or builds several.

use std::path::Path;
use std::process::Command;

fn scratch() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bitter-em-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// Compiles `bitter/tests/fixtures/<name>.basm` to a `.em` in `dir`.
fn compile_fixture(name: &str, dir: &Path) -> std::path::PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let source = workspace.join(format!("bitter/tests/fixtures/{name}.basm"));
    let em = dir.join(format!("{name}.em"));

    // Built alongside `bitter` in the same workspace target directory.
    let bitterasm = Path::new(env!("CARGO_BIN_EXE_bitter")).with_file_name("bitterasm");
    let compile = Command::new(&bitterasm)
        .current_dir(&workspace)
        .args(["compile", &source.display().to_string(), "-o", &em.display().to_string()])
        .output()
        .expect("bitterasm compile should run");
    assert!(compile.status.success(), "{}", String::from_utf8_lossy(&compile.stderr));
    em
}

fn encode(em: &Path, bin: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bitter"))
        .args(["encode", &em.display().to_string(), "-o", &bin.display().to_string()])
        .output()
        .expect("bitter encode should run")
}

#[test]
fn a_user_struct_named_bits_is_not_packed_as_binary() {
    let dir = scratch();
    let em = compile_fixture("user_bits", &dir);
    let encode = encode(&em, &dir.join("user_bits.bin"));

    // As std.binary.bits it would encode to one byte, 0x41. As a plain
    // struct, its bare `int` field has no width to pack into.
    assert!(!encode.status.success());
    assert!(
        String::from_utf8_lossy(&encode.stderr).contains("can't tell how many bits a bare Int"),
        "{}",
        String::from_utf8_lossy(&encode.stderr)
    );
}

#[test]
fn encode_groups_sections_like_build_does() {
    let dir = scratch();
    let em = compile_fixture("sections_bytes", &dir);
    let bin = dir.join("sections_bytes.bin");

    let output = encode(&em, &bin);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    // Source order is .text 1, .data 2, .text 3; grouped: .text then .data.
    assert_eq!(std::fs::read(&bin).unwrap(), [1, 3, 2]);
}

#[test]
fn encode_rejects_an_unversioned_em_file() {
    let dir = scratch();
    let em = dir.join("old.em");
    std::fs::write(&em, r#"[{"kind": "Int", "value": "1"}]"#).unwrap();

    let output = encode(&em, &dir.join("old.bin"));
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unversioned `.em` file"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn image_start_and_end_measure_the_encoded_image() {
    let dir = scratch();
    let em = compile_fixture("image_size", &dir);
    let bin = dir.join("image_size.bin");

    let output = encode(&em, &bin);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    // 4 header bytes + 3 data bytes = 7, little-endian.
    assert_eq!(std::fs::read(&bin).unwrap(), [7, 0, 0, 0, 0xAA, 0xBB, 0xCC]);
}

#[test]
fn align_pads_after_variable_length_instructions() {
    let dir = scratch();
    let em = compile_fixture("align_x86", &dir);
    let bin = dir.join("align_x86.bin");

    let output = encode(&em, &bin);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));

    let bytes = std::fs::read(&bin).unwrap();
    // 4-byte header + `mov eax, 1` (5) + `xor edi, edi` (2) = 11, padded to 16.
    assert_eq!(&bytes[..4], [16, 0, 0, 0]);
    assert_eq!(&bytes[4..9], [0xB8, 1, 0, 0, 0]);
    assert_eq!(&bytes[9..11], [0x31, 0xFF]);
    assert_eq!(&bytes[11..16], [0; 5]);
    assert_eq!(&bytes[16..], [0xC3]);
}
