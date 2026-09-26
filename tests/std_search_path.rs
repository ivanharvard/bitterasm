// Absolute imports (`from std.x import ...`) are looked up under the current
// directory, then each `BITTERASM_PATH` entry, then the install root
// `~/.bitterasm` — see `loader::search_roots`. Each test runs the real CLI
// from a scratch directory outside this repo, so the repo's own `std/` can't
// satisfy the import by accident.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const PROGRAM: &str = "from std.probe import answer\n\
                       macro show(value: int) {\n    @emit value\n}\n\
                       show answer\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "bitterasm_std_search_{name}_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_probe(root: &Path, answer: u32) {
    fs::create_dir_all(root.join("std")).unwrap();
    fs::write(
        root.join("std/probe.basm"),
        format!("pub const answer = {answer}\n"),
    )
    .unwrap();
}

fn compile(cwd: &Path, home: &Path, bitterasm_path: Option<&Path>) -> Output {
    fs::write(cwd.join("main.basm"), PROGRAM).unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_bitterasm"));
    command
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("BITTERASM_PATH")
        .args(["compile", "main.basm", "-o", "main.em"]);

    if let Some(path) = bitterasm_path {
        command.env("BITTERASM_PATH", path);
    }

    command.output().expect("bitterasm compile should run")
}

fn emitted(cwd: &Path, output: &Output) -> String {
    assert!(
        output.status.success(),
        "bitterasm compile failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read_to_string(cwd.join("main.em")).unwrap()
}

#[test]
fn falls_back_to_the_installed_std_under_home() {
    let dir = scratch("home");
    let (cwd, home) = (dir.join("project"), dir.join("home"));
    fs::create_dir_all(&cwd).unwrap();
    write_probe(&home.join(".bitterasm"), 7);

    let output = compile(&cwd, &home, None);
    assert!(emitted(&cwd, &output).contains("\"7\""));
}

#[test]
fn bitterasm_path_is_searched_before_the_install_root() {
    let dir = scratch("env");
    let (cwd, home, extra) = (dir.join("project"), dir.join("home"), dir.join("extra"));
    fs::create_dir_all(&cwd).unwrap();
    write_probe(&home.join(".bitterasm"), 7);
    write_probe(&extra, 11);

    let output = compile(&cwd, &home, Some(&extra));
    assert!(emitted(&cwd, &output).contains("\"11\""));
}

#[test]
fn the_current_directory_shadows_every_other_root() {
    let dir = scratch("cwd");
    let (cwd, home, extra) = (dir.join("project"), dir.join("home"), dir.join("extra"));
    write_probe(&cwd, 3);
    write_probe(&home.join(".bitterasm"), 7);
    write_probe(&extra, 11);

    let output = compile(&cwd, &home, Some(&extra));
    assert!(emitted(&cwd, &output).contains("\"3\""));
}

#[test]
fn a_missing_module_lists_every_root_searched() {
    let dir = scratch("missing");
    let (cwd, home) = (dir.join("project"), dir.join("home"));
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&home).unwrap();

    let output = compile(&cwd, &home, None);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("could not find module `std.probe`"),
        "{stderr}"
    );
    assert!(stderr.contains(".bitterasm"), "{stderr}");
}
