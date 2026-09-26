// Executable containers written in bitterasm (`std/formats/`, Part E of
// `docs/1.0/PROGRESS.md`), built through the real `bitterasm compile` +
// `bitter encode` pipeline.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn scratch() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bitter-formats-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// Compiles and encodes `bitter/tests/fixtures/formats/<name>.basm`,
// returning the output file's path.
fn build(name: &str) -> PathBuf {
    let dir = scratch();
    let source = workspace().join(format!("bitter/tests/fixtures/formats/{name}.basm"));
    let em = dir.join(format!("{name}.em"));
    let out = dir.join(name);

    let bitterasm = Path::new(env!("CARGO_BIN_EXE_bitter")).with_file_name("bitterasm");
    let compile = Command::new(&bitterasm)
        .current_dir(workspace())
        .args(["compile", &source.display().to_string(), "-o", &em.display().to_string()])
        .output()
        .expect("bitterasm compile should run");
    assert!(compile.status.success(), "{}", String::from_utf8_lossy(&compile.stderr));

    let encode = Command::new(env!("CARGO_BIN_EXE_bitter"))
        .args(["encode", &em.display().to_string(), "-o", &out.display().to_string()])
        .output()
        .expect("bitter encode should run");
    assert!(encode.status.success(), "{}", String::from_utf8_lossy(&encode.stderr));

    out
}

// `readelf -h -l` output, if `readelf` is installed.
fn readelf(path: &Path) -> Option<String> {
    let output = Command::new("readelf").args(["-h", "-l"]).arg(path).output().ok()?;
    assert!(output.status.success(), "readelf rejected {}: {}", path.display(), String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "readelf warned about {}: {}", path.display(), String::from_utf8_lossy(&output.stderr));
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
fn elf64_matches_the_rust_writer_it_replaced() {
    let elf = build("hello_x86_64");
    let golden = workspace().join("bitter/tests/fixtures/formats/hello_x86_64.golden");
    assert_eq!(std::fs::read(&elf).unwrap(), std::fs::read(golden).unwrap());

    if let Some(report) = readelf(&elf) {
        assert!(report.contains("ELF64"), "{report}");
        assert!(report.contains("Advanced Micro Devices X86-64"), "{report}");
        // Load address + 120 header bytes + the 14-byte message before `_start`.
        assert!(report.contains("Entry point address:               0x400086"), "{report}");
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn elf64_hello_runs() {
    use std::os::unix::fs::PermissionsExt;

    let elf = build("hello_x86_64");
    std::fs::set_permissions(&elf, std::fs::Permissions::from_mode(0o755)).unwrap();

    let run = Command::new(&elf).output().expect("the executable should run");
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(run.stdout, b"Hello, World!\n");
}

#[test]
fn elf32_riscv_header_and_code() {
    let elf = build("exit_riscv");
    let bytes = std::fs::read(&elf).unwrap();

    // 84 bytes of header, then `addi a0, zero, 42`, `addi a7, zero, 93`,
    // `ecall`.
    assert_eq!(bytes.len(), 96);
    assert_eq!(&bytes[..6], [0x7F, b'E', b'L', b'F', 1, 1]);
    assert_eq!(&bytes[84..], [0x13, 0x05, 0xA0, 0x02, 0x93, 0x08, 0xD0, 0x05, 0x73, 0x00, 0x00, 0x00]);

    if let Some(report) = readelf(&elf) {
        assert!(report.contains("ELF32"), "{report}");
        assert!(report.contains("RISC-V"), "{report}");
        assert!(report.contains("Entry point address:               0x10054"), "{report}");
        assert!(report.contains("LOAD           0x000000 0x00010000 0x00010000 0x00060 0x00060 R E 0x1000"), "{report}");
    }
}
