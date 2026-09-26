// Phase D1 of `docs/1.0/PROGRESS.md`: `bitter` recognizes std types by
// `.em` id, so a same-named struct elsewhere isn't mistaken for one.

use std::path::Path;
use std::process::Command;

#[test]
fn a_user_struct_named_bits_is_not_packed_as_binary() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let source = workspace.join("bitter/tests/fixtures/user_bits.basm");
    let dir = std::env::temp_dir().join(format!("bitter-ids-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let em = dir.join("user_bits.em");

    // Built alongside `bitter` in the same workspace target directory.
    let bitterasm = Path::new(env!("CARGO_BIN_EXE_bitter")).with_file_name("bitterasm");
    let compile = Command::new(&bitterasm)
        .current_dir(&workspace)
        .args(["compile", &source.display().to_string(), "-o", &em.display().to_string()])
        .output()
        .expect("bitterasm compile should run");
    assert!(compile.status.success(), "{}", String::from_utf8_lossy(&compile.stderr));

    let encode = Command::new(env!("CARGO_BIN_EXE_bitter"))
        .args(["encode", &em.display().to_string(), "-o", &dir.join("user_bits.bin").display().to_string()])
        .output()
        .expect("bitter encode should run");

    // As std.binary.bits it would encode to one byte, 0x41. As a plain
    // struct, its bare `int` field has no width to pack into.
    assert!(!encode.status.success());
    assert!(
        String::from_utf8_lossy(&encode.stderr).contains("can't tell how many bits a bare Int"),
        "{}",
        String::from_utf8_lossy(&encode.stderr)
    );
}
