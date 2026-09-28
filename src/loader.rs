// Resolves `from ... import ...` statements to files on disk and flattens
// them into a single, self-contained `Program` before the resolver ever
// sees it. The resolver and symbol table stay import-agnostic: by the time
// they run, every declaration a file depends on already lives directly in
// its statement list, and every name has been rewritten to what it means
// in the file that wrote it (see "namespaces" below).
//
// Module paths map onto the filesystem 1:1: `std.binary.native` is
// `<root>/std/binary/native.basm`. For relative imports (leading dots)
// `<root>` is the importing file's own directory, ascended once per extra
// leading dot. For absolute imports it is the first of `search_roots()` that
// holds the file: the current working directory, then each `BITTERASM_PATH`
// entry, then the install root `~/.bitterasm` (where `install.sh` puts `std/`).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    literal_name, ConstructItem, Doc, Expr, ExternLabel, Facet, FacetPayload, ImportItems,
    ImportStatement, MetaStatement, ModulePath, NamePart, Program, Statement,
};
use crate::eval::Int;
use crate::lexer;
use crate::parser::{self, ParserSeed};
use crate::resolver::{self, ResolveError};
use crate::token::{DocComment, Span};
use crate::types::{GenericParameter, StructBodyItem, TypeArgument, TypeExpr};

#[derive(Debug)]
pub enum LoadError {
    Io {
        path: PathBuf,
        message: String,
    },
    Lex {
        path: PathBuf,
        message: String,
        span: Span,
    },
    Parse {
        path: PathBuf,
        message: String,
        span: Span,
    },
    ModuleNotFound {
        importer: PathBuf,
        module: String,
        /// Every root directory the module was looked for under, in order.
        searched: Vec<PathBuf>,
    },
    CyclicImport {
        cycle: Vec<PathBuf>,
    },
    UnknownImportedName {
        module: String,
        name: String,
    },
    /// A top-level `@for`/`@if`/`@fold` in `path` couldn't be unrolled, or
    /// `path` declares one name twice.
    Resolve {
        path: PathBuf,
        error: Box<ResolveError>,
    },
    /// `importer` uses `name`, which more than one of its imports brings
    /// in, each meaning something different.
    AmbiguousName {
        importer: PathBuf,
        name: String,
        modules: Vec<String>,
    },
    /// Two of `importer`'s own imports each assign `name` a different
    /// `syntax` override, and `importer` doesn't locally declare its own
    /// override for `name` to say which one it means — see
    /// `parser::ParserSeed::syntax_overrides`'s doc.
    ConflictingSyntaxOverride {
        importer: PathBuf,
        name: String,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io { path, message } => {
                write!(f, "failed to read {}: {message}", path.display())
            }

            LoadError::Lex { path, message, .. } => {
                write!(f, "{}: {message}", path.display())
            }

            LoadError::Parse { path, message, .. } => {
                write!(f, "{}: {message}", path.display())
            }

            LoadError::ModuleNotFound { importer, module, searched } => {
                write!(
                    f,
                    "{}: could not find module `{module}`",
                    importer.display(),
                )?;

                if !searched.is_empty() {
                    let roots: Vec<String> = searched
                        .iter()
                        .map(|root| root.display().to_string())
                        .collect();

                    write!(f, " (searched: {})", roots.join(", "))?;
                }

                Ok(())
            }

            LoadError::CyclicImport { cycle } => {
                let names: Vec<String> = cycle
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect();

                write!(f, "cyclic import: {}", names.join(" -> "))
            }

            LoadError::UnknownImportedName { module, name } => {
                write!(f, "module `{module}` has no `{name}`")
            }

            LoadError::Resolve { path, error } => {
                write!(f, "{}: {error:?}", path.display())
            }

            LoadError::AmbiguousName { importer, name, modules } => {
                let modules: Vec<String> = modules.iter().map(|module| format!("`{module}`")).collect();
                write!(
                    f,
                    "{}: `{name}` is imported from more than one module ({}) — \
                     import it by name from the one you mean",
                    importer.display(),
                    modules.join(", "),
                )
            }

            LoadError::ConflictingSyntaxOverride { importer, name } => {
                write!(
                    f,
                    "{}: multiple imports assign different syntax to `{name}` — \
                     add your own `syntax {name}(...) = {{ ... }}` here to pick one",
                    importer.display(),
                )
            }
        }
    }
}

impl std::error::Error for LoadError {}

// A fully loaded module, kept around by canonical path so a module imported
// from multiple places is only read, parsed and named once.
struct LoadedModule {
    // The file's statements exactly as parsed: the source-faithful view
    // `load_entry_program` returns.
    source_statements: Vec<Statement>,

    // The same statements with top-level `@for`/`@if`/`@fold` unrolled, but
    // not yet renamed: what `load_unrolled_entry_program` returns.
    unrolled: Vec<Statement>,

    // The file's `#!` block, and the doc comments that document nothing.
    doc: Option<Doc>,
    stray_docs: Vec<DocComment>,

    // The same statements with top-level `@for`/`@if`/`@fold` unrolled and
    // every declaration and reference given its internal name (see
    // "namespaces" below). This is what gets spliced into a program.
    statements: Vec<Statement>,

    // The `pub` labels this file imports by name, as already-named extern
    // label declarations, each with the id of the module that declares it.
    extern_labels: Vec<(ExternLabel, usize)>,

    span: Span,

    // Generic signatures and macro syntax patterns for the names this
    // module exports. Handed to files that import it, so they can parse
    // `bits<width>`-shaped usages, and custom-syntax macro invocations,
    // without the declaration itself being textually present.
    seed: ParserSeed,

    // What each name means inside this file, and the part of that an
    // importer sees: its `pub` declarations plus its `pub from` imports.
    scope: HashMap<String, Binding>,
    exports: HashMap<String, Binding>,

    // Stable, unique-per-module id assigned the first time this module is
    // loaded. Internal names carry it (see "namespaces" below).
    module_id: usize,
}

#[derive(Default)]
struct LoadState {
    cache: HashMap<PathBuf, LoadedModule>,
    stack: Vec<PathBuf>,

    // Every loaded module's top-level integer constants, by internal name:
    // what a later module's top-level `@for` bounds can read through its
    // imports.
    const_values: HashMap<String, Int>,
}

pub fn load_program(entry: &Path) -> Result<Program, LoadError> {
    Ok(load_program_with_modules(entry)?.0)
}

/// A loaded program's statements, one-to-one with a "which file declared
/// this" tag per statement — [`ModuleOrigins::path`] turns that tag back
/// into a path for diagnostics. Every statement in the flattened program
/// has a tag: the entry file's own top-level statements get its module id
/// (whatever [`load_module`] happened to assign it, not necessarily 0 — see
/// `load_module`'s own doc), and everything spliced in from an import
/// carries the module id of whichever file actually declared it, however
/// many other files it was spliced through to get there.
#[derive(Debug, Clone, Default)]
pub struct ModuleOrigins {
    module_of_statement: Vec<usize>,
    paths: Vec<PathBuf>,
    module_paths: Vec<String>,
    scopes: Vec<HashMap<String, String>>,
    entry_module: usize,
}

impl ModuleOrigins {
    pub fn module_of(&self, statement_index: usize) -> usize {
        self.module_of_statement[statement_index]
    }

    /// One entry per statement in the `Program` this came from, in the
    /// same order — the shape [`crate::resolver::unroll_top_level`] takes.
    pub fn all(&self) -> &[usize] {
        &self.module_of_statement
    }

    pub fn path(&self, module: usize) -> &Path {
        &self.paths[module]
    }

    /// Every module's module path (see [`module_path_of`]), indexed by
    /// module id.
    pub fn module_paths(&self) -> &[String] {
        &self.module_paths
    }

    /// Every module's names, each mapped to the internal name the loader
    /// rewrote references to it into, indexed by module id. The resolver
    /// reads it for a name it builds itself (`` x`i` ``), which the loader
    /// never saw.
    pub fn scopes(&self) -> &[HashMap<String, String>] {
        &self.scopes
    }

    /// The module id of the file originally passed to
    /// [`load_program_with_modules`] — not necessarily 0; see
    /// [`load_module`]'s own doc for why discovery order doesn't guarantee
    /// that.
    pub fn entry_module(&self) -> usize {
        self.entry_module
    }
}

/// Like [`load_program`], but also returns, for each of the returned
/// `Program`'s top-level statements, which file originally declared it —
/// needed to enforce a non-`pub` struct field's visibility against the
/// module that's actually trying to read it, since that check has to
/// survive the same flattening that makes namespaces need internal names
/// in the first place (see "namespaces" below). Ordinary callers that don't
/// care about module attribution (formatting, `expand`, every existing
/// test) should keep using [`load_program`].
pub fn load_program_with_modules(entry: &Path) -> Result<(Program, ModuleOrigins), LoadError> {
    let entry_path = canonicalize(entry)?;

    let mut state = LoadState::default();
    load_module(&entry_path, &mut state)?;
    let cache = &state.cache;

    // The entry file's own imports get spliced into its statement list; it
    // is never itself spliced into anything, so seed the "already included"
    // set with just itself.
    let mut spliced: HashSet<PathBuf> = HashSet::new();
    spliced.insert(entry_path.clone());

    let entry_module = &cache[&entry_path];
    let span = entry_module.span;
    let entry_module_id = entry_module.module_id;

    let mut statements = Vec::new();
    let mut module_of_statement = Vec::new();

    for statement in &entry_module.statements {
        match statement {
            Statement::Import(import) => {
                for target in import_targets(import, &entry_path)? {
                    collect_declarations(&target, cache, &mut spliced, &mut statements, &mut module_of_statement)?;
                }
            }

            other => {
                statements.push(other.clone());
                module_of_statement.push(entry_module_id);
            }
        }
    }

    for (label, module_id) in &entry_module.extern_labels {
        statements.push(Statement::ExternLabel(label.clone()));
        module_of_statement.push(*module_id);
    }

    // Several modules may import the same label; it's one declaration.
    let mut seen_labels: HashSet<String> = HashSet::new();
    let (statements, module_of_statement): (Vec<_>, Vec<_>) = statements
        .into_iter()
        .zip(module_of_statement)
        .filter(|(statement, _)| match statement {
            Statement::ExternLabel(label) => seen_labels.insert(label.name.clone()),
            _ => true,
        })
        .unzip();

    let mut paths: Vec<PathBuf> = vec![PathBuf::new(); cache.len()];
    let mut scopes: Vec<HashMap<String, String>> = vec![HashMap::new(); cache.len()];
    for (path, module) in cache {
        paths[module.module_id] = path.clone();
        scopes[module.module_id] = module
            .scope
            .iter()
            .filter_map(|(name, binding)| Some((name.clone(), binding.internal_name()?)))
            .collect();
    }

    let module_paths = paths.iter().map(|path| module_path_of(path)).collect();

    Ok((
        Program {
            statements,
            span,
            doc: entry_module.doc.clone(),
            stray_docs: entry_module.stray_docs.clone(),
        },
        ModuleOrigins { module_of_statement, paths, module_paths, scopes, entry_module: entry_module_id },
    ))
}

