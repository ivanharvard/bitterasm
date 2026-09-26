use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::io::IsTerminal;

use clap::{Args, Parser, Subcommand, ValueEnum};

use bitterasm::ast::Statement;
use bitterasm::expander::MacroTable;
use bitterasm::loader::ModuleOrigins;
use bitterasm::resolver::{SymbolTable, Value};
use bitterasm::verbose::VerboseReporter;
use bitterasm::{emit, eval, expander, formatter, lexer, loader, parser, resolver};
use bitterasm::diagnostics::{
    self, Diagnostic, DiagnosticFormat, LintConfig, LintLevel, LintName,
    RenderOptions, Severity, SourceId, SourceMap,
};

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliDiagnosticFormat { Terminal, Plain, Json }

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorChoice { Auto, Always, Never }

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

        /// Also write every top-level `pub` label's resolved position, as
        /// `{"name": "decimal position", ...}`, to this path. An advanced,
        /// opt-in escape hatch for `bitter build`'s multi-file linking
        /// (Phase 6, `docs/sections-and-linking/PROGRESS.md`) to resolve a
        /// `Deferred { file, symbol }` cross-unit reference (Phase 5)
        /// against this file's own compiled output — ordinary `bitterasm
        /// compile` use never needs this, and `.em`'s own shape is
        /// unaffected either way.
        #[arg(long)]
        labels: Option<PathBuf>,

        /// Print an animated, per-invocation status line to stderr as each
        /// top-level invocation is expanded, with its elapsed time and
        /// memory usage once it finishes.
        #[arg(long)]
        verbose: bool,

        #[command(flatten)]
        diagnostics: DiagnosticCliOptions,
    },

    /// Run all diagnostics and semantic checks without writing output.
    Check {
        path: PathBuf,

        #[command(flatten)]
        diagnostics: DiagnosticCliOptions,
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

        /// Print an animated, per-invocation status line to stderr as each
        /// top-level invocation is expanded, with its elapsed time and
        /// memory usage once it finishes.
        #[arg(long)]
        verbose: bool,
    },

    /// Format .basm files in place according to bitterasm.toml.
    #[command(alias = "fmt")]
    Format {
        /// Files or directories to format; defaults to the current directory.
        paths: Vec<PathBuf>,

        /// Check formatting without changing files.
        #[arg(long)]
        check: bool,

        /// Use this configuration instead of searching parent directories.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Compile { path, output, labels, verbose, diagnostics } => {
            compile(&path, output, labels, verbose, diagnostics)
        }

        Command::Check { path, diagnostics } => check(&path, diagnostics),

        Command::Expand { path, depth, lines, chars, output, verbose } => {
            expand(&path, depth, lines, chars, output, verbose)
        }

        Command::Format { paths, check, config } => format_files(paths, check, config),
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn check_accepts_compile_diagnostic_options() {
        let cli = Cli::try_parse_from([
            "bitterasm", "check", "program.basm", "--diagnostic-format", "json",
            "--color", "never", "-D", "unused",
        ]).unwrap();
        let Command::Check { path, diagnostics } = cli.command else {
            panic!("expected check command");
        };
        assert_eq!(path, PathBuf::from("program.basm"));
        assert!(matches!(diagnostics.diagnostic_format, CliDiagnosticFormat::Json));
        assert_eq!(diagnostics.deny, ["unused"]);
    }
}

fn format_files(mut paths: Vec<PathBuf>, check: bool, config_path: Option<PathBuf>) {
    if paths.is_empty() {
        paths.push(PathBuf::from("."));
    }

    let mut files = Vec::new();
    for path in paths {
        if let Err(error) = collect_basm_files(&path, &mut files) {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    files.sort();
    files.dedup();

    let mut unformatted = Vec::new();
    for path in files {
        let selected_config = config_path.clone().or_else(|| formatter::discover_config(&path));
        let config = match selected_config {
            Some(ref config_path) => match formatter::load_config(config_path) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            },
            None => formatter::FormatConfig::default(),
        };
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("failed to read {}: {error}", path.display());
                std::process::exit(1);
            }
        };
        let formatted = match formatter::format_source(&source, &config) {
            Ok(formatted) => formatted,
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                std::process::exit(1);
            }
        };

        if formatted != source {
            if check {
                unformatted.push(path);
            } else if let Err(error) = std::fs::write(&path, formatted) {
                eprintln!("failed to write {}: {error}", path.display());
                std::process::exit(1);
            }
        }
    }

    if !unformatted.is_empty() {
        for path in &unformatted {
            eprintln!("would reformat {}", path.display());
        }
        std::process::exit(1);
    }
}

