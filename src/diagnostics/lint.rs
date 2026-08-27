use super::{Diagnostic, Severity, SourceId};
use crate::ast::{Expr, Facet, FacetPayload, ImportItems, MacroDeclaration, NamePart, Program, Statement};
use crate::token::Span;
use crate::types::{StructBodyItem, TypeArgument, TypeExpr};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct LintName(&'static str);

impl LintName {
    pub const UNUSED_PARAMETER: Self = Self("unused_parameter");
    pub const UNUSED_IMPORT: Self = Self("unused_import");
    pub const UNREACHABLE_CODE: Self = Self("unreachable_code");
    pub const GENERATED_DECLARATIONS: Self = Self("generated_declarations");
    pub const UNFULFILLED_EXPECTATION: Self = Self("unfulfilled_lint_expectation");

    pub const fn as_str(self) -> &'static str { self.0 }

    pub fn named(name: &str) -> Option<Self> {
        match name {
            "unused_parameter" => Some(Self::UNUSED_PARAMETER),
            "unused_import" => Some(Self::UNUSED_IMPORT),
            "unreachable_code" => Some(Self::UNREACHABLE_CODE),
            "generated_declarations" => Some(Self::GENERATED_DECLARATIONS),
            "unfulfilled_lint_expectation" => Some(Self::UNFULFILLED_EXPECTATION),
            _ => None,
        }
    }

    pub fn group(name: &str) -> Option<&'static [Self]> {
        match name {
            "unused" => Some(&[Self::UNUSED_PARAMETER, Self::UNUSED_IMPORT]),
            "all" | "warnings" => Some(&[
                Self::UNUSED_PARAMETER,
                Self::UNUSED_IMPORT,
                Self::UNREACHABLE_CODE,
                Self::GENERATED_DECLARATIONS,
                Self::UNFULFILLED_EXPECTATION,
            ]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintLevel { Allow, Expect, Warn, Deny, Forbid }

impl FromStr for LintLevel {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "allow" => Ok(Self::Allow),
            "expect" => Ok(Self::Expect),
            "warn" => Ok(Self::Warn),
            "deny" => Ok(Self::Deny),
            "forbid" => Ok(Self::Forbid),
            _ => Err(format!("unknown lint level `{value}`")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LintConfig {
    levels: HashMap<LintName, LintLevel>,
}

impl Default for LintConfig {
    fn default() -> Self {
        let mut levels = HashMap::new();
        for lint in LintName::group("all").unwrap() {
            levels.insert(*lint, LintLevel::Warn);
        }
        Self { levels }
    }
}

impl LintConfig {
    pub fn level(&self, lint: LintName) -> LintLevel {
        self.levels.get(&lint).copied().unwrap_or(LintLevel::Warn)
    }

    pub fn set(&mut self, selector: &str, level: LintLevel) -> Result<(), String> {
        if let Some(lint) = LintName::named(selector) {
            self.levels.insert(lint, level);
            return Ok(());
        }
        if let Some(group) = LintName::group(selector) {
            for lint in group { self.levels.insert(*lint, level); }
            return Ok(());
        }
        Err(format!("unknown lint or lint group `{selector}`"))
    }
}

pub fn load_lint_config(path: &Path) -> Result<LintConfig, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    parse_lint_config(&source, &path.display().to_string())
}

fn parse_lint_config(source: &str, display_path: &str) -> Result<LintConfig, String> {
    let mut config = LintConfig::default();
    let mut section = String::new();
    for (index, raw_line) in source.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
            continue;
        }
        if section != "lints" { continue; }
        let (selector, value) = line.split_once('=').ok_or_else(|| {
            format!("{display_path}:{}: expected `lint = level`", index + 1)
        })?;
        let selector = selector.trim();
        let level = value.trim().trim_matches('"').parse::<LintLevel>()?;
        config.set(selector, level).map_err(|error| {
            format!("{display_path}:{}: {error}", index + 1)
        })?;
    }
    Ok(config)
}

pub fn lint_program(program: &Program, source: SourceId, config: &LintConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut references = HashSet::new();
    for statement in &program.statements {
        collect_program_statement_references(statement, &mut references);
    }
    for statement in &program.statements {
        let Statement::Import(import) = statement else { continue };
        let ImportItems::Names(names) = &import.items else { continue };
        for name in names {
            if !references.contains(name) {
                emit_lint(
                    Diagnostic::warning(
                        LintName::UNUSED_IMPORT,
                        format!("imported name `{name}` is never used"),
                    )
                    .primary(source, import.span, "unused import")
                    .help("remove the imported name"),
                    config.level(LintName::UNUSED_IMPORT),
                    &mut diagnostics,
                );
            }
        }
    }
    for statement in &program.statements {
        if let Statement::Macro(declaration) = statement {
            lint_macro(declaration, source, config, &mut diagnostics);
        }
    }
    diagnostics
}

fn collect_program_statement_references(statement: &Statement, names: &mut HashSet<String>) {
    match statement {
        Statement::Import(_) | Statement::Label(_) => {}
        Statement::Struct(declaration) => {
            for item in &declaration.fields { collect_struct_item_references(item, names); }
            for facet in &declaration.facets {
                if let FacetPayload::Expr(expr) = &facet.payload { collect_expr_identifiers(expr, names); }
            }
        }
        Statement::Enum(declaration) => {
            for variant in &declaration.variants {
                if let Some(ty) = &variant.payload { collect_type_references(ty, names); }
            }
        }
        Statement::TypeAlias(declaration) => collect_type_references(&declaration.ty, names),
        Statement::Const(declaration) => {
            if let Some(ty) = &declaration.ty { collect_type_references(ty, names); }
            collect_expr_identifiers(&declaration.value, names);
        }
        Statement::Invocation(invocation) => {
            names.insert(invocation.name.clone());
            invocation.operands.iter().for_each(|expr| collect_expr_identifiers(expr, names));
        }
        // References the macro it's overriding the syntax of — counts as
        // a use, the same as an ordinary invocation of it would, so a
        // macro only ever called through its overridden spelling isn't
        // flagged as unused.
        Statement::SyntaxOverride(override_statement) => {
            names.insert(override_statement.name.clone());
        }
        Statement::Macro(declaration) => {
            for parameter in &declaration.params {
                collect_type_references(&parameter.ty, names);
                if let Some(default) = &parameter.default { collect_expr_identifiers(default, names); }
            }
            if let Some(ty) = &declaration.return_ty { collect_type_references(ty, names); }
            declaration.body.iter().for_each(|item| collect_program_statement_references(item, names));
            for facet in &declaration.facets {
                if !is_lint_facet(&facet.name) {
                    if let FacetPayload::Expr(expr) = &facet.payload { collect_expr_identifiers(expr, names); }
                }
            }
        }
        Statement::Meta(meta) => {
            meta.args.iter().for_each(|expr| collect_expr_identifiers(expr, names));
            if let Some(body) = &meta.body { body.iter().for_each(|item| collect_program_statement_references(item, names)); }
            if let Some(body) = &meta.else_body { body.iter().for_each(|item| collect_program_statement_references(item, names)); }
            for arm in &meta.match_arms {
                if let Some(pattern) = &arm.pattern { collect_expr_identifiers(pattern, names); }
                arm.body.iter().for_each(|item| collect_program_statement_references(item, names));
            }
        }
    }
}

fn collect_struct_item_references(item: &StructBodyItem, names: &mut HashSet<String>) {
    match item {
        StructBodyItem::Field(field) => {
            collect_type_references(&field.ty, names);
            if let Some(default) = &field.default { collect_expr_identifiers(default, names); }
        }
        StructBodyItem::For { source, body, .. } => {
            collect_expr_identifiers(source, names);
            body.iter().for_each(|item| collect_struct_item_references(item, names));
        }
        StructBodyItem::If { condition, body, else_body, .. } => {
            collect_expr_identifiers(condition, names);
            body.iter().for_each(|item| collect_struct_item_references(item, names));
            if let Some(body) = else_body { body.iter().for_each(|item| collect_struct_item_references(item, names)); }
        }
    }
}

fn collect_type_references(ty: &TypeExpr, names: &mut HashSet<String>) {
    match ty {
        TypeExpr::Named { path, .. } => {
            if let Some(name) = path.last() { names.insert(name.clone()); }
        }
        TypeExpr::Apply { base, args, .. } => {
            collect_type_references(base, names);
            for arg in args {
                match arg {
                    TypeArgument::Type(ty) => collect_type_references(ty, names),
                    TypeArgument::Const(expr) => collect_expr_identifiers(expr, names),
                    TypeArgument::Wildcard(_) => {}
                }
            }
        }
    }
}

fn lint_macro(
    declaration: &MacroDeclaration,
    source: SourceId,
    global: &LintConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let directives = facet_directives(&declaration.facets);
    let mut referenced = HashSet::new();
    for parameter in &declaration.params {
        if let Some(default) = &parameter.default {
            collect_expr_identifiers(default, &mut referenced);
        }
    }
    for statement in &declaration.body { collect_statement_identifiers(statement, &mut referenced); }
    for facet in &declaration.facets {
        if !is_lint_facet(&facet.name) {
            if let FacetPayload::Expr(expr) = &facet.payload { collect_expr_identifiers(expr, &mut referenced); }
        }
    }

    let mut occurred = HashSet::new();
    for parameter in &declaration.params {
        if !referenced.contains(parameter.name.as_str()) && !parameter.name.starts_with('_') {
            occurred.insert(LintName::UNUSED_PARAMETER);
            emit_lint(
                Diagnostic::warning(
                    LintName::UNUSED_PARAMETER,
                    format!("parameter `{}` is never used", parameter.name),
                )
                .primary(source, parameter.span, "unused parameter")
                .help(format!("prefix it with an underscore: `_{}`", parameter.name)),
                effective_level(LintName::UNUSED_PARAMETER, global, &directives),
                diagnostics,
            );
        }
    }

    lint_unreachable_block(
        &declaration.body,
        source,
        effective_level(LintName::UNREACHABLE_CODE, global, &directives),
        &mut occurred,
        diagnostics,
    );

    for (lint, level, span) in directives {
        if level == LintLevel::Expect && !occurred.contains(&lint) {
            emit_lint(
                Diagnostic::warning(
                    LintName::UNFULFILLED_EXPECTATION,
                    format!("expected `{}` warning was not produced", lint.as_str()),
                )
                .primary(source, span, "unfulfilled lint expectation")
                .help("remove this stale `expect` facet"),
                global.level(LintName::UNFULFILLED_EXPECTATION),
                diagnostics,
            );
        }
    }
}

fn effective_level(
    lint: LintName,
    global: &LintConfig,
    directives: &[(LintName, LintLevel, Span)],
) -> LintLevel {
    let mut level = global.level(lint);
    for (selected, next, _) in directives {
        if *selected == lint && level != LintLevel::Forbid { level = *next; }
    }
    level
}

fn emit_lint(mut diagnostic: Diagnostic, level: LintLevel, output: &mut Vec<Diagnostic>) {
    match level {
        LintLevel::Allow | LintLevel::Expect => {}
        LintLevel::Warn => output.push(diagnostic),
        LintLevel::Deny | LintLevel::Forbid => {
            diagnostic.severity = Severity::Error;
            output.push(diagnostic);
        }
    }
}

fn facet_directives(facets: &[Facet]) -> Vec<(LintName, LintLevel, Span)> {
    let mut directives = Vec::new();
    for facet in facets {
        let level = match facet.name.as_str() {
            "allow" => LintLevel::Allow,
            "expect" => LintLevel::Expect,
            "warn" => LintLevel::Warn,
            "deny" => LintLevel::Deny,
            "forbid" => LintLevel::Forbid,
            _ => continue,
        };
        let FacetPayload::Expr(expr) = &facet.payload else { continue };
        let selectors: Vec<&str> = match expr {
            Expr::Identifier { name, .. } => vec![name],
            Expr::Call { callee, arguments, .. }
                if matches!(callee.as_ref(), Expr::Identifier { name, .. } if name == "lints") =>
            {
                arguments.iter().filter_map(|argument| match &argument.value {
                    Expr::Identifier { name, .. } => Some(name.as_str()),
                    _ => None,
                }).collect()
            }
            _ => Vec::new(),
        };
        for selector in selectors {
            if let Some(lint) = LintName::named(selector) {
                directives.push((lint, level, facet.span));
            } else if let Some(group) = LintName::group(selector) {
                directives.extend(group.iter().map(|lint| (*lint, level, facet.span)));
            }
        }
    }
    directives
}

fn is_lint_facet(name: &str) -> bool {
    matches!(name, "allow" | "expect" | "warn" | "deny" | "forbid")
}

fn lint_unreachable_block(
    statements: &[Statement],
    source: SourceId,
    level: LintLevel,
    occurred: &mut HashSet<LintName>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut returned = false;
    for statement in statements {
        if returned {
            occurred.insert(LintName::UNREACHABLE_CODE);
            emit_lint(
                Diagnostic::warning(LintName::UNREACHABLE_CODE, "unreachable statement")
                    .primary(source, statement_span(statement), "this statement cannot be reached"),
                level,
                diagnostics,
            );
            continue;
        }
        if let Statement::Meta(meta) = statement {
            if let Some(body) = &meta.body {
                lint_unreachable_block(body, source, level, occurred, diagnostics);
            }
            if let Some(body) = &meta.else_body {
                lint_unreachable_block(body, source, level, occurred, diagnostics);
            }
            for arm in &meta.match_arms {
                lint_unreachable_block(&arm.body, source, level, occurred, diagnostics);
            }
            returned = meta.name == "return";
        }
    }
}

fn statement_span(statement: &Statement) -> Span {
    match statement {
        Statement::Import(value) => value.span,
        Statement::Struct(value) => value.span,
        Statement::Enum(value) => value.span,
        Statement::TypeAlias(value) => value.span,
        Statement::Const(value) => value.span,
        Statement::Label(value) => value.span,
        Statement::Invocation(value) => value.span,
        Statement::Macro(value) => value.span,
        Statement::Meta(value) => value.span,
        Statement::SyntaxOverride(value) => value.span,
    }
}

fn collect_statement_identifiers(statement: &Statement, names: &mut HashSet<String>) {
    match statement {
        Statement::Const(value) => collect_expr_identifiers(&value.value, names),
        Statement::Invocation(value) => value.operands.iter().for_each(|expr| collect_expr_identifiers(expr, names)),
        Statement::Meta(value) => {
            value.args.iter().for_each(|expr| collect_expr_identifiers(expr, names));
            if let Some(body) = &value.body { body.iter().for_each(|item| collect_statement_identifiers(item, names)); }
            if let Some(body) = &value.else_body { body.iter().for_each(|item| collect_statement_identifiers(item, names)); }
            for arm in &value.match_arms {
                if let Some(pattern) = &arm.pattern { collect_expr_identifiers(pattern, names); }
                arm.body.iter().for_each(|item| collect_statement_identifiers(item, names));
            }
        }
        Statement::Struct(_) | Statement::Enum(_) | Statement::TypeAlias(_) |
        Statement::Import(_) | Statement::Label(_) | Statement::Macro(_) |
        Statement::SyntaxOverride(_) => {}
    }
}

fn collect_expr_identifiers(expr: &Expr, names: &mut HashSet<String>) {
    match expr {
        Expr::Identifier { name, .. } => { names.insert(name.clone()); }
        Expr::Member { object, .. } => collect_expr_identifiers(object, names),
        Expr::Call { callee, arguments, .. } => {
            collect_expr_identifiers(callee, names);
            arguments.iter().for_each(|arg| collect_expr_identifiers(&arg.value, names));
        }
        Expr::EnumVariant { enum_name, generic_args, payload, .. } => {
            names.insert(enum_name.clone());
            for argument in generic_args {
                match argument {
                    TypeArgument::Type(ty) => collect_type_references(ty, names),
                    TypeArgument::Const(expr) => collect_expr_identifiers(expr, names),
                    TypeArgument::Wildcard(_) => {}
                }
            }
            if let Some(payload) = payload { collect_expr_identifiers(payload, names); }
        }
        Expr::Construct { callee, generic_args, fields, .. } => {
            collect_expr_identifiers(callee, names);
            for argument in generic_args {
                match argument {
                    TypeArgument::Type(ty) => collect_type_references(ty, names),
                    TypeArgument::Const(expr) => collect_expr_identifiers(expr, names),
                    TypeArgument::Wildcard(_) => {}
                }
            }
            collect_construct_identifiers(fields, names);
        }
        Expr::As { value, ty, .. } => {
            collect_expr_identifiers(value, names);
            collect_type_references(ty, names);
        }
        Expr::Unary { operand: value, .. } |
        Expr::Splice { inner: value, .. } => collect_expr_identifiers(value, names),
        Expr::Binary { left, right, .. } | Expr::Range { start: left, end: right, .. } => {
            collect_expr_identifiers(left, names);
            collect_expr_identifiers(right, names);
        }
        Expr::Integer { .. } | Expr::String { .. } | Expr::Here { .. } => {}
    }
}

fn collect_construct_identifiers(
    items: &[crate::ast::ConstructItem],
    names: &mut HashSet<String>,
) {
    for item in items {
        match item {
            crate::ast::ConstructItem::Field { name, value, .. } => {
                for part in name {
                    if let NamePart::Splice(expr) = part { collect_expr_identifiers(expr, names); }
                }
                collect_expr_identifiers(value, names);
            }
            crate::ast::ConstructItem::For { source, body, .. } => {
                collect_expr_identifiers(source, names);
                collect_construct_identifiers(body, names);
            }
            crate::ast::ConstructItem::If { condition, body, else_body, .. } => {
                collect_expr_identifiers(condition, names);
                collect_construct_identifiers(body, names);
                if let Some(else_body) = else_body {
                    collect_construct_identifiers(else_body, names);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer, parser};

    fn lint(source: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        lint_program(&program, SourceId(0), config)
    }

    #[test]
    fn unused_parameter_warns_by_default() {
        let diagnostics = lint("macro f(value: int) {\n}\n", &LintConfig::default());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].lint, Some(LintName::UNUSED_PARAMETER));
        assert_eq!(diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn named_unused_import_is_reported_but_used_import_is_not() {
        let diagnostics = lint(
            "from std.binary import Endian, BITS_PER_BYTE\nconst width = BITS_PER_BYTE\n",
            &LintConfig::default(),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].lint, Some(LintName::UNUSED_IMPORT));
        assert!(diagnostics[0].message.contains("Endian"));
    }

    #[test]
    fn declaration_allow_suppresses_a_lint() {
        let diagnostics = lint(
            "macro f(value: int) | allow unused_parameter {\n}\n",
            &LintConfig::default(),
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn declaration_deny_promotes_a_lint_to_error() {
        let diagnostics = lint(
            "macro f(value: int) | deny unused_parameter {\n}\n",
            &LintConfig::default(),
        );
        assert_eq!(diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn stale_expectation_warns() {
        let diagnostics = lint(
            "macro f(value: int) | expect unused_parameter {\n@return value\n}\n",
            &LintConfig::default(),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].lint, Some(LintName::UNFULFILLED_EXPECTATION));
    }

    #[test]
    fn unreachable_statement_is_reported() {
        let diagnostics = lint(
            "macro f() {\n@return 1\n@return 2\n}\n",
            &LintConfig::default(),
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].lint, Some(LintName::UNREACHABLE_CODE));
    }

    #[test]
    fn project_lint_configuration_sets_groups_and_individual_lints() {
        let config = parse_lint_config(
            "indent_width = 2\n\n[lints]\nunused = \"allow\"\nunreachable_code = \"deny\"\n",
            "bitterasm.toml",
        )
        .unwrap();
        assert_eq!(config.level(LintName::UNUSED_PARAMETER), LintLevel::Allow);
        assert_eq!(config.level(LintName::UNUSED_IMPORT), LintLevel::Allow);
        assert_eq!(config.level(LintName::UNREACHABLE_CODE), LintLevel::Deny);
    }
}
