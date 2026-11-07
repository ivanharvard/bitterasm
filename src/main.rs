use std::collections::HashSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::io::IsTerminal;

use clap::{ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};

use bitterasm::expander::MacroTable;
use bitterasm::verbose::VerboseReporter;
use bitterasm::{compile, emit, expander, formatter, lexer, loader, parser};
use bitterasm::diagnostics::{self, DiagnosticFormat, LintLevel, RenderOptions, SourceMap};

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

        /// Print an animated, per-invocation status line to stderr as each
        /// top-level invocation is expanded, with its elapsed time and
        /// memory usage once it finishes.
        #[arg(long)]
        verbose: bool,

        /// Write the `.em` file indented, for reading. It's compact by
        /// default; indenting is one extra pass as it's written.
        #[arg(long)]
        pretty: bool,

        /// Print how long each phase took, peak memory, and the output's
        /// size to stderr.
        #[arg(long)]
        timings: bool,

        #[command(flatten)]
        diagnostics: DiagnosticCliOptions,
    },

    /// Run all diagnostics and semantic checks without writing output.
    Check {
        path: PathBuf,

        /// Print how long each phase took and peak memory to stderr.
        #[arg(long)]
        timings: bool,

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

    /// Generate Markdown reference pages from `##` and `#!` doc comments:
    /// one page per module, plus an `index.md` listing them.
    Doc {
        /// .basm files, or directories to search for them.
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// The directory to write the pages to.
        #[arg(short, long, default_value = "doc")]
        output: PathBuf,

        /// Write nothing; fail if the pages in `--output` (or the
        /// `--summary` list) are out of date.
        #[arg(long, conflicts_with = "test")]
        check: bool,

        /// An mdBook `SUMMARY.md` to list the pages in, between
        /// `<!-- bitterasm doc: begin -->` and `<!-- bitterasm doc: end -->`.
        #[arg(long, value_name = "SUMMARY.md", conflicts_with = "test")]
        summary: Option<PathBuf>,

        /// Compile the examples in doc comments instead of writing pages.
        #[arg(long)]
        test: bool,
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
    let matches = Cli::command().get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    if let (Some((_, command_matches)), Command::Compile { diagnostics, .. } | Command::Check { diagnostics, .. }) =
        (matches.subcommand(), &mut cli.command)
    {
        diagnostics.levels_in_order = lint_levels_in_order(command_matches);
    }

    match cli.command {
        Command::Compile { path, output, verbose, pretty, timings, diagnostics } => {
            compile(&path, output, CompileFlags { verbose, pretty, timings }, diagnostics)
        }

        Command::Check { path, timings, diagnostics } => check(&path, diagnostics, timings),

        Command::Expand { path, depth, lines, chars, output, verbose } => {
            expand(&path, depth, lines, chars, output, verbose)
        }

        Command::Doc { paths, output, check, summary, test } => {
            if test { test_docs(&paths) } else { write_docs(&paths, &output, check, summary.as_deref()) }
        }

        Command::Format { paths, check, config } => format_files(paths, check, config),
    }
}

fn doc_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    match bitterasm::doc::collect_files(paths) {
        Ok(files) if files.is_empty() => {
            eprintln!("no .basm files found");
            std::process::exit(1);
        }
        Ok(files) => files,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn write_docs(paths: &[PathBuf], output: &Path, check: bool, summary: Option<&Path>) {
    let fail = |message: String| -> ! {
        eprintln!("{message}");
        std::process::exit(1);
    };
    let reference = bitterasm::doc::generate(&doc_files(paths)).unwrap_or_else(|error| fail(error.to_string()));

    // The pages' links in `SUMMARY.md` go from its own directory to `output`.
    let summary_update = summary.map(|summary_path| {
        let current = std::fs::read_to_string(summary_path)
            .unwrap_or_else(|error| fail(format!("failed to read {}: {error}", summary_path.display())));
        let base = link_base(summary_path, output);
        let updated = bitterasm::doc::update_summary(&current, &reference, &base)
            .unwrap_or_else(|error| fail(format!("{}: {error}", summary_path.display())));
        (summary_path, current, updated)
    });

    if check {
        let mut stale: Vec<String> = reference
            .pages
            .iter()
            .filter(|page| std::fs::read_to_string(output.join(&page.path)).ok().as_ref() != Some(&page.markdown))
            .map(|page| output.join(&page.path).display().to_string())
            .collect();
        // A page for a module that's gone is out of date too.
        let expected: HashSet<PathBuf> = reference.pages.iter().map(|page| output.join(&page.path)).collect();
        let mut existing = Vec::new();
        markdown_files(output, &mut existing);
        stale.extend(existing.into_iter().filter(|path| !expected.contains(path)).map(|path| path.display().to_string()));
        if let Some((path, current, updated)) = &summary_update
            && current != updated
        {
            stale.push(path.display().to_string());
        }
        if !stale.is_empty() {
            stale.sort();
            for name in &stale {
                eprintln!("out of date: {name}");
            }
            fail("run `bitterasm doc` without `--check` to regenerate".to_string());
        }
        println!("{} page(s) in {} are up to date", reference.pages.len(), output.display());
        return;
    }

    for page in &reference.pages {
        let path = output.join(&page.path);
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&path, &page.markdown));
        if let Err(error) = written {
            fail(format!("failed to write {}: {error}", path.display()));
        }
    }
    // Pages of modules that no longer exist. They could as well be someone's
    // own notes, so they're only pointed out, never deleted.
    let expected: HashSet<PathBuf> = reference.pages.iter().map(|page| output.join(&page.path)).collect();
    let mut existing = Vec::new();
    markdown_files(output, &mut existing);
    for path in existing.into_iter().filter(|path| !expected.contains(path)) {
        eprintln!("warning: {} isn't a page of this reference; delete it if it's an old one", path.display());
    }
    if let Some((path, _, updated)) = summary_update {
        if let Err(error) = std::fs::write(path, updated) {
            fail(format!("failed to write {}: {error}", path.display()));
        }
    }
    println!("wrote {} page(s) to {}", reference.pages.len(), output.display());
}

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            out.push(path);
        }
    }
}

