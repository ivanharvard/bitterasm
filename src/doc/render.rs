//! A module's reference page, in Markdown. Macros come first, grouped by
//! name with one row per overload, each shown in the spelling that works
//! in *this* module (a dialect re-spells the instructions it re-exports).
//! Types, constants and labels follow; ones re-exported unchanged from
//! another module are listed by name with a link to their own page.
//! Folders get an index page listing what's in them; see [`super::layout`].

use std::collections::HashMap;

use super::doctest::doc_comment_info;
use super::layout::{relative, Layout};
use super::model::{Item, ItemKind, Module, Pattern};
use crate::ast::{literal_name, Doc, FacetPayload, MacroDeclaration};
use crate::printer::{print_expr, print_generic_params, print_type_expr};
use crate::types::StructBodyItem;

/// How one page links to the others.
pub(super) struct Links<'a> {
    pub layout: &'a Layout,
    /// This page's path.
    pub from: String,
    /// Each module's summary, for folder contents.
    pub summaries: &'a HashMap<String, String>,
}

impl Links<'_> {
    // A module's name, linked to its page when it has one.
    fn module(&self, module: &str) -> String {
        match self.layout.module_page(module) {
            Some(page) => format!("[{}]({})", code(module), relative(&self.from, &page)),
            None => code(module),
        }
    }

    // The table of what's in the folder at `path`.
    fn contents(&self, path: &[String]) -> String {
        let mut out = String::from("| Name | Summary |\n|---|---|\n");
        for child in self.layout.children(path) {
            let summary = match &child.module {
                Some(module) => self.summaries.get(module).cloned().unwrap_or_default(),
                None => {
                    let inner: Vec<String> = path.iter().chain(std::iter::once(&child.name)).cloned().collect();
                    let names: Vec<String> = self.layout.children(&inner).iter().map(|c| code(&c.name)).collect();
                    names.join(", ")
                }
            };
            let name = if child.is_folder { format!("{}/", child.name) } else { child.name.clone() };
            out.push_str(&format!("| [{}]({}) | {} |\n", code(&name), relative(&self.from, &child.page), summary));
        }
        out
    }
}

pub(super) fn page(module: &Module, links: &Links) -> String {
    let path = links.layout.path_of(&module.name).unwrap_or_default().to_vec();
    let title = path.last().cloned().unwrap_or_else(|| module.name.clone());
    let mut out = heading(&title, &links.layout.breadcrumb(&path, &links.from));
    if let Some(doc) = &module.doc {
        out.push_str(&markdown(&doc.text, 1));
        out.push_str("\n\n");
    }
    if !module.reexports.is_empty() {
        let names: Vec<String> = module.reexports.iter().map(|name| links.module(name)).collect();
        out.push_str(&format!("Re-exports {}.\n\n", join_words(&names)));
    }
    // A module that is also a folder lists the folder's contents too.
    if links.layout.is_folder(&path) {
        out.push_str(&format!("## Contents\n\n{}\n", links.contents(&path)));
    }

    let own = |item: &&Item| item.origin == module.name;
    let macros: Vec<&Item> = module.items.iter().filter(|item| matches!(item.kind, ItemKind::Macro { .. })).collect();
    let types: Vec<&Item> = module
        .items
        .iter()
        .filter(own)
        .filter(|item| matches!(item.kind, ItemKind::Struct(_) | ItemKind::Enum(_) | ItemKind::Alias(_)))
        .collect();
    let consts: Vec<&Item> = module.items.iter().filter(own).filter(|item| matches!(item.kind, ItemKind::Const(_))).collect();
    let labels: Vec<&Item> = module.items.iter().filter(own).filter(|item| matches!(item.kind, ItemKind::Label(_))).collect();

    if !macros.is_empty() {
        out.push_str("## Macros\n\n");
        let mut names: Vec<&str> = Vec::new();
        for item in &macros {
            if !names.contains(&item.name.as_str()) {
                names.push(&item.name);
            }
        }
        for name in names {
            let overloads: Vec<&Item> = macros.iter().copied().filter(|item| item.name == name).collect();
            out.push_str(&macro_section(module, name, &overloads, links));
        }
    }

    if !types.is_empty() {
        out.push_str("## Types\n\n");
        for item in types {
            out.push_str(&type_section(item));
        }
    }

    if !consts.is_empty() {
        out.push_str("## Constants\n\n| Constant | Type | Value | Description |\n|---|---|---|---|\n");
        for item in consts {
            let ItemKind::Const(declaration) = &item.kind else { continue };
            let ty = declaration.ty.as_ref().map(|ty| code(&print_type_expr(ty))).unwrap_or_default();
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                cell(&code(&item.name)),
                cell(&ty),
                cell(&code(&print_expr(&declaration.value))),
                summary(declaration.doc.as_ref()),
            ));
        }
        out.push('\n');
    }

    if !labels.is_empty() {
        out.push_str("## Labels\n\n| Label | Description |\n|---|---|\n");
        for item in labels {
            out.push_str(&format!("| {} | {} |\n", cell(&code(&item.name)), summary(item.kind.doc())));
        }
        out.push('\n');
    }

    // Types, constants and labels read the same whichever module they're
    // imported through, so a re-export just points at their own page.
    let mut origins: Vec<(&str, Vec<&str>)> = Vec::new();
    for item in &module.items {
        if item.origin == module.name || matches!(item.kind, ItemKind::Macro { .. }) {
            continue;
        }
        match origins.iter_mut().find(|(origin, _)| *origin == item.origin) {
            Some((_, names)) => names.push(&item.name),
            None => origins.push((&item.origin, vec![&item.name])),
        }
    }
    if !origins.is_empty() {
        out.push_str("## Re-exported\n\n");
        for (origin, names) in origins {
            let names: Vec<String> = names.iter().map(|name| code(name)).collect();
            out.push_str(&format!("From {}: {}.\n\n", links.module(origin), names.join(", ")));
        }
    }

    format!("{}\n", out.trim_end())
}

