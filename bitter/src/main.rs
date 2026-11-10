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
    /// executable — `bitterasm compile` + `bitter encode` in one process,
    /// without the intermediate .em files. Given more than one path, same-named
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

        /// Print how long each phase took, peak memory, and the output's
        /// size to stderr.
        #[arg(long)]
        timings: bool,
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
        Command::Build { paths, output, timings } => build(&paths, output, timings),
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
    drop(json);

    // Laid out exactly as `bitter build` lays out one input: grouped by
    // section, with label positions translated to match. A cross-file
    // reference has no input to resolve against here and is an error.
    let input = link::LinkInput {
        file: path.clone(),
        module: file.module,
        entries: file.entries,
        labels: exports_as_positions(file.exports),
    };
    let (values, link_range) = match link::link(vec![input]) {
        Ok(linked) => (linked.values, linked.link),

        Err(error) => {
            eprintln!("failed to encode {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    let packed = match link_range {
        Some(range) => pack::pack_linked(&values, range),
        None => pack::pack_stream(&values),
    };
    let bytes = match packed {
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

/// Compiles one `bitter build` input in this process, with no `.em` file
/// in between: its entries, its module path (what other inputs'
/// `Deferred` references name it by) and its exported `pub` labels.
/// Diagnostics are printed as `bitterasm compile` would print them.
fn compile_input(path: &std::path::Path, timings: &mut bitterasm::compile::Timings) -> link::LinkInput {
    let options = bitterasm::compile::DiagnosticOptions::default();
    let expansion = bitterasm::compile::analyze(path, &options, None, timings)
        .unwrap_or_else(|bitterasm::compile::Failed| std::process::exit(1));
    let (module, exports, entries) = expansion.into_entries();
    timings.lap("convert");

    link::LinkInput { file: path.to_path_buf(), module, entries, labels: exports_as_positions(exports) }
}

fn build(paths: &[PathBuf], output: Option<PathBuf>, print_timings: bool) {
    let output_path = output.unwrap_or_else(|| paths[0].with_extension(""));
    let mut timings = bitterasm::compile::Timings::new();

    let inputs: Vec<link::LinkInput> = paths.iter().map(|path| compile_input(path, &mut timings)).collect();

    let (values, link_range) = match link::link(inputs) {
        Ok(linked) => (linked.values, linked.link),

        Err(error) => {
            eprintln!("failed to link: {error}");
            std::process::exit(1);
        }
    };

    timings.lap("link");

    let packed = match link_range {
        Some(range) => pack::pack_linked(&values, range),
        None => pack::pack_stream(&values),
    };
    let bytes = match packed {
        Ok(bytes) => bytes,

        Err(error) => {
            eprintln!("failed to encode: {error}");
            std::process::exit(1);
        }
    };
    timings.lap("pack");

    if let Err(error) = write_executable(&output_path, &bytes) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }
    timings.lap("write");

    if print_timings {
        timings.report();
        eprintln!("  {:<22} {:>10}", "values", values.len());
        eprintln!("  {:<22} {:>10}", "output bytes", bytes.len());
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