// `output` as a link prefix from the directory `summary` is in:
// `docs/book/src/SUMMARY.md` and `docs/book/src/std/reference` give
// `std/reference/`.
fn link_base(summary: &Path, output: &Path) -> String {
    let absolute = |path: &Path| {
        std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
    };
    let from = absolute(summary.parent().unwrap_or(Path::new(".")));
    let to = absolute(output);
    let shared = from.components().zip(to.components()).take_while(|(a, b)| a == b).count();
    let ups = from.components().count() - shared;
    let mut base = "../".repeat(ups);
    for component in to.components().skip(shared) {
        base.push_str(&component.as_os_str().to_string_lossy());
        base.push('/');
    }
    base
}

fn test_docs(paths: &[PathBuf]) {
    let examples = match bitterasm::doc::examples(&doc_files(paths)) {
        Ok(examples) => examples,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    let bitterasm = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("bitterasm"));
    let runner = bitterasm::doc::doctest::Runner {
        bitter: find_bitter(&bitterasm),
        bitterasm,
        cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    let mut checked = 0;
    let mut failures = Vec::new();
    for (index, module) in examples.iter().enumerate() {
        failures.extend(module.errors.iter().cloned());
        let scratch = std::env::temp_dir().join(format!("bitterasm-doc-{}-{index}", std::process::id()));
        let report = runner.run(&module.blocks, &scratch);
        checked += report.checked;
        failures.extend(report.failures);
    }

    for failure in &failures {
        eprintln!("error: {failure}\n");
    }
    println!("{} of {checked} doc example(s) passed", checked - failures.len().min(checked));
    if !failures.is_empty() {
        std::process::exit(1);
    }
}

// `bitter` ships alongside `bitterasm`; otherwise look on PATH.
fn find_bitter(bitterasm: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) { "bitter.exe" } else { "bitter" };
    let beside = bitterasm.parent().map(|dir| dir.join(name));
    beside
        .into_iter()
        .chain(std::env::var_os("PATH").iter().flat_map(std::env::split_paths).map(|dir| dir.join(name)))
        .find(|path| path.is_file())
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
        let Command::Check { path, diagnostics, .. } = cli.command else {
            panic!("expected check command");
        };
        assert_eq!(path, PathBuf::from("program.basm"));
        assert!(matches!(diagnostics.diagnostic_format, CliDiagnosticFormat::Json));
        assert_eq!(diagnostics.deny, ["unused"]);
    }

    #[test]
    fn lint_flags_apply_in_command_line_order() {
        let matches = Cli::command().get_matches_from([
            "bitterasm", "check", "program.basm", "-D", "all", "-A", "unused", "-W", "unreachable_code",
        ]);
        let (_, check) = matches.subcommand().unwrap();
        assert_eq!(
            lint_levels_in_order(check),
            [
                ("all".to_string(), LintLevel::Deny),
                ("unused".to_string(), LintLevel::Allow),
                ("unreachable_code".to_string(), LintLevel::Warn),
            ]
        );
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

    /// Every `-A`/`-W`/`-D`/`-F`, in command-line order, so a later flag
    /// overrides an earlier one (`-D all -A unused` allows `unused`). Filled
    /// in by `main` from clap's argument positions; empty when the options
    /// were built some other way.
    #[arg(skip)]
    levels_in_order: Vec<(String, LintLevel)>,
}

