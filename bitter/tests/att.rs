// `std.x86_64.att` against GNU `as`: one AT&T source, assembled by both.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

// A fresh directory per call, since tests run in parallel.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bitter-att-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn bitter_build(source: &Path, out: &Path) {
    let build = Command::new(env!("CARGO_BIN_EXE_bitter"))
        .current_dir(workspace())
        .args(["build", &source.display().to_string(), "-o", &out.display().to_string()])
        .output()
        .expect("bitter build should run");
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
}

// `att_forms.s` as BitterASM: the same lines, after the import.
fn forms_binary(dir: &Path) -> PathBuf {
    let gnu = std::fs::read_to_string(workspace().join("bitter/tests/fixtures/att_forms.s")).unwrap();
    let mut basm = String::from("from std.x86_64.att import *\n\npub _start:\n");
    for line in gnu.lines().filter(|line| !line.starts_with('#')) {
        basm.push_str("    ");
        basm.push_str(line);
        basm.push('\n');
    }
    let source = dir.join("att_forms.basm");
    std::fs::write(&source, basm).unwrap();
    let out = dir.join("att_forms.bin");
    bitter_build(&source, &out);
    out
}

#[test]
fn att_forms_encode_to_the_verified_bytes() {
    let dir = scratch("golden");
    let bytes = std::fs::read(forms_binary(&dir)).unwrap();
    let golden = std::fs::read(workspace().join("bitter/tests/fixtures/att_forms.golden")).unwrap();
    assert_eq!(bytes, golden);
}

// The instructions `objdump` decodes, one per line, without addresses or
// bytes.
fn disassemble(args: &[&str], path: &Path) -> Option<Vec<String>> {
    let output = Command::new("objdump").args(args).arg("--no-show-raw-insn").arg(path).output().ok()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.split_once(":\t").map(|(_, instruction)| instruction))
            .map(|instruction| instruction.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect(),
    )
}

#[test]
fn att_forms_disassemble_like_gnu_as() {
    let dir = scratch("gnu");
    let object = dir.join("att_forms.o");
    let assembled = Command::new("as")
        .arg("-o")
        .arg(&object)
        .arg(workspace().join("bitter/tests/fixtures/att_forms.s"))
        .status();
    if !matches!(assembled, Ok(status) if status.success()) {
        eprintln!("skipping: GNU `as` isn't available");
        return;
    }
    let Some(gnu) = disassemble(&["-d"], &object) else {
        eprintln!("skipping: `objdump` isn't available");
        return;
    };
    let ours = disassemble(&["-D", "-b", "binary", "-mi386:x86-64"], &forms_binary(&dir)).unwrap();

    // Two encodings differ but mean the same: `impl` gives a 64-bit `mov`
    // a full 64-bit immediate, and rotates by 1 with an immediate count.
    let ours: Vec<String> = ours
        .into_iter()
        .map(|line| line.replace("movabs $0x5,%rax", "mov $0x5,%rax").replace("rol $0x1,%al", "rol $1,%al"))
        .collect();
    assert_eq!(ours, gnu);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn the_hexdump_example_dumps_a_file() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("hexdump");
    let hexdump = dir.join("hexdump");
    bitter_build(&workspace().join("examples/x86_64/hexdump.basm"), &hexdump);
    std::fs::set_permissions(&hexdump, std::fs::Permissions::from_mode(0o755)).unwrap();

    let input = dir.join("input.bin");
    std::fs::write(&input, b"Hello, hexdump!\x00\x01\x7f\xffABC").unwrap();
    let run = Command::new(&hexdump).arg(&input).output().unwrap();
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "0000000000000000  48 65 6c 6c 6f 2c 20 68 65 78 64 75 6d 70 21 00  |Hello, hexdump!.|\n\
         0000000000000010  01 7f ff 41 42 43                                |...ABC          |\n"
    );

    let usage = Command::new(&hexdump).output().unwrap();
    assert_eq!(usage.status.code(), Some(1));
    assert_eq!(usage.stderr, b"Usage: hexdump FILE\n");

    let missing = Command::new(&hexdump).arg(dir.join("missing")).output().unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(missing.stderr, b"hexdump: open, read, or write failed\n");
}
