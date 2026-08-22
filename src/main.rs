use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use bitterasm::ast::Statement;
use bitterasm::resolver::{SymbolTable, Value};
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

    /// Resolve and expand a .basm program, printing what it expanded to
    /// instead of writing a .em file — the emitted value stream `compile`
    /// would write, plus the generated declarations `compile` can only
    /// warn about and drop.
    Expand { path: PathBuf },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { path, output } => compile(&path, output),
        Command::Expand { path } => expand(&path),
    }
}

/// The shared front half of both `compile` and `expand`: load, validate,
/// resolve every struct/alias up front (so a broken declaration fails
/// either command even if unreached), then expand every top-level
/// invocation. Exits the process on the first error, same as both commands
/// already did before this was factored out.
struct Expansion {
    symbols: SymbolTable,
    emitted: Vec<Value>,
    generated: Vec<Statement>,
}

fn resolve_and_expand(path: &Path) -> Expansion {
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
    // the whole command, the same way a real compiler wouldn't skip type
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
    let mut generated = Vec::new();

    for statement in &program.statements {
        if let Statement::Invocation(invocation) = statement {
            match alias_resolver.expand_invocation(invocation, &HashMap::new()) {
                Ok(expansion) => {
                    emitted.extend(expansion.emitted);
                    generated.extend(expansion.generated);
                }

                Err(error) => {
                    eprintln!("resolver error: {error:?}");
                    std::process::exit(1);
                }
            }
        }
    }

    Expansion { symbols, emitted, generated }
}

fn compile(path: &Path, output: Option<PathBuf>) {
    let expansion = resolve_and_expand(path);

    if !expansion.generated.is_empty() {
        eprintln!(
            "warning: {count} declaration(s) were generated but not included in {path} — \
             there's no pass yet to splice them back into the program; run `bitterasm expand` \
             to see them",
            count = expansion.generated.len(),
            path = path.display(),
        );
    }

    let emitted: Vec<emit::EmittedValue> = expansion
        .emitted
        .iter()
        .map(|value| emit::reify_value(&expansion.symbols, value))
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

fn expand(path: &Path) {
    let expansion = resolve_and_expand(path);

    let emitted: Vec<emit::EmittedValue> = expansion
        .emitted
        .iter()
        .map(|value| emit::reify_value(&expansion.symbols, value))
        .collect();

    let json = match serde_json::to_string_pretty(&emitted) {
        Ok(json) => json,

        Err(error) => {
            eprintln!("serialization error: {error}");
            std::process::exit(1);
        }
    };

    println!("emitted values:\n{json}");
    println!("generated declarations:\n{:#?}", expansion.generated);
}
