use clap::{Parser, Subcommand};

mod install;

#[derive(Parser)]
#[command(name = "bitter", version, about = "Exporter CLI for BitterASM-emitted values")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Interactively install the bitterasm language, the bitter CLI, and the stdlib
    Install,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Install => install::run(),
    }
}