/// Loads and parses the entry module with all imported parser signatures and
/// custom syntax available, but does not splice imported declarations into
/// its statement list. This is the source-faithful view used by file-local
/// diagnostics: every span belongs to `entry`, while ordinary compilation
/// continues to use [`load_program`]'s flattened program.
pub fn load_entry_program(entry: &Path) -> Result<Program, LoadError> {
    let entry_path = canonicalize(entry)?;
    let mut state = LoadState::default();
    load_module(&entry_path, &mut state)?;
    let entry_module = &state.cache[&entry_path];
    Ok(Program {
        statements: entry_module.source_statements.clone(),
        span: entry_module.span,
        doc: entry_module.doc.clone(),
        stray_docs: entry_module.stray_docs.clone(),
    })
}

/// [`load_entry_program`] with the entry's top-level `@for`/`@if`/`@fold`
/// already unrolled, so a declaration it generates (`pub const r`i``) has
/// its literal name (`r0`, `r1`, ...). Names aren't made internal, and every
/// span still belongs to `entry`: the view `bitterasm doc` documents.
pub fn load_unrolled_entry_program(entry: &Path) -> Result<Program, LoadError> {
    let entry_path = canonicalize(entry)?;
    let mut state = LoadState::default();
    load_module(&entry_path, &mut state)?;
    let entry_module = &state.cache[&entry_path];
    Ok(Program {
        statements: entry_module.unrolled.clone(),
        span: entry_module.span,
        doc: entry_module.doc.clone(),
        stray_docs: entry_module.stray_docs.clone(),
    })
}

/// Returns every source file in an entry's import graph. The order is
/// deterministic by canonical path and is intended for diagnostics, whose
/// resolver spans must be matched back to the module they came from.
pub fn load_sources(entry: &Path) -> Result<Vec<(PathBuf, String)>, LoadError> {
    let entry_path = canonicalize(entry)?;
    let mut state = LoadState::default();
    load_module(&entry_path, &mut state)?;
    let mut paths: Vec<_> = state.cache.into_keys().collect();
    paths.sort();
    paths.into_iter().map(|path| {
        let source = fs::read_to_string(&path).map_err(|error| LoadError::Io {
            path: path.clone(), message: error.to_string(),
        })?;
        Ok((path, source))
    }).collect()
}

fn load_module(path: &Path, state: &mut LoadState) -> Result<(), LoadError> {
    if state.cache.contains_key(path) {
        return Ok(());
    }

    if let Some(start) = state.stack.iter().position(|p| p == path) {
        let mut cycle: Vec<PathBuf> = state.stack[start..].to_vec();
        cycle.push(path.to_path_buf());

        return Err(LoadError::CyclicImport { cycle });
    }

    let source = fs::read_to_string(path).map_err(|error| LoadError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let (tokens, docs) = lexer::lex_with_docs(&source).map_err(|error| LoadError::Lex {
        path: path.to_path_buf(),
        message: error.message,
        span: error.span,
    })?;

    let imports = parser::discover_imports(tokens.clone());

    state.stack.push(path.to_path_buf());

    let mut seed = ParserSeed::default();

    // A name with more than one *distinct* syntax override arriving from
    // this file's own imports — not yet an error, since a local
    // declaration for that name (checked against `local_syntax_overrides`
    // once parsing finishes, below) is exactly how a developer resolves
    // exactly this ambiguity. `seed.syntax_overrides` itself just ends up
    // holding whichever import's pattern happened to merge in first for a
    // conflicted name; that value is discarded (or replaced by a local
    // declaration) before it's ever used, so which one doesn't matter.
    let mut conflicted_overrides: HashSet<String> = HashSet::new();

    for import in &imports {
        // A named import only brings those names' parse information.
        let only = match &import.items {
            ImportItems::Names(names) => Some(names.as_slice()),
            ImportItems::All => None,
        };

        let (child_paths, only) = match resolve_import_paths(import, path)? {
            ImportResolution::Plain(child_path) => (vec![child_path], only),
            ImportResolution::Package(child_paths) => (child_paths, None),
        };

        for child_path in child_paths {
            load_module(&child_path, state)?;
            merge_seed(&mut seed, &state.cache[&child_path].seed, only, &mut conflicted_overrides);
        }
    }

    let (program, mut seed) = parser::parse_seeded(tokens, docs, &seed).map_err(|error| {
        LoadError::Parse {
            path: path.to_path_buf(),
            message: error.message,
            span: error.span,
        }
    })?;

    for name in &conflicted_overrides {
        if !seed.local_syntax_overrides.contains(name) {
            return Err(LoadError::ConflictingSyntaxOverride {
                importer: path.to_path_buf(),
                name: name.clone(),
            });
        }
    }

    state.stack.pop();

    let module_id = state.cache.len();
    let imported = imported_bindings(&program.statements, path, &state.cache)?;

    // Top-level `@for`/`@if` bounds may read the integer constants this
    // file can see, except where one of its own consts shadows the name.
    let own_const_names: HashSet<String> = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Const(decl) => literal_name(&decl.name),
            _ => None,
        })
        .collect();

    let mut consts: HashMap<String, Int> = imported
        .scope
        .iter()
        .filter(|(name, _)| !own_const_names.contains(*name))
        .filter_map(|(name, binding)| match binding {
            Binding::Decl(internal) => Some((name.clone(), state.const_values.get(internal)?.clone())),
            _ => None,
        })
        .collect();

    let unrolled = resolver::unroll_module(&program.statements, &mut consts).map_err(|error| {
        LoadError::Resolve { path: path.to_path_buf(), error: Box::new(error) }
    })?;

    // This file's own declarations shadow anything it imports, except that
    // a macro's overloads join whatever overloads of the same name it
    // imports: a dialect adds `mov` overloads to the `mov` it builds on.
    let mut own: HashMap<String, Binding> = HashMap::new();
    let mut own_pub: HashMap<String, Binding> = HashMap::new();

    for statement in &unrolled {
        // One name, one declaration per file — only macros overload.
        if let Some((name, span)) = own_name_and_span(statement) {
            let is_macro = matches!(statement, Statement::Macro(_));
            let clashes = match own.get(&name) {
                Some(Binding::Macros { .. }) => !is_macro,
                Some(_) => true,
                None => false,
            };
            if clashes {
                return Err(LoadError::Resolve {
                    path: path.to_path_buf(),
                    error: Box::new(ResolveError::DuplicateSymbol { name, span }),
                });
            }
        }

        if let Some((name, is_pub)) = declaration_name(statement) {
            let binding = match statement {
                Statement::Macro(_) => Binding::Macros {
                    name: name.clone(),
                    modules: BTreeSet::from([module_id]),
                },
                _ => Binding::Decl(internal_name(&name, module_id)),
            };

            if let (Statement::Const(_), Some(value)) = (statement, consts.get(&name)) {
                state.const_values.insert(internal_name(&name, module_id), value.clone());
            }

            if is_pub {
                own_pub.insert(name.clone(), binding.clone());
            }
            own.insert(name, binding);
        } else if let Statement::Label(label) = statement {
            own.insert(label.name.clone(), Binding::Label(label.name.clone()));
        }
    }

    let scope = shadow(own, imported.scope);
    let exports = shadow(own_pub, imported.exports);

    let mut statements = Vec::with_capacity(unrolled.len());

    for mut statement in unrolled.clone() {
        let mut renamer = Renamer::new(&scope, &statement);
        name_declaration(&mut statement, module_id);
        rename_statement(&mut statement, &mut renamer);

        if let Some((name, candidates)) = renamer.ambiguous {
            return Err(LoadError::AmbiguousName {
                importer: path.to_path_buf(),
                name,
                modules: describe_origins(&candidates, &state.cache),
            });
        }

        statements.push(statement);
    }

    // Importers get parse information only for what they can name.
    seed.generic_signatures.retain(|name, _| exports.contains_key(name));
    seed.macro_syntaxes.retain(|name, _| exports.contains_key(name));
    seed.syntax_overrides.retain(|name, _| exports.contains_key(name));
    seed.unanchored_syntaxes.retain(|(name, _)| exports.contains_key(name));

    state.cache.insert(
        path.to_path_buf(),
        LoadedModule {
            source_statements: program.statements,
            unrolled,
            doc: program.doc,
            stray_docs: program.stray_docs,
            statements,
            extern_labels: imported.extern_labels,
            span: program.span,
            seed,
            scope,
            exports,
            module_id,
        },
    );

    Ok(())
}

