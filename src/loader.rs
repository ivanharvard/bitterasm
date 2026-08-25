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

use crate::ast::{
    literal_name, ConstructItem, Expr, Facet, FacetPayload, ImportItems, ImportStatement,
    MetaStatement, ModulePath, NamePart, Program, Statement,
};
use crate::lexer;
use crate::parser::{self, ParserSeed};
use crate::token::Span;
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

    // Generic signatures and macro syntax patterns visible by the end of
    // this file: its own struct/type-alias/macro declarations plus
    // everything transitively pulled in by its own imports. Handed to
    // files that import this module, so they can parse `bits<width>`-shaped
    // usages, and custom-syntax macro invocations, correctly without the
    // declaration itself being textually present. Private (non-pub)
    // declarations from this file are filtered out of both maps before
    // caching, since a name an importer can never write shouldn't shadow
    // how they interpret an unrelated same-named generic/macro of their
    // own.
    seed: ParserSeed,

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

    let mut seed = ParserSeed::default();

    for import in &imports {
        let child_paths = match resolve_import_paths(import, path)? {
            ImportResolution::Plain(child_path) => vec![child_path],
            ImportResolution::Package(child_paths) => child_paths,
        };

        for child_path in child_paths {
            load_module(&child_path, cache, stack)?;
            let child_seed = &cache[&child_path].seed;
            seed.generic_signatures.extend(child_seed.generic_signatures.clone());
            seed.macro_syntaxes.extend(child_seed.macro_syntaxes.clone());
        }
    }

    let (program, mut seed) = parser::parse_seeded(tokens, &seed).map_err(|error| {
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
            Statement::Macro(decl) if !decl.is_pub => Some(decl.name.as_str()),
            _ => None,
        })
        .collect();

    seed.generic_signatures.retain(|name, _| !own_private_names.contains(name.as_str()));
    seed.macro_syntaxes.retain(|name, _| !own_private_names.contains(name.as_str()));

    let module_id = cache.len();

    cache.insert(
        path.to_path_buf(),
        LoadedModule {
            statements: program.statements,
            span: program.span,
            seed,
            module_id,
        },
    );

    Ok(())
}

// Resolves one `from <module> import <items>` statement to the declarations
// it brings in, recursing into that module's own imports. Modules already
// spliced elsewhere in this build (diamond imports) contribute nothing the
// second time, since their declarations are already present.
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

        (Err(_), ImportItems::Names(names)) => names
            .iter()
            .map(|name| resolve_submodule_path(&import.module, name, importer))
            .collect::<Result<_, _>>()
            .map(ImportResolution::Package),

        (Err(error), ImportItems::All) => Err(error),
    }
}