impl DiagnosticCliOptions {
    fn lint_levels(&self) -> Vec<(String, LintLevel)> {
        if !self.levels_in_order.is_empty() {
            return self.levels_in_order.clone();
        }
        [
            (&self.allow, LintLevel::Allow),
            (&self.warn, LintLevel::Warn),
            (&self.deny, LintLevel::Deny),
            (&self.forbid, LintLevel::Forbid),
        ]
        .into_iter()
        .flat_map(|(selectors, level)| selectors.iter().map(move |selector| (selector.clone(), level)))
        .collect()
    }
}

fn lint_levels_in_order(matches: &ArgMatches) -> Vec<(String, LintLevel)> {
    let mut levels = Vec::new();
    for (id, level) in [
        ("allow", LintLevel::Allow),
        ("warn", LintLevel::Warn),
        ("deny", LintLevel::Deny),
        ("forbid", LintLevel::Forbid),
    ] {
        let (Some(values), Some(indices)) = (matches.get_many::<String>(id), matches.indices_of(id)) else {
            continue;
        };
        levels.extend(indices.zip(values).map(|(index, value)| (index, value.clone(), level)));
    }
    levels.sort_by_key(|(index, ..)| *index);
    levels.into_iter().map(|(_, selector, level)| (selector, level)).collect()
}


impl DiagnosticCliOptions {
    fn to_options(&self) -> compile::DiagnosticOptions {
        compile::DiagnosticOptions {
            lint_levels: self.lint_levels(),
            format: match self.diagnostic_format {
                CliDiagnosticFormat::Terminal => DiagnosticFormat::Terminal,
                CliDiagnosticFormat::Plain => DiagnosticFormat::Plain,
                CliDiagnosticFormat::Json => DiagnosticFormat::Json,
            },
            color: match self.color {
                ColorChoice::Always => true,
                ColorChoice::Never => false,
                ColorChoice::Auto => compile::stderr_wants_color(),
            },
        }
    }
}

fn analyze_or_exit(
    path: &Path,
    options: &DiagnosticCliOptions,
    verbose: Option<&VerboseReporter>,
    timings: &mut compile::Timings,
) -> compile::Expansion {
    compile::analyze(path, &options.to_options(), verbose, timings).unwrap_or_else(|compile::Failed| {
        std::process::exit(1);
    })
}

fn check(path: &Path, options: DiagnosticCliOptions, timings: bool) {
    let mut recorded = compile::Timings::new();
    let _ = analyze_or_exit(path, &options, None, &mut recorded);
    if timings {
        recorded.report();
    }
    println!("checked {}", path.display());
}

struct CompileFlags {
    verbose: bool,
    pretty: bool,
    timings: bool,
}

fn compile(path: &Path, output: Option<PathBuf>, flags: CompileFlags, options: DiagnosticCliOptions) {
    let reporter = flags.verbose.then(VerboseReporter::new);
    let mut timings = compile::Timings::new();
    let expansion = analyze_or_exit(path, &options, reporter.as_ref(), &mut timings);
    drop(reporter);

    let output_path = output.unwrap_or_else(|| path.with_extension("em"));
    let fail = |error: std::io::Error| -> ! {
        eprintln!("failed to write {}: {error}", output_path.display());
        std::process::exit(1);
    };

    let file = std::fs::File::create(&output_path).unwrap_or_else(|error| fail(error));
    let written = if flags.pretty {
        let mut pretty = emit::PrettyJson::new(std::io::BufWriter::new(file));
        expansion.write_em(&mut pretty).and_then(|()| pretty.finish().map(drop))
    } else {
        expansion.write_em(file)
    };
    written.unwrap_or_else(|error| fail(error));
    timings.lap(if flags.pretty { "write (pretty)" } else { "write" });

    if flags.timings {
        timings.report();
        let size = std::fs::metadata(&output_path).map_or(0, |metadata| metadata.len());
        eprintln!("  {:<22} {:>10}", "values", expansion.emitted.len());
        eprintln!("  {:<22} {:>10}", ".em bytes", size);
    }

    println!(
        "compiled {} emitted value(s) to {}",
        expansion.emitted.len(),
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
