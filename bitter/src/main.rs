use clap::Parser;

#[derive(Parser)]
#[command(name = "bitter", version, about = "Exporter CLI for BitterASM-emitted values")]
struct Cli;

fn main() {
    Cli::parse();
    println!("bitter doesn't do anything yet — @emit support isn't built.");
}
