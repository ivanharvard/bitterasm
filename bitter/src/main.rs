use std::path::PathBuf;

use bitterasm::emit::EmittedValue;
use clap::{Parser, Subcommand};

mod formats;
mod pack;

use formats::Format;

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

    /// Wrap a raw machine-code (.bin) file in a native, runnable executable
    /// container (ELF, PE, or Mach-O) for the given, or the host, OS.
    Exec {
        path: PathBuf,

        /// Defaults to `path` with its extension dropped (`.exe` added
        /// back for `--format pe`).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// `elf`, `pe`, or `macho`. Defaults to whatever this build of
        /// `bitter` is itself running on.
        #[arg(short, long)]
        format: Option<String>,
    },

    /// The whole pipeline in one command: compile a .basm program, encode
    /// it, and wrap the result in a native executable — equivalent to
    /// `bitterasm compile` + `bitter encode` + `bitter exec` run in
    /// sequence, without the intermediate .em/.bin files.
    Build {
        path: PathBuf,

        /// Defaults to `path` with its extension dropped (`.exe` added
        /// back for `--format pe`).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// `elf`, `pe`, or `macho`. Defaults to whatever this build of
        /// `bitter` is itself running on.
        #[arg(short, long)]
        format: Option<String>,
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
        Command::Exec { path, output, format } => exec(&path, output, format),
        Command::Build { path, output, format } => build(&path, output, format),
        Command::External(args) => delegate_to_bitterasm(&args),
    }
}

/// Parses `--format`, exiting with a clear error on an unknown name, or
/// falls back to [`Format::native`].
fn resolve_format(format: Option<String>) -> Format {
    match format {
        Some(name) => Format::parse(&name).unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1);
        }),
        None => Format::native(),
    }
}

fn output_path_for(path: &std::path::Path, format: Format) -> PathBuf {
    let stem = path.with_extension("");
    if format.extension().is_empty() { stem } else { stem.with_extension(format.extension()) }
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

    let values: Vec<EmittedValue> = match serde_json::from_str(&json) {
        Ok(values) => values,

        Err(error) => {
            eprintln!("failed to parse {}: {error}", path.display());
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

fn exec(path: &PathBuf, output: Option<PathBuf>, format: Option<String>) {
    let code = match std::fs::read(path) {
        Ok(code) => code,

        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    let format = resolve_format(format);
    let output_path = output.unwrap_or_else(|| output_path_for(path, format));

    if let Err(error) = formats::write_executable(&code, &output_path, format) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!(
        "wrapped {} byte(s) of code into a {format:?} executable at {}",
        code.len(),
        output_path.display()
    );
}

/// Runs `bitterasm compile <path> -o <em_path>` as a subprocess (the same
/// sibling-binary lookup `delegate_to_bitterasm` uses), the way a real
/// `cc`-style driver shells out to a separate compiler pass rather than
/// re-implementing it — `bitter` already depends on `bitterasm` as a
/// library for `EmittedValue`'s own type, but resolving/expanding a
/// program is `bitterasm compile`'s job, not this crate's.
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

fn build(path: &PathBuf, output: Option<PathBuf>, format: Option<String>) {
    let format = resolve_format(format);
    let output_path = output.unwrap_or_else(|| output_path_for(path, format));

    let em_path = std::env::temp_dir().join(format!("bitter-build-{}.em", std::process::id()));
    run_bitterasm_compile(path, &em_path);

    let json = match std::fs::read_to_string(&em_path) {
        Ok(json) => json,

        Err(error) => {
            eprintln!("failed to read {}: {error}", em_path.display());
            std::process::exit(1);
        }
    };
    let _ = std::fs::remove_file(&em_path);

    let values: Vec<EmittedValue> = match serde_json::from_str(&json) {
        Ok(values) => values,

        Err(error) => {
            eprintln!("failed to parse {}: {error}", em_path.display());
            std::process::exit(1);
        }
    };

    let code = match pack::pack_stream(&values) {
        Ok(code) => code,

        Err(error) => {
            eprintln!("failed to encode {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    if let Err(error) = formats::write_executable(&code, &output_path, format) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!(
        "built {} byte(s) of code into a {format:?} executable at {}",
        code.len(),
        output_path.display()
    );
}