// Adds a child module's exported parse information to `seed`: all of it, or
// only the entries for `only`'s names.
fn merge_seed(
    seed: &mut ParserSeed,
    child_seed: &ParserSeed,
    only: Option<&[String]>,
    conflicted_overrides: &mut HashSet<String>,
) {
    let wanted = |name: &String| only.is_none_or(|names| names.contains(name));

    for (name, signature) in &child_seed.generic_signatures {
        if wanted(name) {
            seed.generic_signatures.insert(name.clone(), signature.clone());
        }
    }

    for (name, patterns) in &child_seed.macro_syntaxes {
        if !wanted(name) {
            continue;
        }
        let known = seed.macro_syntaxes.entry(name.clone()).or_default();
        for pattern in patterns {
            if !known.contains(pattern) {
                known.push(pattern.clone());
            }
        }
    }

    for entry in &child_seed.unanchored_syntaxes {
        if wanted(&entry.0) && !seed.unanchored_syntaxes.contains(entry) {
            seed.unanchored_syntaxes.push(entry.clone());
        }
    }

    for (name, pattern) in &child_seed.syntax_overrides {
        if !wanted(name) {
            continue;
        }
        match seed.syntax_overrides.get(name) {
            Some(existing) if existing != pattern => {
                conflicted_overrides.insert(name.clone());
            }
            Some(_) => {}
            None => {
                seed.syntax_overrides.insert(name.clone(), pattern.clone());
            }
        }
    }
}

// Resolves one `from <module> import <items>` statement to every file path
// it draws from: a plain module (`<module>` resolves directly to a `.basm`
// file) is `Plain`, one path; a package-style import (`<module>` is a
// directory rather than a file — see `resolve_submodule_path`) is
// `Package`, one path per named item, each treated as sugar for
// `from <module>.<name> import *`. `import *` has no name list to fall back
// through, so a directory-shaped `<module>` there is just whatever error
// plain resolution produces — there's no `Package` variant for it.
enum ImportResolution {
    Plain(PathBuf),
    Package(Vec<PathBuf>),
}

fn resolve_import_paths(
    import: &ImportStatement,
    importer: &Path,
) -> Result<ImportResolution, LoadError> {
    match (resolve_module_path(&import.module, importer), &import.items) {
        (Ok(target_path), _) => Ok(ImportResolution::Plain(target_path)),

        // If the submodule fallback fails too, report the module path the
        // user actually wrote, not `<module>.<name>`.
        (Err(error), ImportItems::Names(names)) => names
            .iter()
            .map(|name| resolve_submodule_path(&import.module, name, importer))
            .collect::<Result<_, _>>()
            .map(ImportResolution::Package)
            .map_err(|_| error),

        (Err(error), ImportItems::All) => Err(error),
    }
}

/// Whether `import` names modules in a directory (`from std.riscv import
/// native`) rather than declarations in one module.
pub fn imports_modules(import: &ImportStatement, importer: &Path) -> bool {
    matches!(resolve_import_paths(import, importer), Ok(ImportResolution::Package(_)))
}

fn import_targets(import: &ImportStatement, importer: &Path) -> Result<Vec<PathBuf>, LoadError> {
    Ok(match resolve_import_paths(import, importer)? {
        ImportResolution::Plain(target_path) => vec![target_path],
        ImportResolution::Package(target_paths) => target_paths,
    })
}

// Appends a module's declarations, and everything its own imports need, to
// `out`. A program needs every module it transitively imports, whether or
// not their names are visible to it; internal names keep them apart. Modules
// already spliced elsewhere in this build (diamond imports) contribute
// nothing the second time.
fn collect_declarations(
    path: &Path,
    cache: &HashMap<PathBuf, LoadedModule>,
    spliced: &mut HashSet<PathBuf>,
    out: &mut Vec<Statement>,
    out_modules: &mut Vec<usize>,
) -> Result<(), LoadError> {
    if !spliced.insert(path.to_path_buf()) {
        return Ok(());
    }

    let module = &cache[path];

    for statement in &module.statements {
        match statement {
            Statement::Import(nested) => {
                for target in import_targets(nested, path)? {
                    collect_declarations(&target, cache, spliced, out, out_modules)?;
                }
            }

            Statement::Struct(_)
            | Statement::Enum(_)
            | Statement::TypeAlias(_)
            | Statement::Const(_)
            | Statement::Macro(_)
            | Statement::Meta(_) => {
                out.push(statement.clone());
                out_modules.push(module.module_id);
            }

            // Labels and invocations are program bodies, not declarations;
            // nothing else imports them. A syntax override isn't a
            // declaration either — its effect already happened at parse
            // time, propagated via `ParserSeed`, not by being spliced into
            // an importer's statement list. `ExternLabel` is never present
            // in a *loaded* module's own `statements` — it's synthesized
            // from an import (see `extern_labels` below) — so it's matched
            // here only for exhaustiveness.
            Statement::Label(_) | Statement::Section(_) | Statement::Invocation(_)
            | Statement::SyntaxOverride(_) | Statement::ExternLabel(_) => {}
        }
    }

    // A label this module imports is carried along like its other
    // declarations: its macros may refer to it wherever they're called (e.g.
    // `std.formats.elf` using `std.bitter.link`'s `image_end`).
    for (label, module_id) in &module.extern_labels {
        out.push(Statement::ExternLabel(label.clone()));
        out_modules.push(*module_id);
    }

    Ok(())
}

// A name and whether it's reachable from outside the file that declared it.
// Every one of these can in principle carry a splice, but only meaningfully
// so once generated from inside a live macro body — a top-level declaration
// (the only kind this function ever sees) is always fully literal once
// unrolled, so a non-literal name here just means there's nothing to name —
// `resolver::collect_symbols` is what rejects that case with a real error.
fn declaration_name(statement: &Statement) -> Option<(String, bool)> {
    match statement {
        Statement::Struct(decl) => literal_name(&decl.name).map(|name| (name, decl.is_pub)),
        Statement::Enum(decl) => literal_name(&decl.name).map(|name| (name, decl.is_pub)),
        Statement::TypeAlias(decl) => literal_name(&decl.name).map(|name| (name, decl.is_pub)),
        Statement::Const(decl) => literal_name(&decl.name).map(|name| (name, decl.is_pub)),
        Statement::Macro(decl) => literal_name(&decl.name).map(|name| (name, decl.is_pub)),
        _ => None,
    }
}

// The name a top-level statement declares in its own file, and where.
fn own_name_and_span(statement: &Statement) -> Option<(String, Span)> {
    let (name, span) = match statement {
        Statement::Struct(decl) => (literal_name(&decl.name)?, decl.span),
        Statement::Enum(decl) => (literal_name(&decl.name)?, decl.span),
        Statement::TypeAlias(decl) => (literal_name(&decl.name)?, decl.span),
        Statement::Const(decl) => (literal_name(&decl.name)?, decl.span),
        Statement::Macro(decl) => (literal_name(&decl.name)?, decl.span),
        Statement::Label(label) => (label.name.clone(), label.span),
        _ => return None,
    };
    Some((name, span))
}

// ===============
// namespaces
// ===============
//
// Each file sees its own declarations and exactly what it imports. The
// resolver, though, works over one flattened program with a single flat
// (by-name) symbol table, so two files' same-named declarations can't both
// keep their names. Instead every top-level declaration gets an internal
// name, `name#module`, and every reference is rewritten to the internal
// name of whatever it means in the file that wrote it. `#` can't appear in
// an identifier a `.basm` file actually wrote (the lexer only ever treats
// it as the start of a line comment), so no source text can spell one.
//
// A macro reference means an overload set, possibly spanning modules (a
// dialect adds overloads to the mnemonics it imports), so it's rewritten to
// `name#a,b,...`, which the resolver reads as every overload of `name`
// declared in modules `a`, `b`, .... Labels keep their own names: they only
// exist in the file being compiled.

/// What a name means inside one module.
#[derive(Debug, Clone, PartialEq)]
enum Binding {
    /// A struct, enum, type alias or const, by internal name.
    Decl(String),

    /// A label in this file.
    Label(String),

    /// Every overload of macro `name` declared in one of `modules`.
    Macros { name: String, modules: BTreeSet<usize> },

    /// A `pub` label imported from another file, and that file's module id.
    ExternLabel(ExternLabel, usize),

    /// Imports that bring different declarations under this name. Only an
    /// error once the name is used.
    Ambiguous(Vec<Binding>),
}

impl Binding {
    fn internal_name(&self) -> Option<String> {
        match self {
            Binding::Decl(name) | Binding::Label(name) => Some(name.clone()),

            Binding::Macros { name, modules } => {
                let modules: Vec<String> = modules.iter().map(usize::to_string).collect();
                Some(format!("{name}#{}", modules.join(",")))
            }

            Binding::ExternLabel(label, _) => Some(label.name.clone()),

            Binding::Ambiguous(_) => None,
        }
    }

    // One name brought in twice: the same declaration, or macro overloads
    // that join, or else an ambiguity.
    fn merge(self, other: Binding) -> Binding {
        if self == other {
            return self;
        }

        match (self, other) {
            (Binding::Macros { name, mut modules }, Binding::Macros { modules: theirs, .. }) => {
                modules.extend(theirs);
                Binding::Macros { name, modules }
            }

            (first, second) => {
                let mut candidates = Vec::new();
                for binding in [first, second] {
                    match binding {
                        Binding::Ambiguous(inner) => candidates.extend(inner),
                        other => candidates.push(other),
                    }
                }
                candidates.dedup();
                Binding::Ambiguous(candidates)
            }
        }
    }
}

fn internal_name(name: &str, module_id: usize) -> String {
    format!("{name}#{module_id}")
}

/// Strips the loader's internal naming (`name#module`, `name#a,b`) back off
/// every name in `text`, for anything shown to a person.
pub fn demangle(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let digit_at = |index: usize| chars.get(index).is_some_and(char::is_ascii_digit);

    let mut out = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '#' && digit_at(index + 1) {
            // `#3`, `#3,5`, ... — a comma belongs to the name only when
            // another module id follows it.
            index += 1;
            loop {
                while digit_at(index) {
                    index += 1;
                }
                if chars.get(index) == Some(&',') && digit_at(index + 1) {
                    index += 1;
                } else {
                    break;
                }
            }
            continue;
        }
        out.push(chars[index]);
        index += 1;
    }

    out
}

struct ImportedBindings {
    // Everything this file's imports bring in.
    scope: HashMap<String, Binding>,
    // The part of `scope` its `pub from` imports re-export.
    exports: HashMap<String, Binding>,
    extern_labels: Vec<(ExternLabel, usize)>,
}