/// A folder's index page: what's in it, each with its summary.
pub(super) fn folder(path: &[String], links: &Links) -> String {
    heading(&links.layout.folder_title(path), &links.layout.breadcrumb(path, &links.from)) + &links.contents(path)
}

// A page's title, then its breadcrumb if it has one.
fn heading(title: &str, breadcrumb: &str) -> String {
    if breadcrumb.is_empty() {
        format!("# {}\n\n", code(title))
    } else {
        format!("# {}\n\n{breadcrumb}\n\n", code(title))
    }
}

// One overload: its doc, then its row. Several: a row each, described by
// its doc's first paragraph, with any doc longer than that shown in full
// below the table under the overload's syntax.
fn macro_section(module: &Module, name: &str, overloads: &[&Item], links: &Links) -> String {
    let mut out = format!("### {}\n\n", code(name));

    let patterns = module.patterns.get(name).map(Vec::as_slice).unwrap_or_default();
    // A `syntax` line's own doc describes how the name is written here.
    for doc in patterns.iter().filter_map(|pattern| pattern.doc.as_ref()) {
        out.push_str(&markdown(&doc.text, 3));
        out.push_str("\n\n");
    }
    let single = overloads.len() == 1;
    if single && let Some(doc) = overloads[0].kind.doc() {
        out.push_str(&markdown(&doc.text, 3));
        out.push_str("\n\n");
    }

    let foreign = overloads.iter().any(|item| item.origin != module.name);
    out.push_str("| Syntax | Parameters | Result | Description |");
    out.push_str(if foreign { " From |\n|---|---|---|---|---|\n" } else { "\n|---|---|---|---|\n" });
    let mut long_docs = Vec::new();
    for item in overloads {
        let ItemKind::Macro { declaration, pattern } = &item.kind else { continue };
        let form = form(name, declaration, pattern.as_ref(), patterns);
        let description = if single { String::new() } else { summary(declaration.doc.as_ref()) };
        if let Some(doc) = declaration.doc.as_ref().filter(|doc| !single && has_more_than_summary(doc)) {
            long_docs.push((form.clone(), doc));
        }
        out.push_str(&format!(
            "| {} | {} | {} | {} |",
            cell(&code(&form)),
            cell(&parameters(declaration)),
            cell(&result(declaration)),
            description,
        ));
        if foreign {
            let from = if item.origin == module.name { String::new() } else { links.module(&item.origin) };
            out.push_str(&format!(" {} |", cell(&from)));
        }
        out.push('\n');
    }
    out.push('\n');

    for (form, doc) in long_docs {
        out.push_str(&format!("#### {}\n\n{}\n\n", code(&form), markdown(&doc.text, 4)));
    }
    out
}

