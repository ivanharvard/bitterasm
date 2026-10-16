// Absolute imports (`from std.x import ...`) are looked up under the current
// directory, then each `BITTERASM_PATH` entry, the XDG data roots on
// non-Apple Unix, and finally the legacy install root `~/.bitterasm` — see
// `loader::search_roots`. Each test runs the real CLI from a scratch directory
// outside this repo, so the repo's own `std/` can't satisfy the import by accident.

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
    compile_with(cwd, home, bitterasm_path, |_| {})
}

fn compile_with(
    cwd: &Path,
    home: &Path,
    bitterasm_path: Option<&Path>,
    configure: impl FnOnce(&mut Command),
) -> Output {
    fs::write(cwd.join("main.basm"), PROGRAM).unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_bitterasm"));
    command
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_DATA_HOME", home.join("xdg-data"))
        .env("XDG_DATA_DIRS", home.join("xdg-data-dirs"))
        .env_remove("BITTERASM_PATH")
        .args(["compile", "main.basm", "-o", "main.em"]);

    if let Some(path) = bitterasm_path {
        command.env("BITTERASM_PATH", path);
    }

    configure(&mut command);

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

#[cfg(all(unix, not(target_vendor = "apple")))]
#[test]
fn xdg_data_home_shadows_data_dirs_and_the_legacy_install_root() {
    let dir = scratch("xdg_home_priority");
    let cwd = dir.join("project");
    let home = dir.join("home");
    let data_home = dir.join("data-home");
    let data_dir = dir.join("data-dir");
    fs::create_dir_all(&cwd).unwrap();
    write_probe(&home.join(".bitterasm"), 7);
    write_probe(&data_dir.join("bitterasm"), 11);
    write_probe(&data_home.join("bitterasm"), 13);

    let output = compile_with(&cwd, &home, None, |command| {
        command
            .env("XDG_DATA_HOME", &data_home)
            .env("XDG_DATA_DIRS", &data_dir);
    });

    assert!(emitted(&cwd, &output).contains("\"13\""));
}

#[cfg(all(unix, not(target_vendor = "apple")))]
#[test]
fn xdg_data_dirs_preserve_order_and_ignore_relative_entries() {
    let dir = scratch("xdg_dirs_order");
    let cwd = dir.join("project");
    let home = dir.join("home");
    let first = dir.join("first");
    let second = dir.join("second");
    let relative = Path::new("relative");
    fs::create_dir_all(&cwd).unwrap();
    write_probe(&home.join(".bitterasm"), 7);
    write_probe(&cwd.join(relative).join("bitterasm"), 17);
    write_probe(&first.join("bitterasm"), 19);
    write_probe(&second.join("bitterasm"), 23);
    let data_dirs = std::env::join_paths([relative, first.as_path(), second.as_path()]).unwrap();

    let output = compile_with(&cwd, &home, None, |command| {
        command
            .env("XDG_DATA_HOME", dir.join("missing-data-home"))
            .env("XDG_DATA_DIRS", data_dirs);
    });

    assert!(emitted(&cwd, &output).contains("\"19\""));
}

#[cfg(all(unix, not(target_vendor = "apple")))]
#[test]
fn empty_or_relative_xdg_data_home_uses_the_default() {
    for (name, data_home) in [("empty", ""), ("relative", "relative")] {
        let dir = scratch(&format!("xdg_home_{name}"));
        let cwd = dir.join("project");
        let home = dir.join("home");
        fs::create_dir_all(&cwd).unwrap();
        write_probe(&home.join(".local/share/bitterasm"), 29);

        let output = compile_with(&cwd, &home, None, |command| {
            command
                .env("XDG_DATA_HOME", data_home)
                .env("XDG_DATA_DIRS", dir.join("missing-data-dirs"));
        });

        assert!(emitted(&cwd, &output).contains("\"29\""));
    }
}

#[cfg(all(unix, not(target_vendor = "apple")))]
#[test]
fn empty_or_relative_xdg_data_dirs_use_the_defaults() {
    for (name, data_dirs) in [("empty", ""), ("relative", "relative")] {
        let dir = scratch(&format!("xdg_dirs_{name}"));
        let cwd = dir.join("project");
        let home = dir.join("home");
        fs::create_dir_all(&cwd).unwrap();

        let output = compile_with(&cwd, &home, None, |command| {
            command
                .env("XDG_DATA_HOME", dir.join("missing-data-home"))
                .env("XDG_DATA_DIRS", data_dirs);
        });
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(!output.status.success());
        assert!(stderr.contains("/usr/local/share/bitterasm"), "{stderr}");
        assert!(stderr.contains("/usr/share/bitterasm"), "{stderr}");
    }
}