fn collect_basm_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "basm") {
            files.push(path.to_path_buf());
            return Ok(());
        }
        return Err(format!("{} is not a .basm file", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("{} does not exist", path.display()));
    }

    let entries = std::fs::read_dir(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let child = entry.path();
        if child.is_dir() {
            let name = entry.file_name();
            if name != ".git" && name != "target" {
                collect_basm_files(&child, files)?;
            }
        } else if child.extension().is_some_and(|extension| extension == "basm") {
            files.push(child);
        }
    }
    Ok(())
}

/// The shared front half of both `compile` and `expand`: load, validate,
/// resolve every struct/alias up front (so a broken declaration fails
/// either command even if unreached), then expand every top-level
/// invocation. Exits the process on the first error, same as both commands
/// already did before this was factored out.
struct Expansion {
    symbols: SymbolTable,
    // Each loaded module's module path, indexed by the module ids `symbols`
    // records — what `.em` type ids are built from (`emit::TypeIds`).
    module_paths: Vec<String>,
    emitted: Vec<Value>,
    // Index-aligned with `emitted` — which section (`None` for the
    // implicit default) was active when each value was `@emit`'d. See
    // `resolver::AliasResolver::emitted_sections`.
    sections: Vec<Option<String>>,
    generated: Vec<Statement>,

    // Every top-level `pub` label's resolved position, keyed by name — see
    // `pub_label_positions`'s own doc. Only ever consumed by `compile`'s
    // `--labels` flag (Phase 6, `docs/sections-and-linking/PROGRESS.md`);
    // every other caller of `resolve_and_expand`/`analyze` ignores it.
    pub_label_positions: HashMap<String, eval::Int>,
}

/// Every top-level `pub` label's resolved position, keyed by name — the
/// manifest `bitter build`'s multi-file linking needs (Phase 6) to resolve
/// a `Deferred { file, symbol }` reference (Phase 5) against this file's
/// own emitted stream. `resolver` must already have walked every top-level
/// label (`AliasResolver::record_label_position`) by the time this runs —
/// true of both `resolve_and_expand`'s return points, the single-pass
/// short-circuit and the real pass after a two-pass fallback.
fn pub_label_positions(
    program: &bitterasm::ast::Program,
    symbols: &SymbolTable,
    label_positions: &HashMap<resolver::SymbolId, eval::Int>,
) -> HashMap<String, eval::Int> {
    program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Label(label) if label.is_pub => Some(&label.name),
            _ => None,
        })
        .filter_map(|name| {
            let id = symbols.lookup(name)?;
            let position = label_positions.get(&id)?;
            Some((name.clone(), position.clone()))
        })
        .collect()
}

enum CompileError {
    Load(loader::LoadError),
    Resolve(resolver::ResolveError),
}

impl From<loader::LoadError> for CompileError {
    fn from(error: loader::LoadError) -> Self { Self::Load(error) }
}

impl From<resolver::ResolveError> for CompileError {
    fn from(error: resolver::ResolveError) -> Self { Self::Resolve(error) }
}

/// Labels `--verbose`'s status lines for `compile`, which (unlike `expand`)
/// walks a loader-flattened, potentially multi-file program — a top-level
/// invocation's span means nothing until it's matched back to whichever
/// file's module id `origins` attributes it to. `sources`/`ids` are a
/// build-as-you-go cache: most invocations in a compilation come from the
/// same handful of modules (often just the entry file), so this reads a
/// given module's source off disk at most once no matter how many
/// invocations it's asked about.
struct VerboseCompileContext<'a> {
    reporter: &'a VerboseReporter,
    origins: &'a ModuleOrigins,
    cache: RefCell<(SourceMap, HashMap<usize, SourceId>)>,
}

impl<'a> VerboseCompileContext<'a> {
    fn new(reporter: &'a VerboseReporter, origins: &'a ModuleOrigins) -> Self {
        Self { reporter, origins, cache: RefCell::new((SourceMap::default(), HashMap::new())) }
    }

