use std::path::PathBuf;

use bitterasm::emit::EmittedValue;
use clap::{Parser, Subcommand};

mod pack;

use pack::Endian;

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

        /// Byte order to serialize each emitted value in.
        #[arg(long, value_enum, default_value = "little")]
        endian: Endian,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Encode { path, output, endian } => encode(&path, output, endian),
    }
}

fn encode(path: &PathBuf, output: Option<PathBuf>, endian: Endian) {
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

    let bytes = match pack::pack_stream(&values, endian) {
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
