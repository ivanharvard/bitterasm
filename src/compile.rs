//! The whole compile pipeline, from a `.basm` path to its emitted values:
//! lint, load, unroll, resolve every declaration, then expand every
//! top-level invocation. `bitterasm compile`/`check` and `bitter build` all
//! run it through [`analyze`], so `bitter build` needs no `.em` file in
//! between.
//!
//! Errors are rendered to stderr as they're found; [`analyze`] then
//! returns [`Failed`], and the caller decides how to exit.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::ast::{Program, Statement};
use crate::diagnostics::{
    self, Diagnostic, DiagnosticFormat, LintConfig, LintLevel, LintName, RenderOptions, Severity, SourceId,
    SourceMap,
};
use crate::emit::{self, EmittedEntry};
use crate::eval;
use crate::loader::{self, ModuleOrigins};
use crate::resolver::{self, Scope, SymbolTable, Value};
use crate::verbose::VerboseReporter;
use crate::{formatter, token};

/// How diagnostics are configured and rendered.
#[derive(Debug, Clone)]
pub struct DiagnosticOptions {
    /// Lint level overrides in command-line order, so a later one wins
    /// (`-D all -A unused` allows `unused`). They apply on top of any
    /// `bitterasm.toml` found above the compiled file.
    pub lint_levels: Vec<(String, LintLevel)>,
    pub format: DiagnosticFormat,
    pub color: bool,
}

impl Default for DiagnosticOptions {
    /// No overrides, rendered for a terminal, colored if stderr is one and
    /// `NO_COLOR` isn't set.
    fn default() -> Self {
        Self {
            lint_levels: Vec::new(),
            format: DiagnosticFormat::Terminal,
            color: stderr_wants_color(),
        }
    }
}