// What a file's imports bring in. A name imported explicitly
// (`from x import name`) takes precedence over one a `*` brings in.
fn imported_bindings(
    statements: &[Statement],
    importer: &Path,
    cache: &HashMap<PathBuf, LoadedModule>,
) -> Result<ImportedBindings, LoadError> {
    // (name, binding, explicit, re-exported)
    let mut candidates: Vec<(String, Binding, bool, bool)> = Vec::new();
    let mut extern_labels = Vec::new();

    for statement in statements {
        let Statement::Import(import) = statement else { continue };

        match (resolve_import_paths(import, importer)?, &import.items) {
            (ImportResolution::Plain(target_path), ImportItems::Names(names)) => {
                let target = &cache[&target_path];

                for name in names {
                    if let Some(binding) = target.exports.get(name) {
                        candidates.push((name.clone(), binding.clone(), true, import.is_pub));
                        continue;
                    }

                    let is_pub_label = target.statements.iter().any(|statement| {
                        matches!(statement, Statement::Label(label) if label.is_pub && &label.name == name)
                    });

                    if !is_pub_label {
                        return Err(LoadError::UnknownImportedName {
                            module: module_display(&import.module),
                            name: name.clone(),
                        });
                    }

                    let label = ExternLabel {
                        name: internal_name(name, target.module_id),
                        file: target_path.display().to_string(),
                        module: module_path_of(&target_path),
                        span: import.span,
                    };
                    extern_labels.push((label.clone(), target.module_id));
                    candidates.push((
                        name.clone(),
                        Binding::ExternLabel(label, target.module_id),
                        true,
                        import.is_pub,
                    ));
                }
            }

            (ImportResolution::Plain(target_path), ImportItems::All) => {
                for (name, binding) in &cache[&target_path].exports {
                    candidates.push((name.clone(), binding.clone(), false, import.is_pub));
                }
            }

            (ImportResolution::Package(target_paths), _) => {
                for target_path in target_paths {
                    for (name, binding) in &cache[&target_path].exports {
                        candidates.push((name.clone(), binding.clone(), false, import.is_pub));
                    }
                }
            }
        }
    }

    let resolve = |reexported_only: bool| {
        let mut explicit: HashMap<String, Binding> = HashMap::new();
        let mut glob: HashMap<String, Binding> = HashMap::new();

        for (name, binding, is_explicit, is_pub) in &candidates {
            if reexported_only && !is_pub {
                continue;
            }
            let map = if *is_explicit { &mut explicit } else { &mut glob };
            let merged = match map.remove(name) {
                Some(existing) => existing.merge(binding.clone()),
                None => binding.clone(),
            };
            map.insert(name.clone(), merged);
        }

        glob.extend(explicit);
        glob
    };

    Ok(ImportedBindings { scope: resolve(false), exports: resolve(true), extern_labels })
}

// A file's own declarations over what it imports: its own name wins, except
// that its macro overloads join same-named imported ones.
fn shadow(own: HashMap<String, Binding>, mut imported: HashMap<String, Binding>) -> HashMap<String, Binding> {
    for (name, binding) in own {
        let combined = match (binding, imported.remove(&name)) {
            (Binding::Macros { name: macro_name, mut modules }, Some(Binding::Macros { modules: theirs, .. })) => {
                modules.extend(theirs);
                Binding::Macros { name: macro_name, modules }
            }
            (binding, _) => binding,
        };
        imported.insert(name, combined);
    }

    imported
}

