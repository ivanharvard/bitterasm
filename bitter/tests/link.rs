// Phase 6 (see `docs/sections-and-linking/PROGRESS.md`): `bitter build`
// accepts multiple `.basm` inputs and links them (same-named sections
// concatenated in argument order, `pub` labels resolved across files). The
// fixtures write their own ELF header with `std.formats.elf`, so the result
// is a real, runnable executable — verified the way
// `examples/x86_64/hello.basm` is: actually build and run it.
//
// Linux/x86-64 only, like `phase6_entry.basm`/`phase6_dep.basm` themselves
// (real syscalls, real machine code) — skipped everywhere else, the same
// way a `hello.basm`-style manual check only ever made sense on the OS/ISA
// it actually targets.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::path::Path;
use std::process::Command;

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .display()
        .to_string()
}

// `std.x86_64.intel`-style absolute imports resolve relative to the
// current working directory (`loader::module_base_dir`), which the repo
// root is, not `bitter`'s own `CARGO_MANIFEST_DIR` — every `bitter build`
// invocation below needs this as its subprocess's own CWD so the
// `bitterasm compile` it shells out to (inheriting CWD by default) can
// find `std/x86_64/intel.basm`.
fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn two_files_link_and_the_resulting_executable_actually_runs_correctly() {
    let bitter = env!("CARGO_BIN_EXE_bitter");
    let out = std::env::temp_dir().join("bitter-link-test-phase6");

    let build = Command::new(bitter)
        .current_dir(workspace_root())
        .args([
            "build",
            &fixture("phase6_entry.basm"),
            &fixture("phase6_dep.basm"),
            "-o",
            &out.display().to_string(),
        ])
        .output()
        .expect("bitter build should run");

    assert!(
        build.status.success(),
        "bitter build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&out).status().expect("the linked executable should run");

    // `phase6_entry.basm` is nothing but `call after_first`, a relative
    // jump into `phase6_dep.basm`'s own code — the exit code 7 only exists
    // over there, so seeing it out of the *linked* executable is direct
    // proof the cross-file `Deferred` reference (Phase 5) resolved to the
    // real, merged byte offset (Phase 6), not a placeholder, a wrong
    // address, or a crash.
    assert_eq!(run.code(), Some(7), "unexpected exit code from the linked executable");
}

#[test]
fn the_entry_point_can_be_a_label_in_another_input() {
    let bitter = env!("CARGO_BIN_EXE_bitter");
    let out = std::env::temp_dir().join(format!("bitter-link-test-entry-elsewhere-{}", std::process::id()));

    let build = Command::new(bitter)
        .current_dir(workspace_root())
        .args([
            "build",
            &fixture("entry_in_other_file.basm"),
            &fixture("phase6_dep.basm"),
            "-o",
            &out.display().to_string(),
        ])
        .output()
        .expect("bitter build should run");
    assert!(build.status.success(), "bitter build failed: {}", String::from_utf8_lossy(&build.stderr));

    // The header's `e_entry` names `after_first` in `phase6_dep.basm`:
    // starting there exits with 7, where the first file's own code would
    // exit with 1.
    let run = Command::new(&out).status().expect("the linked executable should run");
    assert_eq!(run.code(), Some(7));
}

#[test]
fn a_single_file_build_is_unaffected_by_multi_file_support() {
    // Regression bar: `bitter build`'s existing one-`.basm`-file behavior
    // (unchanged since before Phase 6) still works byte-for-byte the same
    // way — `hello.basm` itself, actually run.
    let bitter = env!("CARGO_BIN_EXE_bitter");
    let out = std::env::temp_dir().join("bitter-link-test-hello");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let hello = Path::new(manifest_dir).join("../examples/x86_64/hello.basm");

    let build = Command::new(bitter)
        .current_dir(workspace_root())
        .args(["build", &hello.display().to_string(), "-o", &out.display().to_string()])
        .output()
        .expect("bitter build should run");

    assert!(
        build.status.success(),
        "bitter build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new(&out).output().expect("hello.basm's executable should run");

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&run.stdout), "Hello, World!\n");
}
