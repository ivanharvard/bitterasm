use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod formats;
mod link;
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

        /// Byte offset into the code where execution starts (decimal, or
        /// hex with `0x`). A raw .bin has no labels to name, so this is a
        /// number here; `bitter build --entry` takes a label instead.
        /// Defaults to 0, the first byte.
        #[arg(short, long, value_parser = parse_offset)]
        entry: Option<usize>,
    },

    /// The whole pipeline in one command: compile one or more .basm
    /// programs, link them together, encode the result, and wrap it in a
    /// native executable — equivalent to `bitterasm compile` + `bitter
    /// encode` + `bitter exec` run in sequence, without the intermediate
    /// .em/.bin files. A single path behaves exactly as it always has;
    /// given more than one, same-named `section`s are concatenated across
    /// every input (command-line order), and a `pub` label imported
    /// (`from file import label`) in one input but declared in another is
    /// resolved against that other input's own compiled output — real
    /// multi-file linking (Phase 6, `docs/sections-and-linking/
    /// PROGRESS.md`). Execution starts at the `pub` label named by
    /// `--entry`, or at a `pub _start` label if any input declares one, or
    /// else at the very first byte of the linked output.
    Build {
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Defaults to the first path with its extension dropped (`.exe`
        /// added back for `--format pe`).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// `elf`, `pe`, or `macho`. Defaults to whatever this build of
        /// `bitter` is itself running on.
        #[arg(short, long)]
        format: Option<String>,

        /// The `pub` label execution starts at (like NASM's `global` plus
        /// a linker's `-e`). Must be `pub`: only `pub` labels are visible
        /// to the link step. Defaults to `_start` when some input declares
        /// `pub _start:`, otherwise the first byte of the output.
        #[arg(short, long)]
        entry: Option<String>,
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
        Command::Exec { path, output, format, entry } => exec(&path, output, format, entry.unwrap_or(0)),
        Command::Build { paths, output, format, entry } => build(&paths, output, format, entry),
        Command::External(args) => delegate_to_bitterasm(&args),
    }
}

/// `--entry` for `exec`: a decimal or `0x`-prefixed hex byte offset.
fn parse_offset(raw: &str) -> Result<usize, String> {
    let parsed = match raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        Some(hex) => usize::from_str_radix(hex, 16),
        None => raw.parse(),
    };
    parsed.map_err(|error| format!("`{raw}` isn't a byte offset: {error}"))
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

fn exec(path: &PathBuf, output: Option<PathBuf>, format: Option<String>, entry: usize) {
    let code = match std::fs::read(path) {
        Ok(code) => code,

        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    let format = resolve_format(format);
    let output_path = output.unwrap_or_else(|| output_path_for(path, format));

    if let Err(error) = formats::write_executable(&code, &output_path, format, entry) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!(
        "wrapped {} byte(s) of code into a {format:?} executable at {}",
        code.len(),
        output_path.display()
    );
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

fn build(paths: &[PathBuf], output: Option<PathBuf>, format: Option<String>, entry: Option<String>) {
    let format = resolve_format(format);
    let output_path = output.unwrap_or_else(|| output_path_for(&paths[0], format));

    let inputs: Vec<link::LinkInput> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| compile_input(path, &format!("{}-{index}", std::process::id())))
        .collect();

    let linked = match link::link(inputs) {
        Ok(linked) => linked,

        Err(error) => {
            eprintln!("failed to link: {error}");
            std::process::exit(1);
        }
    };
    let values = linked.values;

    let entry_index = match entry {
        Some(name) => *linked.labels.get(&name).unwrap_or_else(|| {
            eprintln!("no `pub` label named `{name}` to use as the entry point (entry labels must be `pub`)");
            std::process::exit(1);
        }),
        None => linked.labels.get("_start").copied().unwrap_or(0),
    };
    let entry_offset = pack::byte_offset_of(&values, entry_index).unwrap_or_else(|error| {
        eprintln!("failed to encode: {error}");
        std::process::exit(1);
    });

    let code = match pack::pack_stream(&values) {
        Ok(code) => code,

        Err(error) => {
            eprintln!("failed to encode: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = formats::write_executable(&code, &output_path, format, entry_offset) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!(
        "built {} byte(s) of code from {} input(s) into a {format:?} executable at {}",
        code.len(),
        paths.len(),
        output_path.display()
    );
}
