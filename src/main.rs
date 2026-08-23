use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use bitterasm::ast::Statement;
use bitterasm::expander::MacroTable;
use bitterasm::resolver::{SymbolTable, Value};
use bitterasm::{emit, eval, expander, lexer, loader, parser, resolver};

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

    /// Paste every macro invocation's body in place of its call, leaving
    /// everything else — `@emit`, `@return`, the rest of the file — exactly
    /// as written. Purely syntactic: nothing is evaluated or resolved, so
    /// this is closer to `cargo expand` than to `compile`.
    Expand {
        path: PathBuf,

        /// How many rounds of substitution to perform per invocation;
        /// omit for fully recursive expansion.
        #[arg(short, long)]
        depth: Option<usize>,

        /// Only expand invocations within this 1-indexed, inclusive line
        /// range (e.g. `10-15`); everything else is left untouched.
        #[arg(long, value_name = "START-END", conflicts_with = "chars")]
        lines: Option<String>,

        /// Only expand invocations within this byte-offset range (e.g.
        /// `120-180`); everything else is left untouched.
        #[arg(long, value_name = "START-END", conflicts_with = "lines")]
        chars: Option<String>,

        /// Defaults to printing to stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { path, output } => compile(&path, output),

        Command::Expand { path, depth, lines, chars, output } => {
            expand(&path, depth, lines, chars, output)
        }
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

    // A label reference (or `@here`) can be a *forward* reference — a
    // label whose `foo:` line appears later in the program than the
    // invocation that names it — so a label's position can't always be
    // known the first time it's read. Two passes over the same program
    // solve this: pass 1 runs the real expansion machinery in
    // `LabelMode::Tolerant`, so an as-yet-unrecorded (forward-referenced)
    // label silently gets a placeholder position instead of erroring, just
    // to discover where every top-level label actually ends up; pass 2
    // reruns the same expansion for real, in `LabelMode::Strict`, now that
    // every position is known. This is only sound because nothing in the
    // language can currently make the *number* of values a macro emits
    // depend on a value derived from `@here`/a label (no `@if`/`@for`
    // exist yet) — a wrong placeholder changes what gets emitted at some
    // points during pass 1, never how many statements execute, so pass 1's
    // discovered positions are still correct for pass 2 to trust.
    let mut discovery = resolver::AliasResolver::new(
        &program,
        &symbols,
        &consts_by_name,
        resolver::LabelMode::Tolerant,
        HashMap::new(),
    );

    resolve_structs_and_aliases(&mut discovery);
    walk_top_level(&program, &symbols, &mut discovery, None);

    let label_positions = discovery.into_label_positions();

    let mut alias_resolver = resolver::AliasResolver::new(
        &program,
        &symbols,
        &consts_by_name,
        resolver::LabelMode::Strict,
        label_positions,
    );

    // Every struct/alias in the program is resolved up front, whether or
    // not any invocation actually reaches it — a broken declaration fails
    // the whole command, the same way a real compiler wouldn't skip type
    // checking an unreachable function.
    resolve_structs_and_aliases(&mut alias_resolver);

    // Expand every top-level invocation (`mov r1, 7`, or a macro calling
    // another macro) in program order, against an empty scope — nothing at
    // the top level is a bound parameter.
    let mut emitted = Vec::new();
    let mut generated = Vec::new();

    walk_top_level(&program, &symbols, &mut alias_resolver, Some((&mut emitted, &mut generated)));

    Expansion { symbols, emitted, generated }
}

/// Resolves every struct/alias in the program up front (independent of
/// label passes — struct/alias resolution never touches `@here`/labels).
/// Run identically on both pass resolvers rather than sharing state across
/// them, mirroring how `stack` (the macro-recursion guard) is deliberately
/// never shared across separate top-level expansions either.
fn resolve_structs_and_aliases(alias_resolver: &mut resolver::AliasResolver) {
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
}