// The module paths an ambiguous name's candidates come from.
fn describe_origins(candidates: &[Binding], cache: &HashMap<PathBuf, LoadedModule>) -> Vec<String> {
    let module_path_of_id = |id: usize| {
        cache
            .iter()
            .find(|(_, module)| module.module_id == id)
            .map(|(path, _)| module_path_of(path))
            .unwrap_or_default()
    };

    let mut modules = Vec::new();
    for candidate in candidates {
        match candidate {
            Binding::Decl(name) => {
                if let Some(id) = name.rsplit_once('#').and_then(|(_, id)| id.parse().ok()) {
                    modules.push(module_path_of_id(id));
                }
            }
            Binding::Macros { modules: ids, .. } => modules.extend(ids.iter().map(|id| module_path_of_id(*id))),
            Binding::ExternLabel(label, _) => modules.push(label.module.clone()),
            Binding::Label(_) | Binding::Ambiguous(_) => {}
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

// Gives a top-level declaration its internal name.
fn name_declaration(statement: &mut Statement, module_id: usize) {
    let name = match statement {
        Statement::Struct(decl) => &mut decl.name,
        Statement::Enum(decl) => &mut decl.name,
        Statement::TypeAlias(decl) => &mut decl.name,
        Statement::Const(decl) => &mut decl.name,
        Statement::Macro(decl) => &mut decl.name,
        _ => return,
    };

    if let Some(literal) = literal_name(name) {
        *name = vec![NamePart::Literal(internal_name(&literal, module_id))];
    }
}

// Rewrites the references inside one top-level statement. A name the
// statement binds anywhere inside itself (a parameter, a loop variable, a
// macro body's own `const`, ...) is left alone everywhere in it, so a
// parameter never turns into a same-named global.
struct Renamer<'a> {
    scope: &'a HashMap<String, Binding>,
    values: HashSet<String>,
    types: HashSet<String>,

    // The first ambiguous name referenced, if any.
    ambiguous: Option<(String, Vec<Binding>)>,
}

impl<'a> Renamer<'a> {
    fn new(scope: &'a HashMap<String, Binding>, statement: &Statement) -> Self {
        let mut binders = Binders::default();
        binders.statement(statement, true);
        Self { scope, values: binders.values, types: binders.types, ambiguous: None }
    }

    /// The internal name for a reference in value position.
    fn get(&mut self, name: &str) -> Option<String> {
        if self.values.contains(name) {
            return None;
        }
        self.lookup(name)
    }

    /// The internal name for a reference in type position.
    fn get_type(&mut self, name: &str) -> Option<String> {
        if self.types.contains(name) {
            return None;
        }
        self.lookup(name)
    }

    fn lookup(&mut self, name: &str) -> Option<String> {
        match self.scope.get(name)? {
            Binding::Ambiguous(candidates) => {
                self.ambiguous.get_or_insert_with(|| (name.to_string(), candidates.clone()));
                None
            }
            binding => binding.internal_name(),
        }
    }
}

// Every name a statement binds inside itself: `values` for names read as
// values, `types` for names used as types.
#[derive(Default)]
struct Binders {
    values: HashSet<String>,
    types: HashSet<String>,
}

impl Binders {
    fn both(&mut self, name: &str) {
        self.values.insert(name.to_string());
        self.types.insert(name.to_string());
    }

    fn generics(&mut self, params: &[GenericParameter]) {
        for param in params {
            match param {
                GenericParameter::Const { name, .. } | GenericParameter::Type { name, .. } => self.both(name),
            }
        }
    }

    // A declaration nested in a macro body is generated when the macro
    // runs, under its own name.
    fn nested(&mut self, name: &[NamePart], top: bool) {
        if !top {
            if let Some(name) = literal_name(name) {
                self.both(&name);
            }
        }
    }

    fn statement(&mut self, statement: &Statement, top: bool) {
        match statement {
            Statement::Struct(decl) => {
                self.nested(&decl.name, top);
                self.generics(&decl.generic_params);
                self.values.insert("source".to_string());
                self.struct_items(&decl.fields);
                self.facets(&decl.facets);
            }

            Statement::TypeAlias(decl) => {
                self.nested(&decl.name, top);
                self.generics(&decl.generic_params);
                self.values.insert("source".to_string());
                self.facets(&decl.facets);
            }

            Statement::Enum(decl) => {
                self.nested(&decl.name, top);
                self.generics(&decl.generic_params);
            }

            Statement::Const(decl) => {
                self.nested(&decl.name, top);
                self.expr(&decl.value);
            }

            Statement::Macro(decl) => {
                self.nested(&decl.name, top);
                self.generics(&decl.generic_params);
                for param in &decl.params {
                    self.values.insert(param.name.clone());
                    if let Some(default) = &param.default {
                        self.expr(default);
                    }
                }
                self.values.insert("result".to_string());
                self.facets(&decl.facets);
                for statement in &decl.body {
                    self.statement(statement, false);
                }
            }

            Statement::Label(label) if !top => {
                self.values.insert(label.name.clone());
            }

            Statement::Meta(meta) => self.meta(meta),

            Statement::Invocation(invocation) => {
                for operand in &invocation.operands {
                    self.expr(operand);
                }
            }

            _ => {}
        }
    }

    fn meta(&mut self, meta: &MetaStatement) {
        if matches!(meta.name.as_str(), "for" | "fold") {
            if let Some(Expr::Identifier { name, .. }) = meta.args.first() {
                self.values.insert(name.clone());
            }
            if meta.name == "fold" {
                for binding in &meta.bindings {
                    self.values.insert(binding.name.clone());
                }
            }
        }

        for arg in &meta.args {
            self.expr(arg);
        }
        for binding in &meta.bindings {
            self.expr(&binding.value);
        }
        for statement in meta.body.iter().chain(&meta.else_body).flatten() {
            self.statement(statement, false);
        }
        for arm in &meta.match_arms {
            // `Variant(binding)` binds `binding` for the arm.
            if let Some(Expr::Call { arguments, .. }) = &arm.pattern {
                for argument in arguments {
                    if let Expr::Identifier { name, .. } = &argument.value {
                        self.values.insert(name.clone());
                    }
                }
            }
            for statement in &arm.body {
                self.statement(statement, false);
            }
        }
    }

    fn facets(&mut self, facets: &[Facet]) {
        for facet in facets {
            match &facet.payload {
                FacetPayload::Expr(expr) => self.expr(expr),
                FacetPayload::Block(statements) => {
                    for statement in statements {
                        self.statement(statement, false);
                    }
                }
                FacetPayload::Bare | FacetPayload::Type(_) | FacetPayload::Pattern(_) => {}
            }
        }
    }

    fn struct_items(&mut self, items: &[StructBodyItem]) {
        for item in items {
            match item {
                StructBodyItem::Field(field) => {
                    if let Some(default) = &field.default {
                        self.expr(default);
                    }
                }
                StructBodyItem::For { var, source, body, .. } => {
                    self.values.insert(var.clone());
                    self.expr(source);
                    self.struct_items(body);
                }
                StructBodyItem::Fold { accumulators, var, source, body, .. } => {
                    self.values.insert(var.clone());
                    for accumulator in accumulators {
                        self.values.insert(accumulator.name.clone());
                        self.expr(&accumulator.value);
                    }
                    self.expr(source);
                    self.struct_items(body);
                }
                StructBodyItem::Next { .. } => {}
                StructBodyItem::If { condition, body, else_body, .. } => {
                    self.expr(condition);
                    self.struct_items(body);
                    if let Some(else_body) = else_body {
                        self.struct_items(else_body);
                    }
                }
            }
        }
    }

    // Only a `@fold` or a construction's `@for`/`@fold` binds anything
    // inside an expression.
    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Fold { fold, .. } => self.meta(fold),

            Expr::Construct { callee, fields, .. } => {
                self.expr(callee);
                self.construct_items(fields);
            }

            Expr::Member { object, .. } => self.expr(object),
            Expr::Call { callee, arguments, .. } => {
                self.expr(callee);
                for argument in arguments {
                    self.expr(&argument.value);
                }
            }
            Expr::EnumVariant { payload: Some(payload), .. } => self.expr(payload),
            Expr::Unary { operand, .. } => self.expr(operand),
            Expr::Binary { left, right, .. } | Expr::Range { start: left, end: right, .. }
            | Expr::In { value: left, source: right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expr::Splice { inner, .. } | Expr::As { value: inner, .. } => self.expr(inner),

            Expr::Identifier { .. } | Expr::SplicedIdentifier { .. } | Expr::Integer { .. }
            | Expr::String { .. } | Expr::EnumVariant { payload: None, .. } => {}
        }
    }

    fn construct_items(&mut self, items: &[ConstructItem]) {
        for item in items {
            match item {
                ConstructItem::Field { value, .. } => self.expr(value),
                ConstructItem::For { var, source, body, .. } => {
                    self.values.insert(var.clone());
                    self.expr(source);
                    self.construct_items(body);
                }
                ConstructItem::Fold { accumulators, var, source, body, .. } => {
                    self.values.insert(var.clone());
                    for accumulator in accumulators {
                        self.values.insert(accumulator.name.clone());
                        self.expr(&accumulator.value);
                    }
                    self.expr(source);
                    self.construct_items(body);
                }
                ConstructItem::Next { .. } => {}
                ConstructItem::If { condition, body, else_body, .. } => {
                    self.expr(condition);
                    self.construct_items(body);
                    if let Some(else_body) = else_body {
                        self.construct_items(else_body);
                    }
                }
            }
        }
    }
}

fn rename_statement(statement: &mut Statement, renames: &mut Renamer) {
    match statement {
        Statement::Struct(decl) => {
            rename_spliced_name(&mut decl.name, renames);

            for param in &mut decl.generic_params {
                rename_generic_parameter(param, renames);
            }

            rename_struct_body_items(&mut decl.fields, renames);

            for facet in &mut decl.facets {
                rename_facet(facet, renames);
            }
        }

        Statement::TypeAlias(decl) => {
            rename_spliced_name(&mut decl.name, renames);

            for param in &mut decl.generic_params {
                rename_generic_parameter(param, renames);
            }

            rename_type_expr(&mut decl.ty, renames);

            for facet in &mut decl.facets {
                rename_facet(facet, renames);
            }
        }

        Statement::Const(decl) => {
            rename_spliced_name(&mut decl.name, renames);

            if let Some(ty) = &mut decl.ty {
                rename_type_expr(ty, renames);
            }

            rename_expr(&mut decl.value, renames);
        }

        Statement::Macro(decl) => {
            rename_spliced_name(&mut decl.name, renames);

            for param in &mut decl.generic_params {
                rename_generic_parameter(param, renames);
            }

            for param in &mut decl.params {
                rename_type_expr(&mut param.ty, renames);
                if let Some(default) = &mut param.default {
                    rename_expr(default, renames);
                }
            }

            if let Some(ty) = &mut decl.return_ty {
                rename_type_expr(ty, renames);
            }

            for facet in &mut decl.facets {
                rename_facet(facet, renames);
            }

            for statement in &mut decl.body {
                rename_statement(statement, renames);
            }
        }

        Statement::Meta(meta) => rename_meta_statement(meta, renames),

        Statement::Enum(decl) => {
            rename_spliced_name(&mut decl.name, renames);

            for param in &mut decl.generic_params {
                rename_generic_parameter(param, renames);
            }
            for variant in &mut decl.variants {
                if let Some(payload) = &mut variant.payload {
                    rename_type_expr(payload, renames);
                }
            }
        }

        // An invocation inside a macro body may call one of this module's
        // private macros, or pass its private constants.
        Statement::Invocation(invocation) => {
            if let Some(mangled) = renames.get(&invocation.name) {
                invocation.name = mangled.clone();
            }
            for operand in &mut invocation.operands {
                rename_expr(operand, renames);
            }
        }

        // Nothing to rename. `collect_declarations` never splices a
        // `SyntaxOverride` or `ExternLabel` into a statement list for this
        // to run on; they're matched here for exhaustiveness.
        Statement::Import(_) | Statement::Label(_) | Statement::Section(_)
        | Statement::SyntaxOverride(_) | Statement::ExternLabel(_) => {}
    }
}

// A macro body's `@if`/`@for` can reference this module's own private
// siblings in its condition/range bounds, just like any other statement
// inside the body — so its `args` and nested `body`/`else_body` all need
// the same rewriting a plain statement would get.
fn rename_meta_statement(meta: &mut MetaStatement, renames: &mut Renamer) {
    for arg in &mut meta.args {
        rename_expr(arg, renames);
    }

    for binding in &mut meta.bindings {
        rename_expr(&mut binding.value, renames);
    }

    if let Some(body) = &mut meta.body {
        for statement in body {
            rename_statement(statement, renames);
        }
    }

    if let Some(else_body) = &mut meta.else_body {
        for statement in else_body {
            rename_statement(statement, renames);
        }
    }

    for arm in &mut meta.match_arms {
        if let Some(pattern) = &mut arm.pattern {
            rename_expr(pattern, renames);
        }
        for statement in &mut arm.body {
            rename_statement(statement, renames);
        }
    }
}

fn rename_struct_body_items(items: &mut [StructBodyItem], renames: &mut Renamer) {
    for item in items {
        match item {
            StructBodyItem::Field(field) => {
                rename_type_expr(&mut field.ty, renames);
                rename_spliced_name(&mut field.name, renames);
                if let Some(default) = &mut field.default {
                    rename_expr(default, renames);
                }
            }

            StructBodyItem::For { source, body, .. } => {
                rename_expr(source, renames);
                rename_struct_body_items(body, renames);
            }

            StructBodyItem::Fold { accumulators, source, body, .. } => {
                for accumulator in accumulators {
                    rename_expr(&mut accumulator.value, renames);
                }
                rename_expr(source, renames);
                rename_struct_body_items(body, renames);
            }

            StructBodyItem::Next { value, updates, .. } => {
                if let Some(value) = value {
                    rename_expr(value, renames);
                }
                for update in updates {
                    rename_expr(&mut update.value, renames);
                }
            }

            StructBodyItem::If { condition, body, else_body, .. } => {
                rename_expr(condition, renames);
                rename_struct_body_items(body, renames);

                if let Some(else_body) = else_body {
                    rename_struct_body_items(else_body, renames);
                }
            }
        }
    }
}

fn rename_spliced_name(parts: &mut [NamePart], renames: &mut Renamer) {
    for part in parts {
        if let NamePart::Splice(expr) = part {
            rename_expr(expr, renames);
        }
    }
}

fn rename_facet(facet: &mut Facet, renames: &mut Renamer) {
    // A lint facet names lints, not declarations.
    if crate::diagnostics::is_lint_facet(&facet.name) {
        return;
    }

    match &mut facet.payload {
        FacetPayload::Bare => {}

        FacetPayload::Expr(expr) => rename_expr(expr, renames),

        FacetPayload::Block(statements) => {
            for statement in statements {
                rename_statement(statement, renames);
            }
        }

        FacetPayload::Type(ty) => rename_type_expr(ty, renames),

        // A pattern's tokens are either `$capture$` names (the declaring
        // macro's own params, not a reference to anything renameable) or
        // literal call-site text the parser matches verbatim — nothing in
        // it is ever a symbol reference.
        FacetPayload::Pattern(_) => {}
    }
}

fn rename_generic_parameter(param: &mut GenericParameter, renames: &mut Renamer) {
    match param {
        GenericParameter::Const { ty, .. } => rename_type_expr(ty, renames),

        GenericParameter::Type { bound: Some(bound), .. } => {
            for param in &mut bound.params {
                rename_type_expr(param, renames);
            }
            if let Some(ret) = &mut bound.ret {
                rename_type_expr(ret, renames);
            }
        }

        GenericParameter::Type { bound: None, .. } => {}
    }
}

fn rename_type_expr(ty: &mut TypeExpr, renames: &mut Renamer) {
    match ty {
        TypeExpr::Named { path, .. } => {
            if path.len() == 1 {
                if let Some(mangled) = renames.get_type(&path[0]) {
                    path[0] = mangled.clone();
                }
            }
        }

        TypeExpr::Apply { base, args, .. } => {
            rename_type_expr(base, renames);

            for arg in args {
                match arg {
                    TypeArgument::Type(ty) => rename_type_expr(ty, renames),
                    TypeArgument::Const(expr) => rename_expr(expr, renames),
                    TypeArgument::Wildcard(_) => {}
                }
            }
        }
    }
}

fn rename_expr(expr: &mut Expr, renames: &mut Renamer) {
    match expr {
        Expr::Identifier { name, .. } => {
            if let Some(mangled) = renames.get(name) {
                *name = mangled.clone();
            }
        }

        // With nothing left to evaluate (`` k`2` ``, or a top-level `@for`
        // body's after unrolling), a spliced name is just a name.
        Expr::SplicedIdentifier { name, span } if crate::ast::literal_spliced_name(name).is_some() => {
            let name = crate::ast::literal_spliced_name(name).unwrap_or_default();
            *expr = Expr::Identifier { name, span: *span };
            rename_expr(expr, renames);
        }

        Expr::SplicedIdentifier { name, .. } => {
            for part in name {
                if let NamePart::Splice(expr) = part {
                    rename_expr(expr, renames);
                }
            }
        }

        Expr::Integer { .. } | Expr::String { .. } => {}

        Expr::Member { object, .. } => rename_expr(object, renames),

        Expr::Call { callee, arguments, .. } => {
            rename_expr(callee, renames);

            for argument in arguments {
                rename_expr(&mut argument.value, renames);
            }
        }

        Expr::EnumVariant { enum_name, generic_args, payload, .. } => {
            if let Some(mangled) = renames.get_type(enum_name) {
                *enum_name = mangled.clone();
            }
            for arg in generic_args {
                match arg {
                    TypeArgument::Type(ty) => rename_type_expr(ty, renames),
                    TypeArgument::Const(expr) => rename_expr(expr, renames),
                    TypeArgument::Wildcard(_) => {}
                }
            }
            if let Some(payload) = payload {
                rename_expr(payload, renames);
            }
        }

        Expr::Unary { operand, .. } => rename_expr(operand, renames),

        Expr::Binary { left, right, .. } => {
            rename_expr(left, renames);
            rename_expr(right, renames);
        }

        Expr::Splice { inner, .. } => rename_expr(inner, renames),

        Expr::Construct { callee, generic_args, fields, .. } => {
            rename_expr(callee, renames);

            for arg in generic_args {
                match arg {
                    TypeArgument::Type(ty) => rename_type_expr(ty, renames),
                    TypeArgument::Const(expr) => rename_expr(expr, renames),
                    TypeArgument::Wildcard(_) => {}
                }
            }

            rename_construct_items(fields, renames);
        }

        Expr::As { value, ty, .. } => {
            rename_expr(value, renames);
            rename_type_expr(ty, renames);
        }

        Expr::Range { start, end, .. } => {
            rename_expr(start, renames);
            rename_expr(end, renames);
        }

        Expr::In { value, source, .. } => {
            rename_expr(value, renames);
            rename_expr(source, renames);
        }

        Expr::Fold { fold, .. } => rename_meta_statement(fold, renames),
    }
}

fn rename_construct_items(items: &mut [ConstructItem], renames: &mut Renamer) {
    for item in items {
        match item {
            ConstructItem::Field { name, value, .. } => {
                rename_spliced_name(name, renames);
                rename_expr(value, renames);
            }

            ConstructItem::For { source, body, .. } => {
                rename_expr(source, renames);
                rename_construct_items(body, renames);
            }

            ConstructItem::Fold { accumulators, source, body, .. } => {
                for accumulator in accumulators {
                    rename_expr(&mut accumulator.value, renames);
                }
                rename_expr(source, renames);
                rename_construct_items(body, renames);
            }

            ConstructItem::Next { value, updates, .. } => {
                if let Some(value) = value {
                    rename_expr(value, renames);
                }
                for update in updates {
                    rename_expr(&mut update.value, renames);
                }
            }

            ConstructItem::If { condition, body, else_body, .. } => {
                rename_expr(condition, renames);
                rename_construct_items(body, renames);

                if let Some(else_body) = else_body {
                    rename_construct_items(else_body, renames);
                }
            }
        }
    }
}

/// The directories an absolute (no leading dots) module path is looked up
/// under, in priority order: the current working directory, each entry of
/// `BITTERASM_PATH` (split like `PATH`), then the install root
/// `~/.bitterasm`. A project's own `std/` therefore shadows the installed one.
pub fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }

    if let Some(paths) = std::env::var_os("BITTERASM_PATH") {
        roots.extend(std::env::split_paths(&paths).filter(|path| !path.as_os_str().is_empty()));
    }

    let home_dir = std::env::home_dir();

    if let Some(home) = &home_dir {
        roots.push(home.join(".bitterasm"));
    }

    if cfg!(all(unix, not(target_vendor = "apple"))) {
        const DEFAULT_DATA_DIRS: &str = "/usr/local/share:/usr/share";

        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| Some(home_dir.as_ref()?.join(".local/share")));

        let data_dirs = std::env::var_os("XDG_DATA_DIRS");
        let data_dirs = data_dirs.as_deref().unwrap_or(DEFAULT_DATA_DIRS.as_ref());
        let data_dirs = std::env::split_paths(data_dirs);

        let dirs = std::iter::chain(data_home, data_dirs)
            .filter(|path| path.is_absolute())
            .map(|path| path.join("bitterasm"));

        roots.extend(dirs);
    }

    roots
}