fn splice_import(
    import: &ImportStatement,
    importer: &Path,
    cache: &HashMap<PathBuf, LoadedModule>,
    spliced: &mut HashSet<PathBuf>,
) -> Result<Vec<Statement>, LoadError> {
    let target_paths: Vec<PathBuf> = match resolve_import_paths(import, importer)? {
        ImportResolution::Plain(target_path) => {
            // Only a plain import's names are "declarations inside one
            // target file" that can be validated this way — a package
            // import's names were already each individually resolved to
            // their own whole file by `resolve_import_paths`, so there's
            // nothing further to check here for those.
            if let ImportItems::Names(names) = &import.items {
                let target = &cache[&target_path];
                let declared: HashSet<String> = target
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

            vec![target_path]
        }

        ImportResolution::Package(target_paths) => target_paths,
    };

    // Everything each target transitively needs still has to be spliced in
    // for resolution to work. Non-pub declarations are mangled (see
    // `collect_declarations`) so they're never nameable outside the module
    // that declared them, regardless of how much gets spliced.
    let mut out = Vec::new();

    for target_path in &target_paths {
        collect_declarations(target_path, cache, spliced, &mut out)?;
    }

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

            Statement::Struct(_)
            | Statement::Enum(_)
            | Statement::TypeAlias(_)
            | Statement::Const(_)
            | Statement::Macro(_) => {
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
// `Const`'s name can in principle carry a splice, but only meaningfully so
// once it's generated from inside a live macro body — a top-level const
// (the only kind this function ever sees; `build_rename_map` doesn't
// descend into macro bodies) is always fully literal, so a non-literal
// name here just means there's nothing to add to the rename map for it —
// `resolver::collect_symbols` is what rejects that case with a real error.
fn declaration_name(statement: &Statement) -> Option<(String, bool)> {
    match statement {
        Statement::Struct(decl) => Some((decl.name.clone(), decl.is_pub)),
        Statement::Enum(decl) => Some((decl.name.clone(), decl.is_pub)),
        Statement::TypeAlias(decl) => Some((decl.name.clone(), decl.is_pub)),
        Statement::Const(decl) => literal_name(&decl.name).map(|name| (name, decl.is_pub)),
        Statement::Macro(decl) => Some((decl.name.clone(), decl.is_pub)),
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
                renames.insert(name.clone(), format!("{name}#{module_id}"));
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

            rename_struct_body_items(&mut decl.fields, renames);

            for facet in &mut decl.facets {
                rename_facet(facet, renames);
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

            for facet in &mut decl.facets {
                rename_facet(facet, renames);
            }
        }

        Statement::Const(decl) => {
            if let Some(literal) = literal_name(&decl.name) {
                if let Some(mangled) = renames.get(&literal) {
                    decl.name = vec![NamePart::Literal(mangled.clone())];
                }
            }

            rename_spliced_name(&mut decl.name, renames);

            if let Some(ty) = &mut decl.ty {
                rename_type_expr(ty, renames);
            }

            rename_expr(&mut decl.value, renames);
        }

        Statement::Macro(decl) => {
            if let Some(mangled) = renames.get(&decl.name) {
                decl.name = mangled.clone();
            }

            for param in &mut decl.params {
                rename_type_expr(&mut param.ty, renames);
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
            if let Some(mangled) = renames.get(&decl.name) {
                decl.name = mangled.clone();
            }
        }

        Statement::Import(_) | Statement::Label(_) | Statement::Invocation(_) => {}
    }
}

// A macro body's `@if`/`@for` can reference this module's own private
// siblings in its condition/range bounds, just like any other statement
// inside the body — so its `args` and nested `body`/`else_body` all need
// the same rewriting a plain statement would get.
fn rename_meta_statement(meta: &mut MetaStatement, renames: &HashMap<String, String>) {
    for arg in &mut meta.args {
        rename_expr(arg, renames);
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
}

fn rename_struct_body_items(items: &mut [StructBodyItem], renames: &HashMap<String, String>) {
    for item in items {
        match item {
            StructBodyItem::Field(field) => {
                rename_type_expr(&mut field.ty, renames);
                rename_spliced_name(&mut field.name, renames);
            }

            StructBodyItem::For { source, body, .. } => {
                rename_expr(source, renames);
                rename_struct_body_items(body, renames);
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

fn rename_spliced_name(parts: &mut [NamePart], renames: &HashMap<String, String>) {
    for part in parts {
        if let NamePart::Splice(expr) = part {
            rename_expr(expr, renames);
        }
    }
}

fn rename_facet(facet: &mut Facet, renames: &HashMap<String, String>) {
    match &mut facet.payload {
        FacetPayload::Bare => {}

        FacetPayload::Expr(expr) => rename_expr(expr, renames),

        FacetPayload::Block(statements) => {
            for statement in statements {
                rename_statement(statement, renames);
            }
        }

        FacetPayload::Type(ty) => rename_type_expr(ty, renames),
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

        Expr::Integer { .. } | Expr::String { .. } | Expr::Here { .. } => {}

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

        Expr::Splice { inner, .. } => rename_expr(inner, renames),

        Expr::Construct { callee, generic_args, fields, .. } => {
            rename_expr(callee, renames);

            for arg in generic_args {
                match arg {
                    TypeArgument::Type(ty) => rename_type_expr(ty, renames),
                    TypeArgument::Const(expr) => rename_expr(expr, renames),
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
    }
}

fn rename_construct_items(items: &mut [ConstructItem], renames: &HashMap<String, String>) {
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

fn module_base_dir(module: &ModulePath, importer: &Path) -> Result<PathBuf, LoadError> {
    if module.relative_level == 0 {
        std::env::current_dir().map_err(|error| LoadError::Io {
            path: importer.to_path_buf(),
            message: error.to_string(),
        })
    } else {
        let mut dir = importer
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        for _ in 1..module.relative_level {
            dir = dir.parent().map(Path::to_path_buf).unwrap_or(dir);
        }

        Ok(dir)
    }
}

fn resolve_module_path(module: &ModulePath, importer: &Path) -> Result<PathBuf, LoadError> {
    let base = module_base_dir(module, importer)?;

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
    let base = module_base_dir(module, importer)?;

    let mut candidate = base;

    for segment in &module.segments {
        candidate.push(segment);
    }

    candidate.push(name);
    candidate.set_extension("basm");

    canonicalize(&candidate).map_err(|_| LoadError::ModuleNotFound {
        importer: importer.to_path_buf(),
        module: format!("{}.{name}", module_display(module)),
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

        let symbols = crate::resolver::collect_symbols(&program)
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
            let id = symbols.lookup(name).expect("symbol should exist");
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
            |statement| matches!(statement, Statement::Struct(decl) if decl.name == "TheStruct")
        ));

        assert!(program.statements.iter().any(
            |statement| matches!(statement, Statement::Const(decl) if literal_name(&decl.name).as_deref() == Some("the_const"))
        ));

        fs::remove_dir_all(&dir).ok();
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

        let symbols = crate::resolver::collect_symbols(&program)
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
            "pub macro mov(dst: int, value: int) | syntax \"mov $dst$, $value$\" {\n}\n",
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
                Statement::Invocation(invocation) if invocation.name == "mov" => Some(invocation),
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
            "macro mov(dst: int, value: int) | syntax \"mov $dst$: $value$\" {\n}\n\nmov r1: 7\n",
        )
        .unwrap();

        load_program(&dir.join("after.basm"))
            .expect("a custom-syntax call site after its own declaration should load");

        fs::write(
            dir.join("before.basm"),
            "mov r1: 7\n\nmacro mov(dst: int, value: int) | syntax \"mov $dst$: $value$\" {\n}\n",
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
