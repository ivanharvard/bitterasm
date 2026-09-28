//! What a module offers the file that imports it, worked out the way the
//! loader and parser work it out, so the docs show what really works.
//!
//! A module exports its own `pub` declarations plus everything its
//! `pub from` imports export. A macro's overloads join across modules (a
//! dialect adds `mov` overloads to the `mov` it builds on); anything else
//! declared `pub` here shadows an imported declaration of the same name.
//!
//! How a macro is *written* is separate from which overloads exist. Each
//! name has a set of syntax patterns, inherited from every import (`pub` or
//! not) and changed in source order: a macro's `syntax` facet adds one, and
//! a standalone `syntax name(...) = { ... }` line replaces the whole set.
//! Once a name has any pattern, its default `name a, b` spelling is gone.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ast::{
    literal_name, ConstDeclaration, Doc, EnumDeclaration, FacetPayload, ImportItems, Label,
    MacroDeclaration, Statement, StructDeclaration, TypeAliasDeclaration,
};
use crate::loader::{self, LoadError};
use crate::token::Span;
use crate::types::StructBodyItem;

pub(super) struct Module {
    pub name: String,
    pub doc: Option<Doc>,
    /// The file's text, for locating doc comments' examples.
    pub source: String,
    /// Every doc in the file, `pub` or not, in source order: what
    /// `bitterasm doc --test` runs the examples of.
    pub docs: Vec<Doc>,
    /// `pub from` imports, by the module they name.
    pub reexports: Vec<String>,
    /// Everything exported, in the order it's declared or imported.
    pub items: Vec<Item>,
    /// The syntax patterns in effect for each exported macro name.
    pub patterns: HashMap<String, Vec<Pattern>>,
}

#[derive(Clone)]
pub(super) struct Item {
    pub name: String,
    /// The module that declares it.
    pub origin: String,
    pub kind: ItemKind,
}

#[derive(Clone)]
pub(super) enum ItemKind {
    Macro { declaration: MacroDeclaration, pattern: Option<Pattern> },
    Struct(StructDeclaration),
    Enum(EnumDeclaration),
    Alias(TypeAliasDeclaration),
    Const(ConstDeclaration),
    Label(Label),
}

impl ItemKind {
    pub fn doc(&self) -> Option<&Doc> {
        match self {
            ItemKind::Macro { declaration, .. } => declaration.doc.as_ref(),
            ItemKind::Struct(declaration) => declaration.doc.as_ref(),
            ItemKind::Enum(declaration) => declaration.doc.as_ref(),
            ItemKind::Alias(declaration) => declaration.doc.as_ref(),
            ItemKind::Const(declaration) => declaration.doc.as_ref(),
            ItemKind::Label(label) => label.doc.as_ref(),
        }
    }
}

/// A call-site shape, as its author wrote it (the text between the braces,
/// whitespace collapsed), with the parameter names its captures bind.
#[derive(Clone, Debug)]
pub(super) struct Pattern {
    pub text: String,
    pub params: Vec<String>,
    /// A `syntax` line's own `##`.
    pub doc: Option<Doc>,
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text && self.params == other.params
    }
}

#[derive(Default)]
pub(super) struct Modules {
    cache: HashMap<PathBuf, Rc<Module>>,
}

impl Modules {
    pub fn load(&mut self, path: &Path) -> Result<Rc<Module>, LoadError> {
        let path = std::fs::canonicalize(path)
            .map_err(|error| LoadError::Io { path: path.to_path_buf(), message: error.to_string() })?;
        if let Some(module) = self.cache.get(&path) {
            return Ok(module.clone());
        }
        let module = Rc::new(self.build(&path)?);
        self.cache.insert(path, module.clone());
        Ok(module)
    }