    fn start(&self, module: usize, span: bitterasm::token::Span, name: &str) {
        let mut cache = self.cache.borrow_mut();
        let (sources, ids) = &mut *cache;
        let path = self.origins.path(module);
        let source_id = *ids.entry(module).or_insert_with(|| {
            let text = std::fs::read_to_string(path).unwrap_or_default();
            sources.add(path, text)
        });
        let line = sources.get(source_id).map_or(0, |file| file.line_column(span.start).0);
        drop(cache);

        self.reporter.start(format!("{}:{line} {name}(...)", path.display()));
    }
}

fn resolve_and_expand(path: &Path, verbose: Option<&VerboseReporter>) -> Result<Expansion, CompileError> {
    let (program, origins) = loader::load_program_with_modules(path)?;
    let entry_module = origins.entry_module();
    let verbose_ctx = verbose.map(|reporter| VerboseCompileContext::new(reporter, &origins));

    // Unrolls every top-level `@for`/`@if` into concrete statements before
    // anything else (symbol collection included) ever sees them — see
    // `resolver::unroll_top_level`'s module doc. `statement_modules` is
    // `origins`'s per-statement attribution, reshuffled to match: whatever
    // a `@for`/`@if` wrapper unrolls into inherits its own module.
    let (program, statement_modules) = resolver::unroll_top_level(program, origins.all())?;
    resolver::validate_facets(&program)?;
    let symbols = resolver::collect_symbols(&program, &statement_modules)?;
    let consts = resolver::ConstEvaluator::new(&program, &symbols).evaluate_all()?;

    let consts_by_name: HashMap<String, eval::Int> = consts
        .iter()
        .map(|(id, value)| (symbols.get(*id).name.clone(), value.clone()))
        .collect();

    // A label reference can be a *forward* reference — a label whose
    // `foo:` line appears later in the program than the invocation that
    // names it — so a label's position can't always be known the first
    // time it's read. This first pass runs the real expansion machinery in
    // `LabelMode::Tolerant`, so an as-yet-unrecorded (forward-referenced)
    // label silently gets a placeholder position instead of erroring —
    // both to discover where every top-level label actually ends up, and,
    // for the common case where no forward reference ever actually occurs,
    // to serve as the final answer outright (see below).
    let mut discovery = resolver::AliasResolver::new(
        &program,
        &symbols,
        &consts_by_name,
        resolver::LabelMode::Tolerant,
        HashMap::new(),
        entry_module,
    );

    // Every struct/alias in the program is resolved up front, whether or
    // not any invocation actually reaches it — a broken declaration fails
    // the whole command, the same way a real compiler wouldn't skip type
    // checking an unreachable function.
    resolve_structs_and_aliases(&mut discovery)?;

    // Expand every top-level invocation (`mov r1, 7`, or a macro calling
    // another macro) in program order, against an empty scope — nothing at
    // the top level is a bound parameter.
    let mut emitted = Vec::new();
    let mut generated = Vec::new();
    walk_top_level(
        &program,
        &symbols,
        &mut discovery,
        Some((&mut emitted, &mut generated)),
        &statement_modules,
        verbose_ctx.as_ref(),
    )?;

    // A wrong placeholder only ever changes what gets emitted at some
    // point, never how *many* values get emitted (see
    // `resolver::values::AliasResolver::resolve_label_value`'s doc) — so if
    // no placeholder was ever actually substituted, nothing this pass
    // computed depended on an unknown label position in the first place,
    // and it's already the exact final answer. Most programs have no
    // labels at all (or none referenced before their own declaration), so
    // this skips a full second walk of the entire program for them.
    if !discovery.used_forward_label_placeholder() {
        let pub_labels = pub_label_positions(&program, &symbols, discovery.label_positions());
        let sections = discovery.take_emitted_sections();
        let symbols = discovery.into_symbols_with_generated();
        return Ok(Expansion {
            symbols,
            module_paths: origins.module_paths().to_vec(),
            emitted,
            sections,
            generated,
            pub_label_positions: pub_labels,
        });
    }

    // This pass's output turned out to be unusable after all (see below) —
    // drop it now rather than let `let`-shadowing keep it alive alongside
    // the real pass's own `emitted`/`generated` until the end of the
    // function, which would hold two full-program-sized copies in memory
    // at once for no reason.
    drop(emitted);
    drop(generated);

    // At least one label was read before its own position was known —
    // rerun the same expansion for real, in `LabelMode::Strict`, now that
    // every position is known from the pass above. This is only sound
    // because of the same "placeholder never changes *how many* values get
    // emitted" invariant, which requires that an `@if`/`@for` inside a
    // macro body never makes its own condition/range depend on a label's
    // position (both `@if`/`@for` exist now, but nothing checks this
    // restriction; it's on the author of an `@if`/`@for`-using macro to
    // not violate it, the same way today's language already trusts a
    // macro not to have an infinite `@for`). Top-level `@for`/`@if`
    // (`resolver::unroll_top_level`) doesn't have this problem at all — it
    // runs before either pass, over plain top-level consts only, with no
    // notion of labels yet.
    let label_positions = discovery.into_label_positions();

    // Reported once, outside the per-invocation lines above: this second
    // pass re-expands the exact same program, so repeating a full
    // "expanding ..." line per invocation for it too would just be the
    // first pass's output twice over.
    if verbose.is_some() {
        eprintln!("re-expanding: a forward-referenced label needs resolved positions");
    }

    let mut alias_resolver = resolver::AliasResolver::new(
        &program,
        &symbols,
        &consts_by_name,
        resolver::LabelMode::Strict,
        label_positions,
        entry_module,
    );

    resolve_structs_and_aliases(&mut alias_resolver)?;

    let mut emitted = Vec::new();
    let mut generated = Vec::new();

    walk_top_level(
        &program,
        &symbols,
        &mut alias_resolver,
        Some((&mut emitted, &mut generated)),
        &statement_modules,
        None,
    )?;

    let pub_labels = pub_label_positions(&program, &symbols, alias_resolver.label_positions());
    let sections = alias_resolver.take_emitted_sections();
    let symbols = alias_resolver.into_symbols_with_generated();
    Ok(Expansion {
        symbols,
        module_paths: origins.module_paths().to_vec(),
        emitted,
        sections,
        generated,
        pub_label_positions: pub_labels,
    })
}

