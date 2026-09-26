use std::path::PathBuf;

use bitterasm::emit::EmittedValue;
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

/// Runs `bitterasm compile <path> -o <em_path> --labels <labels_path>` as a
/// subprocess (the same sibling-binary lookup `delegate_to_bitterasm`
/// uses), the way a real `cc`-style driver shells out to a separate
/// compiler pass rather than re-implementing it — `bitter` already depends
/// on `bitterasm` as a library for `EmittedValue`'s own type, but
/// resolving/expanding a program is `bitterasm compile`'s job, not this
/// crate's. `--labels` is the opt-in manifest (Phase 6, `docs/sections-
/// and-linking/PROGRESS.md`) `link::link` needs to resolve a `Deferred`
/// cross-unit reference (Phase 5) against `path`'s own `pub` labels.
fn run_bitterasm_compile(path: &std::path::Path, em_path: &std::path::Path, labels_path: &std::path::Path) {
    let bitterasm = bitterasm_path();

    let status = std::process::Command::new(&bitterasm)
        .args([
            "compile",
            &path.display().to_string(),
            "-o", &em_path.display().to_string(),
            "--labels", &labels_path.display().to_string(),
        ])
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

/// Compiles one `bitter build` input, then reads back both files
/// `run_bitterasm_compile` produced into a `link::LinkInput` — `file` is
/// `path`, canonicalized the same way `bitterasm`'s own loader
/// canonicalizes a `from file import label_name` target
/// (`ast::ExternLabel::file`), so a `Deferred` value's own `file` field
/// can be matched against it exactly.
fn compile_input(path: &std::path::Path, unique: &str) -> link::LinkInput {
    let em_path = std::env::temp_dir().join(format!("bitter-build-{unique}.em"));
    let labels_path = std::env::temp_dir().join(format!("bitter-build-{unique}.labels.json"));

    run_bitterasm_compile(path, &em_path, &labels_path);

    let em_json = std::fs::read_to_string(&em_path).unwrap_or_else(|error| {
        eprintln!("failed to read {}: {error}", em_path.display());
        std::process::exit(1);
    });
    let _ = std::fs::remove_file(&em_path);

    let entries: Vec<bitterasm::emit::EmittedEntry> = serde_json::from_str(&em_json).unwrap_or_else(|error| {
        eprintln!("failed to parse {}: {error}", em_path.display());
        std::process::exit(1);
    });

    let labels_json = std::fs::read_to_string(&labels_path).unwrap_or_else(|error| {
        eprintln!("failed to read {}: {error}", labels_path.display());
        std::process::exit(1);
    });
    let _ = std::fs::remove_file(&labels_path);

    let raw_labels: std::collections::HashMap<String, String> =
        serde_json::from_str(&labels_json).unwrap_or_else(|error| {
            eprintln!("failed to parse {}: {error}", labels_path.display());
            std::process::exit(1);
        });

    let labels: std::collections::HashMap<String, usize> = raw_labels
        .into_iter()
        .map(|(name, position)| {
            let position: usize = position.parse().unwrap_or_else(|error| {
                eprintln!("{}: `{name}`'s position `{position}` isn't a valid index: {error}", labels_path.display());
                std::process::exit(1);
            });
            (name, position)
        })
        .collect();

    let file = std::fs::canonicalize(path).unwrap_or_else(|error| {
        eprintln!("failed to resolve {}: {error}", path.display());
        std::process::exit(1);
    });

    let module = bitterasm::loader::module_path_of(&file);

    link::LinkInput { file, module, entries, labels }
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
