use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use dialoguer::theme::ColorfulTheme;
use dialoguer::MultiSelect;

const COMPONENTS: &[(&str, &str)] = &[
    ("bitterasm", "bitterasm language (compiler)"),
    ("bitter", "bitter CLI (exporter)"),
];

pub fn run() {
    let selected = match prompt_components() {
        Ok(selected) => selected,

        Err(error) => {
            eprintln!("install error: {error}");
            std::process::exit(1);
        }
    };

    let workspace_root = workspace_root();

    let install_root = match install_root() {
        Ok(root) => root,

        Err(error) => {
            eprintln!("install error: {error}");
            std::process::exit(1);
        }
    };

    let bin_dir = install_root.join("bin");

    for package in &selected {
        println!("Building {package} (release)...");

        let status = Command::new("cargo")
            .args(["build", "--release", "--package", package])
            .current_dir(&workspace_root)
            .status();

        match status {
            Ok(status) if status.success() => {}

            Ok(status) => {
                eprintln!("install error: cargo build for {package} exited with {status}");
                std::process::exit(1);
            }

            Err(error) => {
                eprintln!("install error: failed to run cargo: {error}");
                std::process::exit(1);
            }
        }

        if let Err(error) = fs::create_dir_all(&bin_dir) {
            eprintln!("install error: couldn't create {}: {error}", bin_dir.display());
            std::process::exit(1);
        }

        let built = workspace_root.join("target/release").join(package);
        let dest = bin_dir.join(package);

        if let Err(error) = fs::copy(&built, &dest) {
            eprintln!(
                "install error: couldn't copy {} to {}: {error}",
                built.display(),
                dest.display()
            );
            std::process::exit(1);
        }

        println!("  installed {}", dest.display());
    }

    println!("Installing standard library...");

    let std_dir = install_root.join("std");

    if let Err(error) = install_stdlib(&workspace_root.join("std"), &std_dir) {
        eprintln!("install error: couldn't install stdlib: {error}");
        std::process::exit(1);
    }

    println!("  installed {}", std_dir.display());

    if !selected.is_empty() {
        print_path_hint(&bin_dir);
    }
}

fn prompt_components() -> io::Result<Vec<&'static str>> {
    let labels: Vec<&str> = COMPONENTS.iter().map(|(_, label)| *label).collect();
    let defaults = vec![true; COMPONENTS.len()];

    let chosen = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select components to install (space to toggle, enter to confirm)")
        .items(&labels)
        .defaults(&defaults)
        .interact()?;

    Ok(chosen.into_iter().map(|i| COMPONENTS[i].0).collect())
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is this crate's own directory (`<workspace>/bitter`)
    // at compile time; the workspace root is always its parent.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bitter crate has a parent directory")
        .to_path_buf()
}

fn install_root() -> io::Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| {
        io::Error::new(io::ErrorKind::NotFound, "HOME is not set, don't know where to install")
    })?;

    Ok(Path::new(&home).join(".bitterasm"))
}

fn install_stdlib(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() {
        fs::remove_dir_all(to)?;
    }

    copy_dir_recursive(from, to)
}

fn copy_dir_recursive(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;

    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }

    Ok(())
}

fn print_path_hint(bin_dir: &Path) {
    let already_on_path = std::env::var("PATH")
        .map(|path| std::env::split_paths(&path).any(|entry| entry == bin_dir))
        .unwrap_or(false);

    println!();
    println!("Done.");

    if !already_on_path {
        println!();
        println!("{} isn't on your PATH yet. Add it with:", bin_dir.display());
        println!();
        println!("    export PATH=\"{}:$PATH\"", bin_dir.display());
    }
}