/// Resolves every struct/alias/const in the program up front (independent
/// of label passes — none of struct/alias/const-value resolution touches
/// labels). Run identically on both pass resolvers rather than
/// sharing state across them, mirroring how `stack` (the macro-recursion
/// guard) is deliberately never shared across separate top-level
/// expansions either.
fn resolve_structs_and_aliases(alias_resolver: &mut resolver::AliasResolver) -> Result<(), resolver::ResolveError> {
    alias_resolver.resolve_all_structs()?;
    let aliases = alias_resolver.resolve_all()?;

    for ty in aliases.values() {
        // `strip_alias` here because this is field-resolvability
        // validation, unrelated to whether `ty` is a nominal (invariant-
        // bearing) alias — that struct's fields still need checking either
        // way.
        if let resolver::ResolvedType::Struct { symbol, args } = ty.strip_alias() {
            alias_resolver.instantiate_struct_fields(*symbol, args)?;
        }
    }

    // A broken declaration fails the whole command even if nothing reaches
    // it — already true of every struct/alias above; extends the same
    // guarantee to a const's *value*, previously only checked on demand
    // (see `resolver::values::AliasResolver::resolve_all_const_values`).
    alias_resolver.resolve_all_const_values()?;

    Ok(())
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
    statement_modules: &[usize],
    verbose: Option<&VerboseCompileContext>,
) -> Result<(), resolver::ResolveError> {
    for (index, statement) in program.statements.iter().enumerate() {
        match statement {
            Statement::Invocation(invocation) => {
                if let Some(ctx) = verbose {
                    ctx.start(statement_modules[index], invocation.span, &invocation.name);
                }

                let result = alias_resolver.expand_invocation(invocation, &HashMap::new());

                if let Some(ctx) = verbose {
                    match &result {
                        Ok(_) => ctx.reporter.finish_ok(),
                        Err(_) => ctx.reporter.finish_err(),
                    }
                }

                let expansion = result?;
                if let Some((emitted, generated)) = collect.as_mut() {
                    emitted.extend(expansion.emitted);
                    generated.extend(expansion.generated);
                }
            }

            Statement::Label(label) => {
                let id = symbols
                    .lookup(&label.name)
                    .expect("every top-level label is registered by collect_symbols");

                alias_resolver.record_label_position(id);
            }

            Statement::Section(section) => {
                alias_resolver.set_current_section(Some(section.name.clone()));
            }

            _ => {}
        }
    }
    Ok(())
}

