use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod link;
mod pack;

#[derive(Parser)]
#[command(name = "bitter", version, about = "Exporter CLI for BitterASM-emitted values")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Pack a .em file's emitted value stream into raw machine-code bytes.
    Encode {
        path: PathBuf,

        /// Defaults to `path` with its extension swapped to `.bin`.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// The whole pipeline in one command: compile one or more .basm
    /// programs, link them together, and write the packed bytes, marked
    /// executable — `bitterasm compile` + `bitter encode` without the
    /// intermediate .em files. Given more than one path, same-named
    /// `section`s are concatenated across every input (command-line order),
    /// and a `pub` label imported (`from file import label`) in one input
    /// but declared in another resolves against that other input.
    ///
    /// `bitter` knows no executable format itself: a program that should
    /// run as an ELF, PE or Mach-O executable writes that header in
    /// bitterasm, e.g. `elf64_executable EM_X86_64, _start` from
    /// `std.formats.elf`, first thing in the first input. Without one, the
    /// output is a flat binary.
    Build {
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Defaults to the first path with its extension dropped.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Anything `bitter` doesn't recognize itself is handed to `bitterasm`
    /// unchanged (`bitter check foo.basm` runs `bitterasm check foo.basm`)
    /// — same delegate-to-a-sibling-binary shape as `cargo clippy`/`git
    /// lfs`, so `bitter` picks up every `bitterasm` command for free,
    /// with no rebuild and no dependency on `bitterasm`'s Rust API.
    #[command(external_subcommand)]
    External(Vec<String>),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Encode { path, output } => encode(&path, output),
        Command::Build { paths, output } => build(&paths, output),
        Command::External(args) => delegate_to_bitterasm(&args),
    }
}

/// `bitterasm`'s own executable, preferring one installed alongside this
/// `bitter` binary (how `install.sh` lays both out) over whatever a bare
/// `PATH` lookup would find, the same order `cargo` resolves `cargo-<x>`
/// plugin binaries in.
fn bitterasm_path() -> PathBuf {
    let exe_name = if cfg!(windows) { "bitterasm.exe" } else { "bitterasm" };

    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(exe_name)))
        .filter(|sibling| sibling.is_file())
        .unwrap_or_else(|| PathBuf::from(exe_name))
}

fn delegate_to_bitterasm(args: &[String]) {
    let bitterasm = bitterasm_path();

    // Inherits stdin/stdout/stderr by default, so `bitterasm`'s own output
    // (including `--help`/error messages, which will say `bitterasm`, not
    // `bitter` — it only knows its own argv[0]) reaches the user exactly as
    // if they'd run it directly.
    match std::process::Command::new(&bitterasm).args(args).status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),

        Err(error) => {
            let command = args.first().map(String::as_str).unwrap_or("");
            eprintln!(
                "'{command}' is not a bitter command, and running `{}` failed: {error}",
                bitterasm.display(),
            );
            std::process::exit(1);
        }
    }
}

fn encode(path: &PathBuf, output: Option<PathBuf>) {
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,

        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    let file = read_em(path, &json);

    // Laid out exactly as `bitter build` lays out one input: grouped by
    // section, with label positions translated to match. A cross-file
    // reference has no input to resolve against here and is an error.
    let input = link::LinkInput {
        file: path.clone(),
        module: file.module,
        entries: file.entries,
        labels: exports_as_positions(file.exports),
    };
    let values = match link::link(vec![input]) {
        Ok(linked) => linked.values,

        Err(error) => {
            eprintln!("failed to encode {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    let bytes = match pack::pack_stream(&values) {
        Ok(bytes) => bytes,

        Err(error) => {
            eprintln!("failed to encode {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    let output_path = output.unwrap_or_else(|| path.with_extension("bin"));

    if let Err(error) = std::fs::write(&output_path, &bytes) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!("encoded {} byte(s) to {}", bytes.len(), output_path.display());
}

/// The `.em` features `bitter` understands — every one the format defines.
const SUPPORTED_FEATURES: &[&str] = bitterasm::emit::features::ALL;

/// Parses a `.em` file, exiting with the reason if `bitter` can't read it.
fn read_em(path: &std::path::Path, json: &str) -> bitterasm::emit::EmFile {
    bitterasm::emit::EmFile::parse(json, SUPPORTED_FEATURES).unwrap_or_else(|error| {
        eprintln!("can't read {}: {error}", path.display());
        std::process::exit(1);
    })
}

fn exports_as_positions(exports: std::collections::BTreeMap<String, u64>) -> std::collections::HashMap<String, usize> {
    exports
        .into_iter()
        .map(|(name, position)| (name, usize::try_from(position).expect("a label position fits in usize")))
        .collect()
}

/// Runs `bitterasm compile <path> -o <em_path>` as a subprocess (the same
/// sibling-binary lookup `delegate_to_bitterasm` uses), the way a real
/// `cc`-style driver shells out to a separate compiler pass rather than
/// re-implementing it.
fn run_bitterasm_compile(path: &std::path::Path, em_path: &std::path::Path) {
    let bitterasm = bitterasm_path();

    let status = std::process::Command::new(&bitterasm)
        .args(["compile", &path.display().to_string(), "-o", &em_path.display().to_string()])
        .status();

    match status {
        Ok(status) if status.success() => {}
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),

        Err(error) => {
            eprintln!("failed to run `{} compile`: {error}", bitterasm.display());
            std::process::exit(1);
        }
    }
}

/// Compiles one `bitter build` input and reads its `.em` back as a
/// `link::LinkInput`: its entries, its module path (what other inputs'
/// `Deferred` references name it by) and its exported `pub` labels.
fn compile_input(path: &std::path::Path, unique: &str) -> link::LinkInput {
    let em_path = std::env::temp_dir().join(format!("bitter-build-{unique}.em"));

    run_bitterasm_compile(path, &em_path);

    let em_json = std::fs::read_to_string(&em_path).unwrap_or_else(|error| {
        eprintln!("failed to read {}: {error}", em_path.display());
        std::process::exit(1);
    });
    let _ = std::fs::remove_file(&em_path);

    let file = read_em(&em_path, &em_json);

    link::LinkInput {
        file: path.to_path_buf(),
        module: file.module,
        entries: file.entries,
        labels: exports_as_positions(file.exports),
    }
}

fn build(paths: &[PathBuf], output: Option<PathBuf>) {
    let output_path = output.unwrap_or_else(|| paths[0].with_extension(""));

    let inputs: Vec<link::LinkInput> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| compile_input(path, &format!("{}-{index}", std::process::id())))
        .collect();

    let values = match link::link(inputs) {
        Ok(linked) => linked.values,

        Err(error) => {
            eprintln!("failed to link: {error}");
            std::process::exit(1);
        }
    };

    let bytes = match pack::pack_stream(&values) {
        Ok(bytes) => bytes,

        Err(error) => {
            eprintln!("failed to encode: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = write_executable(&output_path, &bytes) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!(
        "built {} byte(s) from {} input(s) into {}",
        bytes.len(),
        paths.len(),
        output_path.display()
    );
}

/// Writes `bytes` to `path`, marked executable where that's a file
/// permission (on Windows, a PE runs by being named `.exe` instead).
fn write_executable(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}
