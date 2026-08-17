// Resolves `from ... import ...` statements to files on disk and flattens
// them into a single, self-contained `Program` before the resolver ever
// sees it. The resolver and symbol table stay import-agnostic: by the time
// they run, every declaration a file depends on already lives directly in
// its statement list.
//
// Module paths map onto the filesystem 1:1: `std.binary.native` is
// `<root>/std/binary/native.basm`, where `<root>` is the current working
// directory for absolute imports (no leading dots) or the importing file's
// own directory (ascended once per extra leading dot) for relative ones.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Expr, ImportItems, ImportStatement, ModulePath, Program, Statement};
use crate::lexer;
use crate::parser::{self, GenericParamKind};
use crate::token::Span;
use crate::types::{GenericParameter, TypeArgument, TypeExpr};

#[derive(Debug)]
pub enum LoadError {
    Io {
        path: PathBuf,
        message: String,
    },
    Lex {
        path: PathBuf,
        message: String,
    },
    Parse {
        path: PathBuf,
        message: String,
    },
    ModuleNotFound {
        importer: PathBuf,
        module: String,
    },
    CyclicImport {
        cycle: Vec<PathBuf>,
    },
    UnknownImportedName {
        module: String,
        name: String,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io { path, message } => {
                write!(f, "failed to read {}: {message}", path.display())
            }

            LoadError::Lex { path, message } => {
                write!(f, "{}: {message}", path.display())
            }

            LoadError::Parse { path, message } => {
                write!(f, "{}: {message}", path.display())
            }

            LoadError::ModuleNotFound { importer, module } => {
                write!(
                    f,
                    "{}: could not find module `{module}`",
                    importer.display(),
                )
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
        }
    }
}

impl std::error::Error for LoadError {}

// A fully parsed module, kept around by canonical path so a module imported
// from multiple places is only read and parsed once.
struct LoadedModule {
    statements: Vec<Statement>,
    span: Span,

    // Signatures visible by the end of this file: its own struct/type-alias
    // declarations plus everything transitively pulled in by its own
    // imports. Handed to files that import this module, so they can parse
    // `bits<width>`-shaped usages correctly without the declaration itself
    // being textually present. Private (non-pub) signatures declared by
    // this file itself are filtered out before caching, since a name an
    // importer can never write shouldn't shadow how they interpret an
    // unrelated same-named generic of their own.
    signatures: HashMap<String, Vec<GenericParamKind>>,

    // Stable, unique-per-module id assigned the first time this module is
    // loaded. Used to mangle its private declarations into names that can't
    // collide with (or be typed by) any other module's when everything is
    // flattened into one global program.
    module_id: usize,
}

pub fn load_program(entry: &Path) -> Result<Program, LoadError> {
    let entry_path = canonicalize(entry)?;

    let mut cache: HashMap<PathBuf, LoadedModule> = HashMap::new();
    let mut stack: Vec<PathBuf> = Vec::new();

    load_module(&entry_path, &mut cache, &mut stack)?;

    // The entry file's own imports get spliced into its statement list; it
    // is never itself spliced into anything, so seed the "already included"
    // set with just itself.
    let mut spliced: HashSet<PathBuf> = HashSet::new();
    spliced.insert(entry_path.clone());

    let entry_module = &cache[&entry_path];
    let span = entry_module.span;

    let mut statements = Vec::new();

    for statement in &entry_module.statements {
        match statement {
            Statement::Import(import) => {
                statements.extend(splice_import(import, &entry_path, &cache, &mut spliced)?);
            }

            other => statements.push(other.clone()),
        }
    }

    Ok(Program { statements, span })
}