#[derive(Debug, Args)]
struct DiagnosticCliOptions {
    #[arg(long, value_enum, default_value = "terminal")]
    diagnostic_format: CliDiagnosticFormat,

    #[arg(long, value_enum, default_value = "auto")]
    color: ColorChoice,

    /// Suppress a lint or lint group.
    #[arg(short = 'A', long = "allow", value_name = "LINT")]
    allow: Vec<String>,

    /// Emit a lint or lint group as warnings.
    #[arg(short = 'W', long = "warn", value_name = "LINT")]
    warn: Vec<String>,

    /// Promote a lint or lint group to errors.
    #[arg(short = 'D', long = "deny", value_name = "LINT")]
    deny: Vec<String>,

    /// Promote a lint to errors and prevent source-level lowering.
    #[arg(short = 'F', long = "forbid", value_name = "LINT")]
    forbid: Vec<String>,
}

struct DiagnosticRun {
    config: LintConfig,
    sources: SourceMap,
    source_id: SourceId,
    render: RenderOptions,
}

fn prepare_diagnostics(path: &Path, options: &DiagnosticCliOptions) -> Result<DiagnosticRun, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut sources = SourceMap::default();
    let source_id = sources.add(path, source);
    let mut config = match formatter::discover_config(path) {
        Some(config_path) => diagnostics::load_lint_config(&config_path)?,
        None => LintConfig::default(),
    };
    for (selectors, level) in [
        (&options.allow, LintLevel::Allow),
        (&options.warn, LintLevel::Warn),
        (&options.deny, LintLevel::Deny),
        (&options.forbid, LintLevel::Forbid),
    ] {
        for selector in selectors {
            config.set(selector, level)?;
        }
    }
    let format = match options.diagnostic_format {
        CliDiagnosticFormat::Terminal => DiagnosticFormat::Terminal,
        CliDiagnosticFormat::Plain => DiagnosticFormat::Plain,
        CliDiagnosticFormat::Json => DiagnosticFormat::Json,
    };
    let color = match options.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };
    Ok(DiagnosticRun { config, sources, source_id, render: RenderOptions { format, color } })
}

fn emit_diagnostics(diagnostics: &[Diagnostic], run: &DiagnosticRun) -> bool {
    if diagnostics.is_empty() { return false; }
    eprint!("{}", diagnostics::render(diagnostics, &run.sources, run.render));
    diagnostics.iter().any(|diagnostic| diagnostic.severity == Severity::Error)
}

fn analyze(path: &Path, options: DiagnosticCliOptions, verbose: Option<&VerboseReporter>) -> (Expansion, DiagnosticRun) {
    let mut diagnostic_run = match prepare_diagnostics(path, &options) {
        Ok(run) => run,
        Err(error) => {
            eprintln!("diagnostics configuration error: {error}");
            std::process::exit(1);
        }
    };

    // The loader supplies the entry module after seeding it with imported
    // generic signatures and syntax patterns but before flattening imports,
    // so every span here still belongs to the source file we render.
    match loader::load_entry_program(path) {
        Ok(program) => {
            let warnings = diagnostics::lint_program(
                &program,
                diagnostic_run.source_id,
                &diagnostic_run.config,
            );
            if emit_diagnostics(&warnings, &diagnostic_run) {
                std::process::exit(1);
            }
        }
        Err(error) => {
            let diagnostic = diagnostics::load_error(error, &mut diagnostic_run.sources);
            emit_diagnostics(&[diagnostic], &diagnostic_run);
            std::process::exit(1);
        }
    }

    if let Ok(sources) = loader::load_sources(path) {
        for (source_path, source) in sources {
            diagnostic_run.sources.add(source_path, source);
        }
    }

    let expansion = match resolve_and_expand(path, verbose) {
        Ok(expansion) => expansion,
        Err(CompileError::Load(error)) => {
            let diagnostic = diagnostics::load_error(error, &mut diagnostic_run.sources);
            emit_diagnostics(&[diagnostic], &diagnostic_run);
            std::process::exit(1);
        }
        Err(CompileError::Resolve(error)) => {
            let source = diagnostic_run.sources.locate_span(error.span(), error.source_needle());
            let diagnostic = diagnostics::resolve_error(error, source);
            emit_diagnostics(&[diagnostic], &diagnostic_run);
            std::process::exit(1);
        }
    };

    if !expansion.generated.is_empty() {
        let level = diagnostic_run.config.level(LintName::GENERATED_DECLARATIONS);
        if !matches!(level, LintLevel::Allow | LintLevel::Expect) {
            let mut diagnostic = Diagnostic::warning(
                LintName::GENERATED_DECLARATIONS,
                format!("{} declaration(s) were generated but not included in the output", expansion.generated.len()),
            )
            .primary(diagnostic_run.source_id, bitterasm::token::Span::new(0, 0), "generated from this compilation")
            .help("run `bitterasm expand` to inspect generated declarations");
            if matches!(level, LintLevel::Deny | LintLevel::Forbid) {
                diagnostic.severity = Severity::Error;
            }
            if emit_diagnostics(&[diagnostic], &diagnostic_run) {
                std::process::exit(1);
            }
        }
    }

    (expansion, diagnostic_run)
}