/// Whether stderr is a terminal and `NO_COLOR` isn't set.
pub fn stderr_wants_color() -> bool {
    std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

/// Compilation failed, and its diagnostics have already been printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failed;

/// How long each phase of a compilation took, in order. `--timings`
/// prints it.
#[derive(Debug)]
pub struct Timings {
    started: Instant,
    last: Instant,
    phases: Vec<(&'static str, Duration)>,
}

impl Default for Timings {
    fn default() -> Self {
        let now = Instant::now();
        Self { started: now, last: now, phases: Vec::new() }
    }
}

impl Timings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ends the phase running since the last call (or since this was
    /// created) and names it `phase`.
    pub fn lap(&mut self, phase: &'static str) {
        let now = Instant::now();
        self.phases.push((phase, now - self.last));
        self.last = now;
    }

    pub fn phases(&self) -> &[(&'static str, Duration)] {
        &self.phases
    }

    /// Writes one line per phase, then the total and peak memory, to
    /// stderr.
    pub fn report(&self) {
        let mut out = std::io::stderr().lock();
        let _ = writeln!(out, "timings:");
        for (phase, duration) in &self.phases {
            let _ = writeln!(out, "  {phase:<22} {:>10.3}s", duration.as_secs_f64());
        }
        let _ = writeln!(out, "  {:<22} {:>10.3}s", "total", self.started.elapsed().as_secs_f64());
        let _ = writeln!(out, "  {:<22} {:>9.1} MB", "peak memory", crate::verbose::current_memory_mb());
    }
}

/// A finished compilation: everything it emitted, plus what's needed to
/// write it out.
pub struct Expansion {
    pub symbols: SymbolTable,
    /// Each loaded module's module path, indexed by the module ids
    /// `symbols` records — what `.em` type ids are built from
    /// (`emit::TypeIds`).
    pub module_paths: Vec<String>,
    /// Which of those is the compiled file itself.
    pub entry_module: usize,
    pub emitted: Vec<Value>,
    /// Index-aligned with `emitted` — which section (`None` for the
    /// implicit default) was active when each value was `@emit`'d. See
    /// `resolver::AliasResolver::emitted_sections`.
    pub sections: Vec<Option<String>>,
    pub generated: Vec<Statement>,
    /// Every top-level `pub` label's resolved position, keyed by name — see
    /// `pub_label_positions`'s own doc. Written as the `.em` file's
    /// `exports`.
    pub pub_label_positions: HashMap<String, eval::Int>,
}

impl Expansion {
    /// The compiled file's own module path.
    pub fn module(&self) -> &str {
        &self.module_paths[self.entry_module]
    }

    /// `pub_label_positions` as `.em` exports: entry counts.
    pub fn exports(&self) -> BTreeMap<String, u64> {
        self.pub_label_positions
            .iter()
            .map(|(name, position)| {
                let position = u64::try_from(position).expect("a label position is a non-negative entry count");
                (name.clone(), position)
            })
            .collect()
    }

    /// Writes this as a `.em` file, one entry at a time.
    pub fn write_em<W: Write>(&self, out: W) -> std::io::Result<()> {
        let ids = emit::TypeIds::new(&self.symbols, &self.module_paths);
        emit::write_em(out, self.module(), &self.exports(), &ids, &self.emitted, &self.sections)
    }

    /// The emitted values as `.em` entries, freeing each value as it's
    /// converted.
    pub fn into_entries(self) -> (String, BTreeMap<String, u64>, Vec<EmittedEntry>) {
        let module = self.module().to_string();
        let exports = self.exports();
        let ids = emit::TypeIds::new(&self.symbols, &self.module_paths);
        let entries = self
            .emitted
            .into_iter()
            .zip(self.sections)
            .map(|(value, section)| EmittedEntry { value: emit::reify_value(&ids, &value), section })
            .collect();
        (module, exports, entries)
    }
}

/// Lints, loads, resolves and expands `path`, rendering any diagnostics to
/// stderr. `verbose` shows a status line per top-level invocation;
/// `timings` records each phase.
pub fn analyze(
    path: &Path,
    options: &DiagnosticOptions,
    verbose: Option<&VerboseReporter>,
    timings: &mut Timings,
) -> Result<Expansion, Failed> {
    let mut diagnostic_run = match prepare_diagnostics(path, options) {
        Ok(run) => run,
        Err(error) => {
            eprintln!("diagnostics configuration error: {error}");
            return Err(Failed);
        }
    };

    // The loader supplies the entry module after seeding it with imported
    // generic signatures and syntax patterns but before flattening imports,
    // so every span here still belongs to the source file we render.
    match loader::load_entry_program(path) {
        Ok(program) => {
            let warnings = diagnostics::lint_program_with(
                &program,
                diagnostic_run.source_id,
                &diagnostic_run.config,
                |import| loader::imports_modules(import, path),
            );
            if emit_diagnostics(&warnings, &diagnostic_run) {
                return Err(Failed);
            }
        }
        Err(error) => {
            let diagnostic = diagnostics::load_error(error, &mut diagnostic_run.sources);
            emit_diagnostics(&[diagnostic], &diagnostic_run);
            return Err(Failed);
        }
    }

    if let Ok(sources) = loader::load_sources(path) {
        for (source_path, source) in sources {
            diagnostic_run.sources.add(source_path, source);
        }
    }
    timings.lap("lint");

    let expansion = match resolve_and_expand(path, verbose, timings) {
        Ok(expansion) => expansion,
        Err(CompileError::Load(error)) => {
            let diagnostic = diagnostics::load_error(error, &mut diagnostic_run.sources);
            emit_diagnostics(&[diagnostic], &diagnostic_run);
            return Err(Failed);
        }
        Err(CompileError::Resolve(error, file)) => {
            let needle = error.source_needle().map(loader::demangle);
            let source = file
                .and_then(|file| diagnostic_run.sources.find_span_in(&file, error.span()))
                .or_else(|| diagnostic_run.sources.locate_span(error.span(), needle.as_deref()));
            let diagnostic = diagnostics::resolve_error(error, source);
            emit_diagnostics(&[diagnostic], &diagnostic_run);
            return Err(Failed);
        }
    };

    if !expansion.generated.is_empty() {
        let level = diagnostic_run.config.level(LintName::GENERATED_DECLARATIONS);
        if !matches!(level, LintLevel::Allow | LintLevel::Expect) {
            let mut diagnostic = Diagnostic::warning(
                LintName::GENERATED_DECLARATIONS,
                format!("{} declaration(s) were generated but not included in the output", expansion.generated.len()),
            )
            .primary(diagnostic_run.source_id, token::Span::new(0, 0), "generated from this compilation")
            .help("run `bitterasm expand` to inspect generated declarations");
            if matches!(level, LintLevel::Deny | LintLevel::Forbid) {
                diagnostic.severity = Severity::Error;
            }
            if emit_diagnostics(&[diagnostic], &diagnostic_run) {
                return Err(Failed);
            }
        }
    }

    Ok(expansion)
}

struct DiagnosticRun {
    config: LintConfig,
    sources: SourceMap,
    source_id: SourceId,
    render: RenderOptions,
}

fn prepare_diagnostics(path: &Path, options: &DiagnosticOptions) -> Result<DiagnosticRun, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut sources = SourceMap::default();
    let source_id = sources.add(path, source);
    let mut config = match formatter::discover_config(path) {
        Some(config_path) => diagnostics::load_lint_config(&config_path)?,
        None => LintConfig::default(),
    };
    for (selector, level) in &options.lint_levels {
        config.set(selector, *level)?;
    }
    Ok(DiagnosticRun {
        config,
        sources,
        source_id,
        render: RenderOptions { format: options.format, color: options.color },
    })
}

fn emit_diagnostics(diagnostics: &[Diagnostic], run: &DiagnosticRun) -> bool {
    if diagnostics.is_empty() {
        return false;
    }
    eprint!("{}", diagnostics::render(diagnostics, &run.sources, run.render));
    diagnostics.iter().any(|diagnostic| diagnostic.severity == Severity::Error)
}

/// Every top-level `pub` label's resolved position, keyed by name — the
/// manifest `bitter build`'s multi-file linking needs to resolve a
/// `Deferred { file, symbol }` reference against this file's own emitted
/// stream. `resolver` must already have walked every top-level label
/// (`AliasResolver::record_label_position`) by the time this runs — true of
/// both `resolve_and_expand`'s return points, the single-pass short-circuit
/// and the real pass after a two-pass fallback.
/// Reorders the emitted values so each section's are together, sections in
/// `order`: the layout every label position counts in. `bitter` groups
/// sections the same way, so this changes nothing it produces, but a
/// `label as int` now means the same thing to both.
fn group_by_section(
    emitted: Vec<Value>,
    sections: Vec<Option<String>>,
    order: &[Option<String>],
) -> (Vec<Value>, Vec<Option<String>>) {
    if order.len() <= 1 {
        return (emitted, sections);
    }
    let index: HashMap<&Option<String>, usize> = order.iter().enumerate().map(|(i, section)| (section, i)).collect();
    let mut buckets: Vec<Vec<Value>> = order.iter().map(|_| Vec::new()).collect();
    for (value, section) in emitted.into_iter().zip(&sections) {
        buckets[index[section]].push(value);
    }
    let sections = order
        .iter()
        .zip(&buckets)
        .flat_map(|(section, bucket)| std::iter::repeat_n(section.clone(), bucket.len()))
        .collect();
    (buckets.into_iter().flatten().collect(), sections)
}

fn pub_label_positions(
    program: &Program,
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
    /// The file the error's span is in, when it's known.
    Resolve(resolver::ResolveError, Option<PathBuf>),
}

impl From<loader::LoadError> for CompileError {
    fn from(error: loader::LoadError) -> Self {
        Self::Load(error)
    }
}

impl From<resolver::ResolveError> for CompileError {
    fn from(error: resolver::ResolveError) -> Self {
        Self::Resolve(error, None)
    }
}

/// Labels `--verbose`'s status lines for `compile`, which walks a
/// loader-flattened, potentially multi-file program — a top-level
/// invocation's span means nothing until it's matched back to whichever
/// file's module id `origins` attributes it to. `cache` is filled as it
/// goes: most invocations come from the same few modules, so this reads a
/// module's source off disk at most once.
struct VerboseCompileContext<'a> {
    reporter: &'a VerboseReporter,
    origins: &'a ModuleOrigins,
    cache: RefCell<(SourceMap, HashMap<usize, SourceId>)>,
}

impl<'a> VerboseCompileContext<'a> {
    fn new(reporter: &'a VerboseReporter, origins: &'a ModuleOrigins) -> Self {
        Self { reporter, origins, cache: RefCell::new((SourceMap::default(), HashMap::new())) }
    }

    fn start(&self, module: usize, span: token::Span, name: &str) {
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

fn resolve_and_expand(
    path: &Path,
    verbose: Option<&VerboseReporter>,
    timings: &mut Timings,
) -> Result<Expansion, CompileError> {
    let (program, origins) = loader::load_program_with_modules(path)?;
    let entry_module = origins.entry_module();
    let verbose_ctx = verbose.map(|reporter| VerboseCompileContext::new(reporter, &origins));
    timings.lap("load");

    // Unrolls every top-level `@for`/`@if` into concrete statements before
    // anything else (symbol collection included) ever sees them — see
    // `resolver::unroll_top_level`'s module doc. `statement_modules` is
    // `origins`'s per-statement attribution, reshuffled to match: whatever
    // a `@for`/`@if` wrapper unrolls into inherits its own module.
    let (program, statement_modules) = resolver::unroll_top_level(program, origins.all())?;
    timings.lap("unroll");

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
    )
    .with_module_scopes(origins.scopes().to_vec());

    // Every struct/alias in the program is resolved up front, whether or
    // not any invocation actually reaches it — a broken declaration fails
    // the whole command, the same way a real compiler wouldn't skip type
    // checking an unreachable function.
    resolve_structs_and_aliases(&mut discovery).map_err(|error| {
        CompileError::Resolve(error, discovery.take_error_module().map(|module| origins.path(module).to_path_buf()))
    })?;
    timings.lap("declarations");

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
        &consts_by_name,
        verbose_ctx.as_ref(),
    )
    .map_err(|(error, module)| CompileError::Resolve(error, Some(origins.path(module).to_path_buf())))?;
    timings.lap("expand");

    // A wrong placeholder only ever changes what gets emitted at some
    // point, never how *many* values get emitted (see
    // `resolver::values::AliasResolver::resolve_label_value`'s doc) — so if
    // no placeholder was ever actually substituted, nothing this pass
    // computed depended on an unknown label position in the first place,
    // and it's already the exact final answer. Most programs have no
    // labels at all (or none referenced before their own declaration), so
    // this skips a full second walk of the entire program for them.
    if !discovery.needs_label_pass() {
        let pub_labels = pub_label_positions(&program, &symbols, &discovery.grouped_label_positions());
        let sections = discovery.take_emitted_sections();
        let (emitted, sections) = group_by_section(emitted, sections, discovery.section_order());
        let symbols = discovery.into_symbols_with_generated();
        return Ok(Expansion {
            symbols,
            module_paths: origins.module_paths().to_vec(),
            entry_module,
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
    let label_positions = discovery.grouped_label_positions();
    let label_spaces = discovery.label_spaces().clone();
    drop(discovery);

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
    )
    .with_module_scopes(origins.scopes().to_vec())
    .with_label_spaces(label_spaces);

    resolve_structs_and_aliases(&mut alias_resolver).map_err(|error| {
        CompileError::Resolve(error, alias_resolver.take_error_module().map(|module| origins.path(module).to_path_buf()))
    })?;

    let mut emitted = Vec::new();
    let mut generated = Vec::new();

    walk_top_level(
        &program,
        &symbols,
        &mut alias_resolver,
        Some((&mut emitted, &mut generated)),
        &statement_modules,
        &consts_by_name,
        None,
    )
    .map_err(|(error, module)| CompileError::Resolve(error, Some(origins.path(module).to_path_buf())))?;
    timings.lap("expand (label pass)");

    let pub_labels = pub_label_positions(&program, &symbols, alias_resolver.label_positions());
    let sections = alias_resolver.take_emitted_sections();
    let (emitted, sections) = group_by_section(emitted, sections, alias_resolver.section_order());
    let symbols = alias_resolver.into_symbols_with_generated();
    Ok(Expansion {
        symbols,
        module_paths: origins.module_paths().to_vec(),
        entry_module,
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
    program: &Program,
    symbols: &SymbolTable,
    alias_resolver: &mut resolver::AliasResolver,
    mut collect: Option<(&mut Vec<Value>, &mut Vec<Statement>)>,
    statement_modules: &[usize],
    consts: &HashMap<String, eval::Int>,
    verbose: Option<&VerboseCompileContext>,
) -> Result<(), (resolver::ResolveError, usize)> {
    for (index, statement) in program.statements.iter().enumerate() {
        match statement {
            Statement::Invocation(invocation) => {
                if let Some(ctx) = verbose {
                    ctx.start(statement_modules[index], invocation.span, &invocation.name);
                }

                let result = alias_resolver.expand_invocation(invocation, &Scope::default());

                if let Some(ctx) = verbose {
                    match &result {
                        Ok(_) => ctx.reporter.finish_ok(),
                        Err(_) => ctx.reporter.finish_err(),
                    }
                }

                // The module the error arose in: the innermost failing
                // macro's, or else this statement's own.
                let expansion = result.map_err(|error| {
                    let module = alias_resolver.take_error_module().unwrap_or(statement_modules[index]);
                    (error, module)
                })?;
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
                if section.declares() {
                    alias_resolver.declare_section(section, &Scope::default(), statement_modules[index]).map_err(|error| {
                        let module = alias_resolver.take_error_module().unwrap_or(statement_modules[index]);
                        (error, module)
                    })?;
                }
                // Another file's `pub section`: declared, not switched to.
                if !section.imported {
                    alias_resolver.set_current_section(Some(section.name.clone()));
                }
            }

            // A top-level `@for` of invocations, kept as a loop by
            // `resolver::unroll_top_level`.
            Statement::Meta(meta) => {
                let mut each = |resolver: &mut resolver::AliasResolver, invocation: &crate::ast::Invocation, scope: &Scope| {
                    if let Some(ctx) = verbose {
                        ctx.start(statement_modules[index], invocation.span, &invocation.name);
                    }
                    let result = resolver.expand_invocation(invocation, scope);
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
                    Ok(())
                };
                alias_resolver.expand_top_level_loop(meta, consts, &mut each).map_err(|error| {
                    let module = alias_resolver.take_error_module().unwrap_or(statement_modules[index]);
                    (error, module)
                })?;
            }

            _ => {}
        }
    }
    Ok(())
}