fn type_section(item: &Item) -> String {
    let mut out = format!("### {}\n\n", code(&item.name));
    let signature = match &item.kind {
        ItemKind::Struct(declaration) => {
            format!("struct {}{}", item.name, print_generic_params(&declaration.generic_params))
        }
        ItemKind::Enum(declaration) => {
            format!("enum {}{}", item.name, print_generic_params(&declaration.generic_params))
        }
        ItemKind::Alias(declaration) => format!(
            "type {}{} = {}",
            item.name,
            print_generic_params(&declaration.generic_params),
            print_type_expr(&declaration.ty),
        ),
        _ => return String::new(),
    };
    out.push_str(&format!("{}\n\n", code(&signature)));
    if let Some(doc) = item.kind.doc() {
        out.push_str(&markdown(&doc.text, 3));
        out.push_str("\n\n");
    }

    match &item.kind {
        ItemKind::Struct(declaration) => {
            let mut rows = String::new();
            let mut generated = false;
            for field in &declaration.fields {
                match field {
                    StructBodyItem::Field(field) if field.is_pub => {
                        let name = literal_name(&field.name).unwrap_or_default();
                        rows.push_str(&format!(
                            "| {} | {} | {} |\n",
                            cell(&code(&name)),
                            cell(&code(&print_type_expr(&field.ty))),
                            summary(field.doc.as_ref()),
                        ));
                    }
                    StructBodyItem::Field(_) => {}
                    _ => generated = true,
                }
            }
            if !rows.is_empty() {
                out.push_str("| Field | Type | Description |\n|---|---|---|\n");
                out.push_str(&rows);
                out.push('\n');
            }
            if generated {
                out.push_str("Some of its fields are generated by `@for` or `@if`.\n\n");
            }
        }
        ItemKind::Enum(declaration) => {
            out.push_str("| Variant | Payload | Description |\n|---|---|---|\n");
            for variant in &declaration.variants {
                let payload = variant.payload.as_ref().map(|ty| code(&print_type_expr(ty))).unwrap_or_default();
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    cell(&code(&variant.name)),
                    cell(&payload),
                    summary(variant.doc.as_ref()),
                ));
            }
            out.push('\n');
        }
        _ => {}
    }
    out
}

/// How `declaration` is written where `patterns` are in effect for `name`:
/// its own `syntax` facet if that's still in effect, else a pattern taking
/// as many arguments, else the default `name a, b` when `name` has no
/// patterns at all. With patterns but none that fits, it can only be
/// called, `name(a, b)`. A macro that returns a value and emits nothing is
/// used as a call too.
fn form(name: &str, declaration: &MacroDeclaration, own: Option<&Pattern>, patterns: &[Pattern]) -> String {
    let names: Vec<&str> = declaration.params.iter().map(|param| param.name.as_str()).collect();
    let call = format!("{name}({})", names.join(", "));

    if let Some(own) = own.filter(|own| patterns.contains(own)) {
        return fill(own, &names);
    }
    if !patterns.is_empty() {
        let required = declaration.params.iter().filter(|param| param.default.is_none()).count();
        return patterns
            .iter()
            .find(|pattern| (required..=names.len()).contains(&pattern.params.len()))
            .map_or(call, |pattern| fill(pattern, &names));
    }
    if declaration.return_ty.is_some() && emits(declaration).is_none() {
        return call;
    }
    if names.is_empty() { name.to_string() } else { format!("{name} {}", names.join(", ")) }
}