fn check(path: &Path, options: DiagnosticCliOptions) {
    let _ = analyze(path, options, None);
    println!("checked {}", path.display());
}

fn compile(
    path: &Path,
    output: Option<PathBuf>,
    labels: Option<PathBuf>,
    verbose: bool,
    options: DiagnosticCliOptions,
) {
    let reporter = verbose.then(VerboseReporter::new);
    let (expansion, _) = analyze(path, options, reporter.as_ref());

    let ids = emit::TypeIds::new(&expansion.symbols, &expansion.module_paths);
    let emitted: Vec<emit::EmittedEntry> = expansion
        .emitted
        .iter()
        .zip(&expansion.sections)
        .map(|(value, section)| emit::EmittedEntry {
            value: emit::reify_value(&ids, value),
            section: section.clone(),
        })
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

    if let Some(labels_path) = labels {
        let positions: std::collections::BTreeMap<&str, String> = expansion
            .pub_label_positions
            .iter()
            .map(|(name, position)| (name.as_str(), position.to_string()))
            .collect();

        let json = match serde_json::to_string_pretty(&positions) {
            Ok(json) => json,

            Err(error) => {
                eprintln!("serialization error: {error}");
                std::process::exit(1);
            }
        };

        if let Err(error) = std::fs::write(&labels_path, json) {
            eprintln!("failed to write {}: {error}", labels_path.display());
            std::process::exit(1);
        }
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
    verbose: bool,
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
            let mut sources = SourceMap::default();
            let source_id = sources.add(path, source.clone());
            eprint!("{}", diagnostics::render(
                &[diagnostics::lex_error(error, source_id)],
                &sources,
                RenderOptions { format: DiagnosticFormat::Terminal, color: std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() },
            ));
            std::process::exit(1);
        }
    };

    let program = match parser::parse(tokens) {
        Ok(program) => program,

        Err(error) => {
            let mut sources = SourceMap::default();
            let source_id = sources.add(path, source.clone());
            eprint!("{}", diagnostics::render(
                &[diagnostics::parse_error(error, source_id)],
                &sources,
                RenderOptions { format: DiagnosticFormat::Terminal, color: std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() },
            ));
            std::process::exit(1);
        }
    };

    // A separate, import-resolved parse, used only to look up a macro
    // invoked here but declared in an imported file — its own statements'
    // spans are never used, only its `Statement::Macro` declarations.
    let flattened = match loader::load_program(path) {
        Ok(flattened) => flattened,

        Err(error) => {
            let mut sources = SourceMap::default();
            sources.add(path, source.clone());
            let diagnostic = diagnostics::load_error(error, &mut sources);
            eprint!("{}", diagnostics::render(
                &[diagnostic],
                &sources,
                RenderOptions { format: DiagnosticFormat::Terminal, color: std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() },
            ));
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

    let reporter = verbose.then(VerboseReporter::new);
    let expanded = expander::expand_source(&source, &program, &table, depth, range, path, reporter.as_ref());

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