fn load_module(
    path: &Path,
    cache: &mut HashMap<PathBuf, LoadedModule>,
    stack: &mut Vec<PathBuf>,
) -> Result<(), LoadError> {
    if cache.contains_key(path) {
        return Ok(());
    }

    if let Some(start) = stack.iter().position(|p| p == path) {
        let mut cycle: Vec<PathBuf> = stack[start..].to_vec();
        cycle.push(path.to_path_buf());

        return Err(LoadError::CyclicImport { cycle });
    }

    let source = fs::read_to_string(path).map_err(|error| LoadError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let tokens = lexer::lex(&source).map_err(|error| LoadError::Lex {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let imports = parser::discover_imports(tokens.clone());

    stack.push(path.to_path_buf());

    let mut seed: HashMap<String, Vec<GenericParamKind>> = HashMap::new();

    for import in &imports {
        let child_path = resolve_module_path(&import.module, path)?;
        load_module(&child_path, cache, stack)?;
        seed.extend(cache[&child_path].signatures.clone());
    }

    let (program, mut signatures) = parser::parse_seeded(tokens, &seed).map_err(|error| {
        LoadError::Parse {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    })?;

    stack.pop();

    let own_private_names: HashSet<&str> = program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Struct(decl) if !decl.is_pub => Some(decl.name.as_str()),
            Statement::TypeAlias(decl) if !decl.is_pub => Some(decl.name.as_str()),
            _ => None,
        })
        .collect();

    signatures.retain(|name, _| !own_private_names.contains(name.as_str()));

    let module_id = cache.len();

    cache.insert(
        path.to_path_buf(),
        LoadedModule {
            statements: program.statements,
            span: program.span,
            signatures,
            module_id,
        },
    );

    Ok(())
}

// Resolves one `from <module> import <items>` statement to the declarations
// it brings in, recursing into that module's own imports. Modules already
// spliced elsewhere in this build (diamond imports) contribute nothing the
// second time, since their declarations are already present.
fn splice_import(
    import: &ImportStatement,
    importer: &Path,
    cache: &HashMap<PathBuf, LoadedModule>,
    spliced: &mut HashSet<PathBuf>,
) -> Result<Vec<Statement>, LoadError> {
    let target_path = resolve_module_path(&import.module, importer)?;

    if let ImportItems::Names(names) = &import.items {
        let target = &cache[&target_path];
        let declared: HashSet<&str> = target
            .statements
            .iter()
            .filter_map(declaration_name)
            .filter(|(_, is_pub)| *is_pub)
            .map(|(name, _)| name)
            .collect();

        for name in names {
            if !declared.contains(name.as_str()) {
                return Err(LoadError::UnknownImportedName {
                    module: module_display(&import.module),
                    name: name.clone(),
                });
            }
        }
    }

    // The names check above only gates what an importer is allowed to ask
    // for by name; everything the target transitively needs still has to be
    // spliced in for resolution to work. Non-pub declarations are mangled
    // (see `collect_declarations`) so they're never nameable outside the
    // module that declared them, regardless of how much gets spliced.
    let mut out = Vec::new();
    collect_declarations(&target_path, cache, spliced, &mut out)?;
    Ok(out)
}

fn collect_declarations(
    path: &Path,
    cache: &HashMap<PathBuf, LoadedModule>,
    spliced: &mut HashSet<PathBuf>,
    out: &mut Vec<Statement>,
) -> Result<(), LoadError> {
    if !spliced.insert(path.to_path_buf()) {
        return Ok(());
    }

    let module = &cache[path];

    // Non-pub struct/type-alias/const declarations are private to this
    // file: mangle them to a name no other file's source text could ever
    // spell, and rewrite every reference to them within this file's own
    // declarations to match. Declarations from other files are untouched
    // here; each gets its own independent rename pass at its own splice.
    let renames = build_rename_map(&module.statements, module.module_id);

    for statement in &module.statements {
        match statement {
            Statement::Import(nested) => {
                let nested_path = resolve_module_path(&nested.module, path)?;
                collect_declarations(&nested_path, cache, spliced, out)?;
            }

            Statement::Struct(_) | Statement::TypeAlias(_) | Statement::Const(_) => {
                let mut declaration = statement.clone();
                rename_statement(&mut declaration, &renames);
                out.push(declaration);
            }

            // Labels, invocations, and meta statements are program bodies,
            // not declarations; nothing else imports them.
            Statement::Label(_) | Statement::Invocation(_) | Statement::Meta(_) => {}
        }
    }

    Ok(())
}

// A name and whether it's reachable from outside the file that declared it.
fn declaration_name(statement: &Statement) -> Option<(&str, bool)> {
    match statement {
        Statement::Struct(decl) => Some((&decl.name, decl.is_pub)),
        Statement::TypeAlias(decl) => Some((&decl.name, decl.is_pub)),
        Statement::Const(decl) => Some((&decl.name, decl.is_pub)),
        _ => None,
    }
}

// ===============
// privacy mangling
// ===============
//
// Non-pub declarations are only ever meant to be visible within the file
// that declares them. Since the resolver works over one flattened, global
// program with a single flat (by-name) symbol table, "private" can't mean
// "absent from the table" (a pub sibling may still depend on it) — instead
// it means "renamed to something no source text could ever spell", using
// `#`, which the lexer only ever treats as the start of a line comment and
// so can never appear inside an identifier a `.basm` file actually wrote.

fn build_rename_map(statements: &[Statement], module_id: usize) -> HashMap<String, String> {
    let mut renames = HashMap::new();

    for statement in statements {
        if let Some((name, is_pub)) = declaration_name(statement) {
            if !is_pub {
                renames.insert(name.to_string(), format!("{name}#{module_id}"));
            }
        }
    }

    renames
}

fn rename_statement(statement: &mut Statement, renames: &HashMap<String, String>) {
    match statement {
        Statement::Struct(decl) => {
            if let Some(mangled) = renames.get(&decl.name) {
                decl.name = mangled.clone();
            }

            for param in &mut decl.generic_params {
                rename_generic_parameter(param, renames);
            }

            for field in &mut decl.fields {
                rename_type_expr(&mut field.ty, renames);
            }
        }

        Statement::TypeAlias(decl) => {
            if let Some(mangled) = renames.get(&decl.name) {
                decl.name = mangled.clone();
            }

            for param in &mut decl.generic_params {
                rename_generic_parameter(param, renames);
            }

            rename_type_expr(&mut decl.ty, renames);
        }

        Statement::Const(decl) => {
            if let Some(mangled) = renames.get(&decl.name) {
                decl.name = mangled.clone();
            }

            if let Some(ty) = &mut decl.ty {
                rename_type_expr(ty, renames);
            }

            rename_expr(&mut decl.value, renames);
        }

        Statement::Import(_) | Statement::Label(_) | Statement::Invocation(_) | Statement::Meta(_) => {}
    }
}

fn rename_generic_parameter(param: &mut GenericParameter, renames: &HashMap<String, String>) {
    if let GenericParameter::Const { ty, .. } = param {
        rename_type_expr(ty, renames);
    }
}

fn rename_type_expr(ty: &mut TypeExpr, renames: &HashMap<String, String>) {
    match ty {
        TypeExpr::Named { path, .. } => {
            if path.len() == 1 {
                if let Some(mangled) = renames.get(&path[0]) {
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
                }
            }
        }
    }
}

fn rename_expr(expr: &mut Expr, renames: &HashMap<String, String>) {
    match expr {
        Expr::Identifier { name, .. } => {
            if let Some(mangled) = renames.get(name) {
                *name = mangled.clone();
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

        Expr::Unary { operand, .. } => rename_expr(operand, renames),

        Expr::Binary { left, right, .. } => {
            rename_expr(left, renames);
            rename_expr(right, renames);
        }
    }
}

fn resolve_module_path(module: &ModulePath, importer: &Path) -> Result<PathBuf, LoadError> {
    let base = if module.relative_level == 0 {
        std::env::current_dir().map_err(|error| LoadError::Io {
            path: importer.to_path_buf(),
            message: error.to_string(),
        })?
    } else {
        let mut dir = importer
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        for _ in 1..module.relative_level {
            dir = dir.parent().map(Path::to_path_buf).unwrap_or(dir);
        }

        dir
    };

    let mut candidate = base;

    for segment in &module.segments {
        candidate.push(segment);
    }

    candidate.set_extension("basm");

    canonicalize(&candidate).map_err(|_| LoadError::ModuleNotFound {
        importer: importer.to_path_buf(),
        module: module_display(module),
    })
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

    // `bits<width>`'s `width` argument is an identifier, which only parses
    // as a const (rather than a type) argument if the parser already knows
    // `bits`'s generic signature by the time it reaches that line. Loading
    // this fixture end to end proves the signature made it across the
    // `from std.binary.native import *` boundary correctly.
    #[test]
    fn loads_fields_fixture_and_resolves_bits_via_import() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tinycpu/fields.basm");

        let program = load_program(&path).expect("fields.basm should load");

        let bits_struct = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Struct(decl) if decl.name == "bits" => Some(decl),
                _ => None,
            })
            .expect("bits struct should be spliced in from std.binary.native");

        assert_eq!(bits_struct.fields.len(), 1);

        let symbols = crate::resolver::collect_symbols(&program)
            .expect("symbol collection should succeed");

        let mut alias_resolver = crate::resolver::AliasResolver::new(&program, &symbols);

        alias_resolver
            .resolve_all_structs()
            .expect("struct fields should resolve, including bits<width>");
    }

    #[test]
    fn diamond_import_does_not_duplicate_declarations() {
        // Both fields.basm and its (transitive, via std.tinycpu.native)
        // sibling would each pull in std.binary.native independently if the
        // loader didn't dedupe by canonical path; collect_symbols would then
        // fail on a duplicate `bits`.
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tinycpu/mini.basm");

        let program = load_program(&path).expect("mini.basm should load");

        let bits_count = program
            .statements
            .iter()
            .filter(|statement| matches!(statement, Statement::Struct(decl) if decl.name == "bits"))
            .count();

        assert_eq!(bits_count, 1);
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

        let symbols = crate::resolver::collect_symbols(&program)
            .expect("symbol collection should succeed");

        crate::resolver::AliasResolver::new(&program, &symbols)
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

        let symbols = crate::resolver::collect_symbols(&program)
            .expect("symbol collection should succeed");

        let result = crate::resolver::AliasResolver::new(&program, &symbols).resolve_all();

        assert!(
            matches!(result, Err(crate::resolver::ResolveError::UnknownType { .. })),
            "expected Helper to be unresolvable from outside priv.basm, got {result:?}",
        );

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
        let symbols = crate::resolver::collect_symbols(&program)
            .expect("both private Helpers should coexist without a duplicate-symbol error");

        let mut alias_resolver = crate::resolver::AliasResolver::new(&program, &symbols);

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

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "bitterasm-loader-test-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
