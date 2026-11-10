// Link stages (`@link`, `std.bitter.image`): the output built from what a
// link macro emits, writing and placing sections.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

// A fresh directory per call, since tests run in parallel.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bitter-image-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn build(fixtures: &[&str], out: &Path) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bitter"));
    command.current_dir(workspace()).arg("build");
    for fixture in fixtures {
        command.arg(workspace().join("bitter/tests/fixtures/link").join(fixture));
    }
    command.arg("-o").arg(out).output().expect("bitter build should run")
}

#[test]
fn sections_are_written_and_placed_by_the_link_macro() {
    let dir = scratch("placed");
    let out = dir.join("placed.bin");
    let built = build(&["placed.basm"], &out);
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    // `address(target)`, `offset(target)`, then `.text` (1, and the
    // distance from 0x11 to 0x40) and `.data` (2).
    assert_eq!(std::fs::read(&out).unwrap(), [0x40, 0x04, 0x01, 0x2f, 0x02]);
}

#[test]
fn every_section_with_values_goes_to_the_link_stage() {
    let dir = scratch("left-out");
    let built = build(&["left_out.basm"], &dir.join("out"));
    assert!(!built.status.success());
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(stderr.contains("1 value(s) are in a section the link stage neither writes nor places"), "{stderr}");
}

#[test]
fn an_offset_needs_a_link_stage() {
    let dir = scratch("no-link");
    let built = build(&["offset_without_link.basm"], &dir.join("out"));
    assert!(!built.status.success());
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(stderr.contains("`Deferred.Offset` needs a link stage"), "{stderr}");
}

#[test]
fn a_link_stage_is_built_from_one_file() {
    let dir = scratch("two-files");
    let built = build(&["placed.basm", "offset_without_link.basm"], &dir.join("out"));
    assert!(!built.status.success());
    let stderr = String::from_utf8_lossy(&built.stderr);
    assert!(stderr.contains("is built from one file for now"), "{stderr}");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn an_elf_link_gives_each_section_its_own_segment_and_zeroes_bss() {
    use std::os::unix::fs::PermissionsExt;

    let dir = scratch("elf");
    let out = dir.join("elf_bss");
    let built = build(&["elf_bss.basm"], &out);
    assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
    std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755)).unwrap();

    let run = Command::new(&out).output().unwrap();
    assert_eq!(run.status.code(), Some(7));

    if let Ok(readelf) = Command::new("readelf").args(["-W", "-l"]).arg(&out).output() {
        let report = String::from_utf8_lossy(&readelf.stdout);
        let loads: Vec<&str> = report.lines().filter(|line| line.trim_start().starts_with("LOAD")).collect();
        assert_eq!(loads.len(), 3, "{report}");
        assert!(loads[0].ends_with("R E 0x1000"), "{report}");
        assert!(loads[1].ends_with("R   0x1000"), "{report}");
        // `.bss`: nothing in the file, 4 bytes in memory.
        assert!(loads[2].contains("0x000000 0x000004 RW  0x1000"), "{report}");
    }
}