/// A file's module path, the name `.em` identifies its declarations by
/// (`std.binary` for `std/binary.basm`): its path relative to the deepest
/// search root that contains it, with `.` between segments and no
/// extension. The deepest root, not the first: with the working directory
/// at `~`, `~/.bitterasm/std/binary.basm` is `std.binary` (how it's
/// imported), not `.bitterasm.std.binary`. A file under no root is named
/// relative to the working directory, with one leading `.` per step up
/// plus one — the spelling a relative import from there would use
/// (`..shared.util` for `../shared/util.basm`).
pub fn module_path_of(path: &Path) -> String {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    let deepest = search_roots()
        .into_iter()
        .filter_map(|root| fs::canonicalize(&root).ok())
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count());

    if let Some(root) = deepest {
        return dotted(path.strip_prefix(&root).expect("filtered on starts_with"));
    }

    let cwd = std::env::current_dir()
        .and_then(fs::canonicalize)
        .unwrap_or_else(|_| PathBuf::from("."));
    let common = cwd
        .components()
        .zip(path.components())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = cwd.components().count() - common;
    let rest: PathBuf = path.components().skip(common).collect();

    format!("{}{}", ".".repeat(ups + 1), dotted(&rest))
}

// `a/b/c.basm` -> `a.b.c`
fn dotted(relative: &Path) -> String {
    relative
        .with_extension("")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(".")
}

fn module_base_dirs(module: &ModulePath, importer: &Path) -> Vec<PathBuf> {
    if module.relative_level == 0 {
        search_roots()
    } else {
        let mut dir = importer
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        for _ in 1..module.relative_level {
            dir = dir.parent().map(Path::to_path_buf).unwrap_or(dir);
        }

        vec![dir]
    }
}

// The first `<base>/<segments...>[/<name>].basm` that exists, across every
// base directory `module` may live under.
fn find_module_file(
    module: &ModulePath,
    name: Option<&str>,
    importer: &Path,
) -> Result<PathBuf, LoadError> {
    let bases = module_base_dirs(module, importer);

    for base in &bases {
        let mut candidate = base.clone();

        for segment in &module.segments {
            candidate.push(segment);
        }

        if let Some(name) = name {
            candidate.push(name);
        }

        candidate.set_extension("basm");

        if let Ok(path) = fs::canonicalize(&candidate) {
            return Ok(path);
        }
    }

    let display = match name {
        Some(name) => format!("{}.{name}", module_display(module)),
        None => module_display(module),
    };

    Err(LoadError::ModuleNotFound {
        importer: importer.to_path_buf(),
        module: display,
        searched: if module.relative_level == 0 { bases } else { Vec::new() },
    })
}

/// The file an import's module path names, resolved the same way the loader
/// itself resolves it (see `search_roots`).
pub fn resolve_module_path(module: &ModulePath, importer: &Path) -> Result<PathBuf, LoadError> {
    find_module_file(module, None, importer)
}

// `from <package> import <name>` where `<package>` is a directory rather
// than a file (`resolve_module_path` fails on it): treats `<name>` as a
// submodule one level under `<package>`, i.e. `<package>/<name>.basm` —
// `from std import u8string` reaching for `std/u8string.basm`, sugar for
// `from std.u8string import *` (see `splice_import`). Only ever tried as a
// fallback after the plain-file resolution already failed.
fn resolve_submodule_path(
    module: &ModulePath,
    name: &str,
    importer: &Path,
) -> Result<PathBuf, LoadError> {
    find_module_file(module, Some(name), importer)
}