// A pattern's text with each `$capture$` replaced by the parameter it binds
// in this overload (captures bind positionally across overloads), and
// `\$`/`` \` `` escapes shown as the characters they stand for.
fn fill(pattern: &Pattern, names: &[&str]) -> String {
    let mut out = String::new();
    let mut chars = pattern.text.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => out.extend(chars.next()),
            '$' => {
                let capture: String = chars.by_ref().take_while(|&ch| ch != '$').collect();
                let bound = pattern
                    .params
                    .iter()
                    .position(|param| *param == capture)
                    .and_then(|index| names.get(index))
                    .copied()
                    .unwrap_or(&capture);
                out.push_str(bound);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn parameters(declaration: &MacroDeclaration) -> String {
    let mut parts = Vec::new();
    let generics = print_generic_params(&declaration.generic_params);
    if !generics.is_empty() {
        parts.push(code(&generics));
    }
    for param in &declaration.params {
        let default = param.default.as_ref().map(|value| format!(" = {}", print_expr(value))).unwrap_or_default();
        parts.push(code(&format!("{}: {}{default}", param.name, print_type_expr(&param.ty))));
    }
    parts.join(", ")
}

fn emits(declaration: &MacroDeclaration) -> Option<String> {
    declaration.facets.iter().find_map(|facet| match &facet.payload {
        FacetPayload::Type(ty) if facet.name == "emits" => Some(print_type_expr(ty)),
        _ => None,
    })
}

fn result(declaration: &MacroDeclaration) -> String {
    let mut parts = Vec::new();
    if let Some(ty) = &declaration.return_ty {
        parts.push(format!("returns {}", code(&print_type_expr(ty))));
    }
    if let Some(ty) = emits(declaration) {
        parts.push(format!("emits {}", code(&ty)));
    }
    parts.join(", ")
}

/// Doc text as page Markdown: headings moved down `shift` levels so they
/// nest under the item's own heading, and fences given their doc-comment
/// meaning (no language means BitterASM).
fn markdown(text: &str, shift: usize) -> String {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            if in_fence {
                out.push(line.to_string());
            } else {
                let indent = &line[..line.len() - trimmed.len()];
                out.push(format!("{indent}```{}", doc_comment_info(info)));
            }
            in_fence = !in_fence;
        } else if !in_fence && line.starts_with('#') {
            let level = line.chars().take_while(|&ch| ch == '#').count();
            let rest = &line[level..];
            if rest.is_empty() || rest.starts_with(' ') {
                out.push(format!("{}{rest}", "#".repeat((level + shift).min(6))));
            } else {
                out.push(line.to_string());
            }
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n").trim().to_string()
}

fn first_paragraph(doc: &Doc) -> Vec<&str> {
    doc.text
        .lines()
        .map(str::trim)
        .skip_while(|line| line.is_empty())
        .take_while(|line| !line.is_empty() && !line.starts_with("```"))
        .collect()
}

/// A doc's first paragraph on one line, for a table cell.
pub(super) fn summary(doc: Option<&Doc>) -> String {
    doc.map(|doc| cell(&first_paragraph(doc).join(" "))).unwrap_or_default()
}

fn has_more_than_summary(doc: &Doc) -> bool {
    let words = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    words(&doc.text) != words(&first_paragraph(doc).join(" "))
}


/// `text` as inline code, with enough backticks to hold any it contains.
fn code(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let longest = text.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    let ticks = "`".repeat(longest + 1);
    if longest > 0 { format!("{ticks} {text} {ticks}") } else { format!("{ticks}{text}{ticks}") }
}

// Table cells can't hold a raw `|` or a line break.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn join_words(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(text: &str, params: &[&str]) -> Pattern {
        Pattern { text: text.into(), params: params.iter().map(|p| p.to_string()).collect(), doc: None }
    }

    #[test]
    fn fills_captures_with_this_overloads_parameters() {
        let add = pattern("$rd$ = $rs1$ + $rs2$", &["rd", "rs1", "rs2"]);
        assert_eq!(fill(&add, &["dst", "a", "b"]), "dst = a + b");
        let escaped = pattern("cost \\$$n$", &["n"]);
        assert_eq!(fill(&escaped, &["amount"]), "cost $amount");
    }

    #[test]
    fn shifts_headings_but_not_code() {
        let text = "Intro.\n\n# Example\n\n```\n# a comment\nshow 1\n```";
        assert_eq!(
            markdown(text, 3),
            "Intro.\n\n#### Example\n\n```basm\n# a comment\nshow 1\n```"
        );
    }

    #[test]
    fn code_spans_hold_backticks() {
        assert_eq!(code("mov"), "`mov`");
        assert_eq!(code("r`i`"), "`` r`i` ``");
    }
}