/// One left-to-right walk over the program's top-level statements, shared
/// by both label-resolution passes: expand every `Invocation`, and record
/// every top-level `Label`'s position as it's reached. `collect` is `None`
/// for the position-discovery pass (its emitted/generated output is
/// meaningless, only `alias_resolver`'s label positions matter) and
/// `Some(emitted, generated)` for the real pass.
fn walk_top_level(
    program: &bitterasm::ast::Program,
    symbols: &SymbolTable,
    alias_resolver: &mut resolver::AliasResolver,
    mut collect: Option<(&mut Vec<Value>, &mut Vec<Statement>)>,
) {
    for statement in &program.statements {
        match statement {
            Statement::Invocation(invocation) => {
                match alias_resolver.expand_invocation(invocation, &HashMap::new()) {
                    Ok(expansion) => {
                        if let Some((emitted, generated)) = collect.as_mut() {
                            emitted.extend(expansion.emitted);
                            generated.extend(expansion.generated);
                        }
                    }

                    Err(error) => {
                        eprintln!("resolver error: {error:?}");
                        std::process::exit(1);
                    }
                }
            }

            Statement::Label(label) => {
                let id = symbols
                    .lookup(&label.name)
                    .expect("every top-level label is registered by collect_symbols");

                alias_resolver.record_label_position(id);
            }

            _ => {}
        }
    }
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

fn expand(
    path: &Path,
    depth: Option<usize>,
    lines: Option<String>,
    chars: Option<String>,
    output: Option<PathBuf>,
) {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,

        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            std::process::exit(1);
        }
    };

    // Parsed standalone, not via `loader::load_program` — that flattens
    // every imported file into one `Program`, whose statements' spans
    // would then point into files other than `path`'s own source. This
    // program's spans need to mean something against `source` itself, so
    // it's this one file's own AST, imports and all, left unresolved.
    let tokens = match lexer::lex(&source) {
        Ok(tokens) => tokens,

        Err(error) => {
            eprintln!("lex error: {error}");
            std::process::exit(1);
        }
    };

    let program = match parser::parse(tokens) {
        Ok(program) => program,

        Err(error) => {
            eprintln!("parse error: {error:?}");
            std::process::exit(1);
        }
    };

    // A separate, import-resolved parse, used only to look up a macro
    // invoked here but declared in an imported file — its own statements'
    // spans are never used, only its `Statement::Macro` declarations.
    let flattened = match loader::load_program(path) {
        Ok(flattened) => flattened,

        Err(error) => {
            eprintln!("load error: {error}");
            std::process::exit(1);
        }
    };

    let table = MacroTable::from_program(&flattened);
    let depth = depth.unwrap_or(usize::MAX);

    let range = match (lines, chars) {
        (Some(lines), None) => match parse_range(&lines) {
            Ok((start, end)) => Some(line_range_to_bytes(&source, start, end)),

            Err(message) => {
                eprintln!("invalid --lines: {message}");
                std::process::exit(1);
            }
        },

        (None, Some(chars)) => match parse_range(&chars) {
            Ok((start, end)) => Some(start..end),

            Err(message) => {
                eprintln!("invalid --chars: {message}");
                std::process::exit(1);
            }
        },

        (None, None) => None,

        (Some(_), Some(_)) => unreachable!("clap's conflicts_with rules out --lines and --chars together"),
    };

    let expanded = expander::expand_source(&source, &program, &table, depth, range);

    match output {
        Some(output_path) => {
            if let Err(error) = std::fs::write(&output_path, expanded) {
                eprintln!("failed to write {}: {error}", output_path.display());
                std::process::exit(1);
            }
        }

        None => print!("{expanded}"),
    }
}

fn parse_range(spec: &str) -> Result<(usize, usize), String> {
    let (start, end) = spec
        .split_once('-')
        .ok_or_else(|| format!("expected `START-END`, found `{spec}`"))?;

    let start: usize = start
        .parse()
        .map_err(|_| format!("`{start}` isn't a valid start"))?;

    let end: usize = end.parse().map_err(|_| format!("`{end}` isn't a valid end"))?;

    if start > end {
        return Err(format!("start ({start}) is after end ({end})"));
    }

    Ok((start, end))
}

/// Converts a 1-indexed, inclusive line range into a byte range against
/// `source` — the start of `start_line` through the start of the line
/// after `end_line` (or end-of-file, if `end_line` is the last line).
fn line_range_to_bytes(source: &str, start_line: usize, end_line: usize) -> Range<usize> {
    let mut line_starts = vec![0usize];

    for (index, ch) in source.char_indices() {
        if ch == '\n' {
            line_starts.push(index + 1);
        }
    }

    line_starts.push(source.len());

    let last = line_starts.len() - 1;
    let start_index = start_line.saturating_sub(1).min(last);
    let end_index = end_line.min(last).max(start_index);

    line_starts[start_index]..line_starts[end_index]
}