fn canonicalize(path: &Path) -> Result<PathBuf, LoadError> {
    fs::canonicalize(path).map_err(|error| LoadError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn module_display(module: &ModulePath) -> String {
    format!(
        "{}{}",
        ".".repeat(module.relative_level),
        module.segments.join("."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // A declaration's name as its file wrote it, without the internal
    // `#module` suffix the loader gives it.
    fn declared(name: &[NamePart]) -> Option<String> {
        literal_name(name).map(|name| demangle(&name))
    }

    fn lookup_declared(symbols: &crate::resolver::SymbolTable, name: &str) -> Option<crate::resolver::SymbolId> {
        symbols.iter().find(|symbol| demangle(&symbol.name) == name).map(|symbol| symbol.id)
    }

    // `bits<width>`'s `width` argument is an identifier, which only parses
    // as a const (rather than a type) argument if the parser already knows
    // `bits`'s generic signature by the time it reaches that line. Loading
    // this fixture end to end proves the signature made it across the
    // `from std.binary.native import *` boundary correctly.
    #[test]
    fn loads_fields_fixture_and_resolves_bits_via_import() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/riscv/fields.basm");

        let program = load_program(&path).expect("fields.basm should load");

        let bits_struct = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Struct(decl) if declared(&decl.name).as_deref() == Some("bits") => Some(decl),
                _ => None,
            })
            .expect("bits struct should be spliced in from std.binary.native");

        assert_eq!(bits_struct.fields.len(), 1);

        let symbols = crate::resolver::collect_symbols(&program, &vec![0; program.statements.len()])
            .expect("symbol collection should succeed");

        let no_consts = HashMap::new();
        let mut alias_resolver =
            crate::resolver::AliasResolver::new_single_pass(&program, &symbols, &no_consts);

        alias_resolver
            .resolve_all_structs()
            .expect("struct fields should resolve, including bits<width>");
    }

    // `bits<8>`, `bits<4 + 4>`, and `bits<2 * 4>` all denote the same type —
    // this only holds once generic const arguments are actually evaluated
    // rather than compared as unevaluated expression trees, which is what
    // this test locks in.
    #[test]
    fn equivalent_const_generic_arguments_resolve_to_the_same_type() {
        let dir = scratch_dir("const_generic_type_identity");

        fs::write(
            dir.join("a.basm"),
            concat!(
                "from std.binary import *\n\n",
                "type A = bits<8>\n",
                "type B = bits<4 + 4>\n",
                "type C = bits<2 * 4>\n",
                "type D = bits<9>\n",
            ),
        )
        .unwrap();

        let program = load_program(&dir.join("a.basm")).expect("a.basm should load");

        let symbols = crate::resolver::collect_symbols(&program, &vec![0; program.statements.len()])
            .expect("symbol collection should succeed");

        let consts = crate::resolver::ConstEvaluator::new(&program, &symbols)
            .evaluate_all()
            .expect("const evaluation should succeed");

        let consts_by_name: HashMap<String, crate::eval::Int> = consts
            .iter()
            .map(|(id, value)| (symbols.get(*id).name.clone(), value.clone()))
            .collect();

        let mut alias_resolver =
            crate::resolver::AliasResolver::new_single_pass(&program, &symbols, &consts_by_name);

        let aliases = alias_resolver
            .resolve_all()
            .expect("aliases should resolve");

        let resolved = |name: &str| {
            let id = lookup_declared(&symbols, name).expect("symbol should exist");
            aliases.get(&id).expect("alias should resolve").clone()
        };

        let a = resolved("A");
        let b = resolved("B");
        let c = resolved("C");
        let d = resolved("D");

        assert_eq!(a, b, "bits<8> and bits<4 + 4> should be the same type");
        assert_eq!(a, c, "bits<8> and bits<2 * 4> should be the same type");
        assert_ne!(a, d, "bits<8> and bits<9> should be different types");

        fs::remove_dir_all(&dir).ok();
    }

    // `from pkgdir import sub`, where `pkgdir` is a directory (not a file —
    // `pkgdir.basm` doesn't exist) and `sub` is `pkgdir/sub.basm` — sugar
    // for `from pkgdir.sub import *`, per `resolve_import_paths`'s
    // `ImportResolution::Package` case.
    #[test]
    fn package_style_import_resolves_a_submodule_directory() {
        let dir = scratch_dir("package_style_import");
        fs::create_dir_all(dir.join("pkgdir")).unwrap();

        fs::write(
            dir.join("pkgdir").join("sub.basm"),
            "pub struct TheStruct {\n    value: int\n}\n\npub const the_const: int = 42\n",
        )
        .unwrap();

        fs::write(
            dir.join("importer.basm"),
            "from .pkgdir import sub\n\nconst v = the_const\n",
        )
        .unwrap();

        let program = load_program(&dir.join("importer.basm")).expect("importer.basm should load");

        assert!(program.statements.iter().any(
            |statement| matches!(statement, Statement::Struct(decl) if declared(&decl.name).as_deref() == Some("TheStruct"))
        ));

        assert!(program.statements.iter().any(
            |statement| matches!(statement, Statement::Const(decl) if declared(&decl.name).as_deref() == Some("the_const"))
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn diamond_import_does_not_duplicate_declarations() {
        // Both fields.basm and std.riscv.native pull in std.binary
        // independently. If the
        // loader didn't dedupe by canonical path; collect_symbols would then
        // fail on a duplicate `bits`.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/riscv/mini.basm");

        let program = load_program(&path).expect("mini.basm should load");

        let bits_count = program
            .statements
            .iter()
            .filter(|statement| matches!(statement, Statement::Struct(decl) if declared(&decl.name).as_deref() == Some("bits")))
            .count();

        assert_eq!(bits_count, 1);
    }

    #[test]
    fn module_origins_attribute_entry_and_spliced_statements_to_different_modules() {
        let dir = scratch_dir("module_origins");

        fs::write(dir.join("helper.basm"), "pub struct Helper { x: int }\n").unwrap();
        fs::write(
            dir.join("main.basm"),
            "from .helper import *\n\npub struct Main { y: int }\n",
        )
        .unwrap();

        let (program, origins) =
            load_program_with_modules(&dir.join("main.basm")).expect("main.basm should load");

        let helper_index = program
            .statements
            .iter()
            .position(|s| matches!(s, Statement::Struct(decl) if declared(&decl.name).as_deref() == Some("Helper")))
            .expect("Helper should be present");

        let main_index = program
            .statements
            .iter()
            .position(|s| matches!(s, Statement::Struct(decl) if declared(&decl.name).as_deref() == Some("Main")))
            .expect("Main should be present");

        let helper_module = origins.module_of(helper_index);
        let main_module = origins.module_of(main_index);

        assert_ne!(helper_module, main_module);
        assert_eq!(origins.path(helper_module), dir.join("helper.basm"));
        assert_eq!(origins.path(main_module), dir.join("main.basm"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn import_star_carries_a_top_level_for_across_the_module_boundary() {
        // `regs.basm`'s declarations only exist inside a `@for`, not as
        // bare top-level statements. The loader unrolls each module before
        // naming its declarations, so they still reach the importer.
        let dir = scratch_dir("import_star_carries_top_level_for");

        fs::write(
            dir.join("regs.basm"),
            "@for i in 0..3 {\n    pub const x`i` = i\n}\n",
        )
        .unwrap();
        fs::write(dir.join("main.basm"), "from .regs import *\n").unwrap();

        let program = load_program(&dir.join("main.basm")).expect("main.basm should load");
        let module_of = vec![0; program.statements.len()];
        let (program, _) =
            crate::resolver::unroll_top_level(program, &module_of).expect("should unroll");

        let const_names: Vec<String> = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Const(decl) => declared(&decl.name),
                _ => None,
            })
            .collect();

        assert_eq!(const_names, vec!["x0", "x1", "x2"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detects_cyclic_import() {
        let dir = scratch_dir("cyclic_import");

        fs::write(dir.join("a.basm"), "from .b import *\n").unwrap();
        fs::write(dir.join("b.basm"), "from .a import *\n").unwrap();

        let result = load_program(&dir.join("a.basm"));

        assert!(matches!(result, Err(LoadError::CyclicImport { .. })));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reports_missing_module() {
        let dir = scratch_dir("missing_module");

        fs::write(dir.join("a.basm"), "from .does_not_exist import *\n").unwrap();

        let result = load_program(&dir.join("a.basm"));

        assert!(matches!(result, Err(LoadError::ModuleNotFound { .. })));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn private_declaration_is_unreachable_outside_its_module() {
        let dir = scratch_dir("private_unreachable");

        fs::write(
            dir.join("priv.basm"),
            "struct Helper { x: int }\n\npub struct Public { y: Helper }\n",
        )
        .unwrap();

        // Using `Public` from another file should resolve fine: `Public`'s
        // own field type `Helper` is private, but that's an internal detail
        // of `priv.basm`, not something the importer needs to spell.
        fs::write(
            dir.join("uses_public.basm"),
            "from .priv import *\n\ntype Alias = Public\n",
        )
        .unwrap();

        let program = load_program(&dir.join("uses_public.basm"))
            .expect("uses_public.basm should load");

        let symbols = crate::resolver::collect_symbols(&program, &vec![0; program.statements.len()])
            .expect("symbol collection should succeed");

        crate::resolver::AliasResolver::new_single_pass(&program, &symbols, &HashMap::new())
            .resolve_all()
            .expect("Alias should resolve through Public down to the private Helper field");

        // But typing `Helper` directly from outside `priv.basm` should not
        // resolve to anything: its name was mangled away during splicing.
        fs::write(
            dir.join("uses_private.basm"),
            "from .priv import *\n\ntype Bad = Helper\n",
        )
        .unwrap();

        let program = load_program(&dir.join("uses_private.basm"))
            .expect("uses_private.basm should load (privacy is a resolve-time concern)");

        let symbols = crate::resolver::collect_symbols(&program, &vec![0; program.statements.len()])
            .expect("symbol collection should succeed");

        let result =
            crate::resolver::AliasResolver::new_single_pass(&program, &symbols, &HashMap::new()).resolve_all();

        assert!(
            matches!(result, Err(crate::resolver::ResolveError::UnknownType { .. })),
            "expected Helper to be unresolvable from outside priv.basm, got {result:?}",
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_pub_struct_field_is_unreachable_from_outside_its_module() {
        // `private_declaration_is_unreachable_outside_its_module`'s
        // counterpart one level down: `Foo` itself is `pub` and perfectly
        // reachable from `main.basm`, but its `x` field isn't — reading
        // `.x` on a `Foo` value from a module other than the one that
        // declared `Foo` is rejected, even though nothing about naming
        // `Foo` or constructing/receiving a value of it was a problem.
        let dir = scratch_dir("private_field_cross_module");

        fs::write(dir.join("helper.basm"), "pub struct Foo { x: int }\n").unwrap();
        fs::write(
            dir.join("main.basm"),
            "from .helper import *\n\nmacro reads_x(f: Foo) -> int {\n    @return f.x\n}\n",
        )
        .unwrap();

        let (program, origins) =
            load_program_with_modules(&dir.join("main.basm")).expect("main.basm should load");
        let (program, statement_modules) =
            crate::resolver::unroll_top_level(program, origins.all()).expect("should unroll");
        let symbols = crate::resolver::collect_symbols(&program, &statement_modules)
            .expect("symbol collection should succeed");

        let macro_symbol = lookup_declared(&symbols, "reads_x").unwrap();
        let foo_symbol = lookup_declared(&symbols, "Foo").unwrap();
        let declaration = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Macro(decl) if declared(&decl.name).as_deref() == Some("reads_x") => {
                    Some(decl.clone())
                }
                _ => None,
            })
            .expect("reads_x should be in the merged program");

        let consts = HashMap::new();
        let mut resolver = crate::resolver::AliasResolver::new(
            &program,
            &symbols,
            &consts,
            crate::resolver::LabelMode::Strict,
            HashMap::new(),
            origins.entry_module(),
        );

        let arg = crate::resolver::Value::Struct {
            symbol: foo_symbol,
            args: vec![],
            fields: vec![("x".to_string(), crate::resolver::Value::Int(crate::eval::Int::from(1)))],
            nominal: None,
        };

        let mut stack = Vec::new();
        let error = resolver
            .run_macro_body(macro_symbol, &declaration, vec![arg], &mut stack)
            .unwrap_err();

        assert!(matches!(
            error,
            crate::resolver::ResolveError::PrivateFieldAccess { field, type_name, .. }
                if field == "x" && demangle(&type_name) == "Foo"
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_pub_struct_field_is_reachable_from_its_own_module() {
        // Positive control for the test above: the exact same shape, but
        // `Foo` and `reads_x` both live in the one file that declares `Foo`
        // — same-module access to a non-`pub` field is unaffected.
        let dir = scratch_dir("private_field_same_module");

        fs::write(
            dir.join("main.basm"),
            "struct Foo { x: int }\n\nmacro reads_x(f: Foo) -> int {\n    @return f.x\n}\n",
        )
        .unwrap();

        let (program, origins) =
            load_program_with_modules(&dir.join("main.basm")).expect("main.basm should load");
        let (program, statement_modules) =
            crate::resolver::unroll_top_level(program, origins.all()).expect("should unroll");
        let symbols = crate::resolver::collect_symbols(&program, &statement_modules)
            .expect("symbol collection should succeed");

        let macro_symbol = lookup_declared(&symbols, "reads_x").unwrap();
        let foo_symbol = lookup_declared(&symbols, "Foo").unwrap();
        let declaration = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Macro(decl) if declared(&decl.name).as_deref() == Some("reads_x") => {
                    Some(decl.clone())
                }
                _ => None,
            })
            .expect("reads_x should be in the program");

        let consts = HashMap::new();
        let mut resolver = crate::resolver::AliasResolver::new(
            &program,
            &symbols,
            &consts,
            crate::resolver::LabelMode::Strict,
            HashMap::new(),
            origins.entry_module(),
        );

        let arg = crate::resolver::Value::Struct {
            symbol: foo_symbol,
            args: vec![],
            fields: vec![("x".to_string(), crate::resolver::Value::Int(crate::eval::Int::from(1)))],
            nominal: None,
        };

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(macro_symbol, &declaration, vec![arg], &mut stack);

        assert!(result.is_ok(), "expected same-module field access to succeed, got {result:?}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_pub_struct_field_cannot_be_named_in_construction_from_outside_its_module() {
        // The write side of the same rule: naming a non-`pub` field
        // explicitly in a brace-literal construction is rejected from
        // outside `Foo`'s own module too, not just reading it back out
        // afterward.
        let dir = scratch_dir("private_field_write_cross_module");

        fs::write(dir.join("helper.basm"), "pub struct Foo { x: int }\n").unwrap();
        fs::write(
            dir.join("main.basm"),
            "from .helper import *\n\nmacro makes_foo() -> Foo {\n    @return Foo { x: 1 }\n}\n",
        )
        .unwrap();

        let (program, origins) =
            load_program_with_modules(&dir.join("main.basm")).expect("main.basm should load");
        let (program, statement_modules) =
            crate::resolver::unroll_top_level(program, origins.all()).expect("should unroll");
        let symbols = crate::resolver::collect_symbols(&program, &statement_modules)
            .expect("symbol collection should succeed");

        let macro_symbol = lookup_declared(&symbols, "makes_foo").unwrap();
        let declaration = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Macro(decl) if declared(&decl.name).as_deref() == Some("makes_foo") => {
                    Some(decl.clone())
                }
                _ => None,
            })
            .expect("makes_foo should be in the merged program");

        let consts = HashMap::new();
        let mut resolver = crate::resolver::AliasResolver::new(
            &program,
            &symbols,
            &consts,
            crate::resolver::LabelMode::Strict,
            HashMap::new(),
            origins.entry_module(),
        );

        let mut stack = Vec::new();
        let error = resolver
            .run_macro_body(macro_symbol, &declaration, vec![], &mut stack)
            .unwrap_err();

        assert!(matches!(
            error,
            crate::resolver::ResolveError::PrivateFieldAccess { field, type_name, .. }
                if field == "x" && demangle(&type_name) == "Foo"
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn named_import_of_private_declaration_is_rejected() {
        let dir = scratch_dir("private_named_import");

        fs::write(
            dir.join("priv.basm"),
            "struct Helper { x: int }\n\npub struct Public { y: Helper }\n",
        )
        .unwrap();

        fs::write(dir.join("a.basm"), "from .priv import Helper\n").unwrap();

        let result = load_program(&dir.join("a.basm"));

        assert!(
            matches!(result, Err(LoadError::UnknownImportedName { .. })),
            "expected importing a private name by name to fail, got {result:?}",
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn private_declarations_with_the_same_name_do_not_collide_across_modules() {
        let dir = scratch_dir("private_no_collision");

        fs::write(
            dir.join("a.basm"),
            "struct Helper { x: int }\n\npub struct A { h: Helper }\n",
        )
        .unwrap();

        fs::write(
            dir.join("b.basm"),
            "struct Helper { y: int }\n\npub struct B { h: Helper }\n",
        )
        .unwrap();

        fs::write(
            dir.join("importer.basm"),
            "from .a import *\nfrom .b import *\n\ntype UsesA = A\ntype UsesB = B\n",
        )
        .unwrap();

        let program = load_program(&dir.join("importer.basm"))
            .expect("importer.basm should load");

        // Without per-module mangling both files' `Helper` would land in the
        // same flat symbol table under the same name and collide here.
        let symbols = crate::resolver::collect_symbols(&program, &vec![0; program.statements.len()])
            .expect("both private Helpers should coexist without a duplicate-symbol error");

        let no_consts = HashMap::new();
        let mut alias_resolver =
            crate::resolver::AliasResolver::new_single_pass(&program, &symbols, &no_consts);

        alias_resolver
            .resolve_all()
            .expect("UsesA and UsesB should each resolve through their own module's Helper");

        // Each struct's `h: Helper` field should resolve against its own
        // module's (distinctly mangled) Helper, not the other module's.
        alias_resolver
            .resolve_all_structs()
            .expect("A.h and B.h should each resolve to their own module's Helper field");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn custom_syntax_macro_resolves_via_import() {
        let dir = scratch_dir("custom_syntax_import");

        fs::write(
            dir.join("producer.basm"),
            "pub macro mov(dst: int, value: int) | syntax { mov $dst$, $value$ } {\n}\n",
        )
        .unwrap();

        fs::write(
            dir.join("importer.basm"),
            "from .producer import *\n\nmov r1, 7\n",
        )
        .unwrap();

        let program = load_program(&dir.join("importer.basm"))
            .expect("importer.basm should load, using producer's custom mov syntax");

        let invocation = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Invocation(invocation) if demangle(&invocation.name) == "mov" => Some(invocation),
                _ => None,
            })
            .expect("expected a mov invocation");

        assert_eq!(invocation.operands.len(), 2);

        assert!(matches!(
            &invocation.operands[0],
            Expr::Identifier { name, .. } if name == "r1"
        ));

        assert!(matches!(
            &invocation.operands[1],
            Expr::Integer { raw, .. } if raw == "7"
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn overloaded_custom_syntaxes_merge_across_imports() {
        let dir = scratch_dir("overloaded_custom_syntax_import");

        fs::write(
            dir.join("bracketed.basm"),
            "pub macro load(address: int) | syntax { load [$address$] } {\n}\n",
        )
        .unwrap();
        fs::write(
            dir.join("displaced.basm"),
            "pub macro load(base: int, offset: int) | syntax { load $offset$($base$) } {\n}\n",
        )
        .unwrap();
        fs::write(
            dir.join("importer.basm"),
            "from .bracketed import *\nfrom .displaced import *\n\nload [123]\nload 8(sp)\n",
        )
        .unwrap();

        let program = load_program(&dir.join("importer.basm"))
            .expect("both imported load syntaxes should remain registered");
        let invocations: Vec<_> = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) if demangle(&invocation.name) == "load" => Some(invocation),
                _ => None,
            })
            .collect();

        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].operands.len(), 1);
        assert_eq!(invocations[1].operands.len(), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn custom_syntax_call_site_must_follow_its_declaration_in_the_same_file() {
        let dir = scratch_dir("custom_syntax_ordering");

        // A pattern whose separator (`:`) genuinely can't parse as any kind
        // of default-expression continuation, so a same-file, out-of-order
        // call site hard-fails during the prepass rather than "accidentally"
        // still working because the misinterpreted default parse happens to
        // consume the same tokens anyway (which does happen for some
        // separators, e.g. `<-`, since `<` and unary `-` are both valid
        // default-expression continuations even though they're wrong here —
        // this test specifically needs one that isn't).
        fs::write(
            dir.join("after.basm"),
            "macro mov(dst: int, value: int) | syntax { mov $dst$: $value$ } {\n}\n\nmov r1: 7\n",
        )
        .unwrap();

        load_program(&dir.join("after.basm"))
            .expect("a custom-syntax call site after its own declaration should load");

        fs::write(
            dir.join("before.basm"),
            "mov r1: 7\n\nmacro mov(dst: int, value: int) | syntax { mov $dst$: $value$ } {\n}\n",
        )
        .unwrap();

        let result = load_program(&dir.join("before.basm"));

        assert!(
            result.is_err(),
            "a custom-syntax call site before its own declaration is a known v1 \
             limitation — document it explicitly (this test) rather than relying \
             on it silently working or silently failing"
        );

        fs::remove_dir_all(&dir).ok();
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bitterasm-loader-test-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
