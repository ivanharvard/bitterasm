use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use bitterasm::ast::Statement;
use bitterasm::{emit, eval, loader, resolver};

#[derive(Parser)]
#[command(name = "bitterasm", version, about = "Compiler for BitterASM")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolve and expand a .basm program, writing its emitted value
    /// stream to a .em file for `bitter` to encode.
    Compile {
        path: PathBuf,

        /// Defaults to `path` with its extension swapped to `.em`.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { path, output } => compile(&path, output),
    }
}

fn compile(path: &Path, output: Option<PathBuf>) {
    let program = match loader::load_program(path) {
        Ok(program) => program,

        Err(error) => {
            eprintln!("load error: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = resolver::validate_facets(&program) {
        eprintln!("resolver error: {error:?}");
        std::process::exit(1);
    }

    let symbols = match resolver::collect_symbols(&program) {
        Ok(symbols) => symbols,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let consts = match resolver::ConstEvaluator::new(&program, &symbols).evaluate_all() {
        Ok(consts) => consts,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    let consts_by_name: HashMap<String, eval::Int> = consts
        .iter()
        .map(|(id, value)| (symbols.get(*id).name.clone(), value.clone()))
        .collect();

    let mut alias_resolver = resolver::AliasResolver::new(&program, &symbols, &consts_by_name);

    // Every struct/alias in the program is resolved up front, whether or
    // not any invocation actually reaches it — a broken declaration fails
    // the whole compile, the same way a real compiler wouldn't skip type
    // checking an unreachable function.
    if let Err(error) = alias_resolver.resolve_all_structs() {
        eprintln!("resolver error: {error:?}");
        std::process::exit(1);
    }

    let aliases = match alias_resolver.resolve_all() {
        Ok(aliases) => aliases,

        Err(error) => {
            eprintln!("resolver error: {error:?}");
            std::process::exit(1);
        }
    };

    for ty in aliases.values() {
        if let resolver::ResolvedType::Struct { symbol, args } = ty {
            if let Err(error) = alias_resolver.instantiate_struct_fields(*symbol, args) {
                eprintln!("resolver error: {error:?}");
                std::process::exit(1);
            }
        }
    }

    // Expand every top-level invocation (`mov r1, 7`, or a macro calling
    // another macro) in program order, against an empty scope — nothing at
    // the top level is a bound parameter.
    let mut emitted = Vec::new();
    let mut generated_count = 0usize;

    for statement in &program.statements {
        if let Statement::Invocation(invocation) = statement {
            match alias_resolver.expand_invocation(invocation, &HashMap::new()) {
                Ok(expansion) => {
                    generated_count += expansion.generated.len();
                    emitted.extend(expansion.emitted);
                }

                Err(error) => {
                    eprintln!("resolver error: {error:?}");
                    std::process::exit(1);
                }
            }
        }
    }

    if generated_count > 0 {
        eprintln!(
            "warning: {generated_count} declaration(s) were generated but not included in \
             {path} — there's no pass yet to splice them back into the program",
            path = path.display(),
        );
    }

    let emitted: Vec<emit::EmittedValue> = emitted
        .iter()
        .map(|value| emit::reify_value(&symbols, value))
        .collect();

    let json = match serde_json::to_string_pretty(&emitted) {
        Ok(json) => json,

        Err(error) => {
            eprintln!("serialization error: {error}");
            std::process::exit(1);
        }
    };

    let output_path = output.unwrap_or_else(|| path.with_extension("em"));

    if let Err(error) = std::fs::write(&output_path, json) {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    }

    println!(
        "compiled {} emitted value(s) to {}",
        emitted.len(),
        output_path.display()
    );
}