    fn build(&mut self, path: &Path) -> Result<Module, LoadError> {
        let source = std::fs::read_to_string(path)
            .map_err(|error| LoadError::Io { path: path.to_path_buf(), message: error.to_string() })?;
        let program = loader::load_unrolled_entry_program(path)?;
        let name = loader::module_path_of(path);

        let mut items: Vec<Item> = Vec::new();
        let mut patterns: HashMap<String, Vec<Pattern>> = HashMap::new();
        let mut reexports = Vec::new();

        for statement in &program.statements {
            match statement {
                Statement::Import(import) => {
                    // `from std.riscv import native` names a module, not
                    // declarations; there's nothing to merge.
                    if loader::imports_modules(import, path) {
                        continue;
                    }
                    let target = loader::resolve_module_path(&import.module, path)?;
                    let child = self.load(&target)?;
                    let wanted = |name: &str| match &import.items {
                        ImportItems::All => true,
                        ImportItems::Names(names) => names.iter().any(|wanted| wanted == name),
                    };
                    for (name, child_patterns) in &child.patterns {
                        if wanted(name) {
                            let entry = patterns.entry(name.clone()).or_default();
                            for pattern in child_patterns {
                                if !entry.contains(pattern) {
                                    entry.push(pattern.clone());
                                }
                            }
                        }
                    }
                    if import.is_pub {
                        reexports.push(child.name.clone());
                        items.extend(child.items.iter().filter(|item| wanted(&item.name)).cloned());
                    }
                }

                Statement::SyntaxOverride(override_statement) => {
                    let pattern = Pattern {
                        text: pattern_text(&source, override_statement.span),
                        params: override_statement.pattern.param_order.clone(),
                        doc: override_statement.doc.clone(),
                    };
                    patterns.insert(override_statement.name.clone(), vec![pattern]);
                }

                Statement::Macro(declaration) => {
                    let Some(macro_name) = literal_name(&declaration.name) else { continue };
                    let pattern = declaration
                        .facets
                        .iter()
                        .find(|facet| facet.name == "syntax" && matches!(facet.payload, FacetPayload::Pattern(_)))
                        .map(|facet| Pattern {
                            text: pattern_text(&source, facet.span),
                            params: declaration.params.iter().map(|param| param.name.clone()).collect(),
                            doc: None,
                        });
                    if let Some(pattern) = &pattern {
                        let entry = patterns.entry(macro_name.clone()).or_default();
                        if !entry.contains(pattern) {
                            entry.push(pattern.clone());
                        }
                    }
                    if declaration.is_pub {
                        items.push(Item {
                            name: macro_name,
                            origin: name.clone(),
                            kind: ItemKind::Macro { declaration: declaration.clone(), pattern },
                        });
                    }
                }

                other => {
                    let Some((item_name, kind)) = declared_item(other) else { continue };
                    // A `pub` declaration here shadows an imported one.
                    items.retain(|item| item.name != item_name || matches!(item.kind, ItemKind::Macro { .. }));
                    items.push(Item { name: item_name, origin: name.clone(), kind });
                }
            }
        }

        let exported: HashSet<&str> = items.iter().map(|item| item.name.as_str()).collect();
        patterns.retain(|name, _| exported.contains(name.as_str()));

        let mut docs: Vec<Doc> = program.doc.iter().cloned().collect();
        collect_docs(&program.statements, &mut docs);
        // Unrolling a `@for` copies its body's docs once per iteration.
        docs.sort_by_key(|doc| doc.span.start);
        docs.dedup_by_key(|doc| doc.span);

        Ok(Module { name, doc: program.doc, source, docs, reexports, items, patterns })
    }
}

fn collect_docs(statements: &[Statement], out: &mut Vec<Doc>) {
    for statement in statements {
        let doc = match statement {
            Statement::Macro(declaration) => {
                collect_docs(&declaration.body, out);
                &declaration.doc
            }
            Statement::Struct(declaration) => {
                for field in &declaration.fields {
                    if let StructBodyItem::Field(field) = field {
                        out.extend(field.doc.iter().cloned());
                    }
                }
                &declaration.doc
            }
            Statement::Enum(declaration) => {
                out.extend(declaration.variants.iter().filter_map(|variant| variant.doc.clone()));
                &declaration.doc
            }
            Statement::TypeAlias(declaration) => &declaration.doc,
            Statement::Const(declaration) => &declaration.doc,
            Statement::Label(label) => &label.doc,
            Statement::SyntaxOverride(override_statement) => &override_statement.doc,
            Statement::Meta(meta) => {
                for body in [&meta.body, &meta.else_body].into_iter().flatten() {
                    collect_docs(body, out);
                }
                for arm in &meta.match_arms {
                    collect_docs(&arm.body, out);
                }
                continue;
            }
            Statement::Import(_) | Statement::ExternLabel(_) | Statement::Section(_) | Statement::Invocation(_) => continue,
        };
        out.extend(doc.iter().cloned());
    }
}

// A non-macro `pub` declaration, by name.
fn declared_item(statement: &Statement) -> Option<(String, ItemKind)> {
    let (name, is_pub, kind) = match statement {
        Statement::Struct(declaration) => {
            (literal_name(&declaration.name)?, declaration.is_pub, ItemKind::Struct(declaration.clone()))
        }
        Statement::Enum(declaration) => {
            (literal_name(&declaration.name)?, declaration.is_pub, ItemKind::Enum(declaration.clone()))
        }
        Statement::TypeAlias(declaration) => {
            (literal_name(&declaration.name)?, declaration.is_pub, ItemKind::Alias(declaration.clone()))
        }
        Statement::Const(declaration) => {
            (literal_name(&declaration.name)?, declaration.is_pub, ItemKind::Const(declaration.clone()))
        }
        Statement::Label(label) => (label.name.clone(), label.is_pub, ItemKind::Label(label.clone())),
        _ => return None,
    };
    is_pub.then_some((name, kind))
}

// The text between a pattern's braces, e.g. `lea $rd$, $src$` from
// `syntax { lea $rd$, $src$ }`, on one line.
fn pattern_text(source: &str, span: Span) -> String {
    let text = source.get(span.start..span.end).unwrap_or_default();
    let inner = match (text.find('{'), text.rfind('}')) {
        (Some(open), Some(close)) if open < close => &text[open + 1..close],
        _ => text,
    };
    inner.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_text_is_what_the_author_wrote() {
        let source = "pub macro rel(target: int) -> RipLabel\n    | syntax { [rel $target$] } {\n";
        let start = source.find("syntax").unwrap();
        let end = source.find("] }").unwrap() + 3;
        assert_eq!(pattern_text(source, Span::new(start, end)), "[rel $target$]");

        let source = "syntax add(rd, rs1, rs2) = {\n    $rd$ = $rs1$ + $rs2$\n}";
        assert_eq!(pattern_text(source, Span::new(0, source.len())), "$rd$ = $rs1$ + $rs2$");
    }
}
