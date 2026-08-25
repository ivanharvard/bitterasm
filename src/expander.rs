//! Purely syntactic macro expansion — substitutes a macro's declared
//! parameters with a call site's argument *expressions*, not evaluated
//! `Value`s, throughout its body, then (optionally, recursively) does the
//! same for any invocation the substituted body itself contains. Nothing
//! here evaluates `@emit`/`@return` or resolves a single type; that's the
//! resolver's job, and it's why this module doesn't depend on it. This is
//! `bitterasm expand`'s engine — think `cargo expand`, not `cargo build`.
//!
//! A nested declaration (`struct`, `type`, another `macro`) inside a body
//! being substituted gets its own generic params/params removed from the
//! substitution map before its own body/fields/target are walked — the
//! same shadowing a real interpreter would apply, so a nested macro
//! reusing an outer macro's parameter name for one of its own isn't
//! silently substituted into.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::ast::{
    CallArgument, ConstDeclaration, ConstructItem, Expr, Facet, FacetPayload, Invocation,
    MacroDeclaration, MacroParameter, MetaStatement, NamePart, Program, Statement,
    StructDeclaration, TypeAliasDeclaration,
};
use crate::printer;
use crate::token::Span;
use crate::types::{GenericParameter, StructBodyItem, StructField, TypeArgument, TypeExpr};

/// Macro declarations available to expand an invocation against, by name —
/// built once from a fully import-resolved [`Program`] (see
/// [`crate::loader::load_program`]) so a call to an imported macro
/// resolves the same way it would for real compilation. Only ever looks at
/// *top-level* macro declarations: a macro nested inside another macro's
/// body is a declaration to be spliced in, not something invocations
/// elsewhere can already be calling.
pub struct MacroTable<'a>(HashMap<&'a str, &'a MacroDeclaration>);

impl<'a> MacroTable<'a> {
    pub fn from_program(program: &'a Program) -> Self {
        let mut table = HashMap::new();

        for statement in &program.statements {
            if let Statement::Macro(decl) = statement {
                table.insert(decl.name.as_str(), decl);
            }
        }

        Self(table)
    }
}

/// Expands every top-level invocation in `program` whose span overlaps
/// `range` (every one, if `range` is `None`), splicing each one's
/// expansion into `source` at its own original byte span — `program` must
/// have been parsed directly from `source` (not a loader-flattened,
/// multi-file program) for those spans to mean anything against it.
/// Everything outside an expanded invocation's span, including statements
/// never touched at all, is copied through byte-for-byte.
pub fn expand_source(
    source: &str,
    program: &Program,
    table: &MacroTable,
    depth: usize,
    range: Option<Range<usize>>,
) -> String {
    let mut edits: Vec<(Span, String)> = Vec::new();

    for statement in &program.statements {
        let Statement::Invocation(invocation) = statement else {
            continue;
        };

        let in_range = match &range {
            Some(range) => invocation.span.start < range.end && invocation.span.end > range.start,
            None => true,
        };

        if !in_range {
            continue;
        }

        let expanded = expand_invocation(table, invocation, depth);
        edits.push((trim_trailing_newline(source, invocation.span), printer::print_statements(&expanded, 0)));
    }

    // Descending by start so an earlier edit's length change never shifts
    // the byte offset an edit later in this loop (earlier in the file)
    // still needs to land at.
    edits.sort_by(|a, b| b.0.start.cmp(&a.0.start));

    let mut result = source.to_string();

    for (span, text) in edits {
        result.replace_range(span.start..span.end, &text);
    }

    result
}

// `Invocation.span` runs through its trailing newline (`statement_end`
// consumes it) — replacing the full span would swallow the line break that
// separates it from whatever comes next, gluing the two together. Trimming
// it back by one byte leaves that newline as part of the untouched source
// around the edit, same as it always was.
fn trim_trailing_newline(source: &str, span: Span) -> Span {
    if source.as_bytes().get(span.end.wrapping_sub(1)) == Some(&b'\n') {
        Span::new(span.start, span.end - 1)
    } else {
        span
    }
}

/// Expands one invocation against `table`, `depth` rounds deep. `depth ==
/// 0` (and an invocation `table` has no macro for at all — a real target
/// instruction, or one genuinely defined elsewhere) leaves it as a bare,
/// unexpanded `Invocation` statement.
pub fn expand_invocation(table: &MacroTable, invocation: &Invocation, depth: usize) -> Vec<Statement> {
    if depth == 0 {
        return vec![Statement::Invocation(invocation.clone())];
    }

    let Some(decl) = table.0.get(invocation.name.as_str()) else {
        return vec![Statement::Invocation(invocation.clone())];
    };

    let substitutions: HashMap<String, Expr> = decl
        .params
        .iter()
        .map(|param| param.name.clone())
        .zip(invocation.operands.iter().cloned())
        .collect();

    let substituted = substitute_statements(&decl.body, &substitutions);

    if depth == 1 {
        return substituted;
    }

    substituted
        .into_iter()
        .flat_map(|statement| match statement {
            Statement::Invocation(nested) => expand_invocation(table, &nested, depth - 1),
            other => vec![other],
        })
        .collect()
}

fn without_shadowed<'a>(
    substitutions: &HashMap<String, Expr>,
    shadowed: impl Iterator<Item = &'a str>,
) -> HashMap<String, Expr> {
    let shadowed: HashSet<&str> = shadowed.collect();

    substitutions
        .iter()
        .filter(|(name, _)| !shadowed.contains(name.as_str()))
        .map(|(name, expr)| (name.clone(), expr.clone()))
        .collect()
}

fn generic_param_name(param: &GenericParameter) -> &str {
    match param {
        GenericParameter::Const { name, .. } | GenericParameter::Type { name, .. } => name,
    }
}

pub(crate) fn substitute_statements(
    statements: &[Statement],
    substitutions: &HashMap<String, Expr>,
) -> Vec<Statement> {
    statements
        .iter()
        .map(|statement| substitute_statement(statement, substitutions))
        .collect()
}

fn substitute_statement(statement: &Statement, substitutions: &HashMap<String, Expr>) -> Statement {
    match statement {
        Statement::Import(_) | Statement::Label(_) | Statement::Enum(_) => statement.clone(),

        Statement::Invocation(invocation) => Statement::Invocation(Invocation {
            name: invocation.name.clone(),
            operands: invocation
                .operands
                .iter()
                .map(|expr| substitute_expr(expr, substitutions))
                .collect(),
            span: invocation.span,
        }),

        Statement::Const(decl) => Statement::Const(ConstDeclaration {
            name: substitute_spliced_name(&decl.name, substitutions),
            is_pub: decl.is_pub,
            ty: decl.ty.as_ref().map(|ty| substitute_type_expr(ty, substitutions)),
            value: substitute_expr(&decl.value, substitutions),
            span: decl.span,
        }),

        Statement::Struct(decl) => Statement::Struct(substitute_struct(decl, substitutions)),
        Statement::TypeAlias(decl) => Statement::TypeAlias(substitute_type_alias(decl, substitutions)),
        Statement::Macro(decl) => Statement::Macro(substitute_macro(decl, substitutions)),

        // `@for`'s own loop variable (`args[0]`) is a binding introduced
        // by this statement itself, the same way a macro's declared
        // parameters are — so it's shadowed out of `substitutions` before
        // walking `body`, the same way `substitute_macro`/`substitute_struct`
        // shadow their own generic params. `else_body` (only ever present
        // on `@if`) introduces no binding of its own and always uses the
        // outer `substitutions` unchanged.
        Statement::Meta(meta) => {
            let body = match (&meta.body, meta.name.as_str(), meta.args.first()) {
                (Some(body), "for", Some(Expr::Identifier { name, .. })) => {
                    let inner = without_shadowed(substitutions, std::iter::once(name.as_str()));
                    Some(substitute_statements(body, &inner))
                }

                (Some(body), _, _) => Some(substitute_statements(body, substitutions)),

                (None, _, _) => None,
            };

            Statement::Meta(MetaStatement {
                name: meta.name.clone(),
                args: meta.args.iter().map(|expr| substitute_expr(expr, substitutions)).collect(),
                body,
                else_body: meta
                    .else_body
                    .as_ref()
                    .map(|body| substitute_statements(body, substitutions)),
                span: meta.span,
            })
        }
    }
}

fn substitute_struct(decl: &StructDeclaration, substitutions: &HashMap<String, Expr>) -> StructDeclaration {
    let inner = without_shadowed(substitutions, decl.generic_params.iter().map(generic_param_name));

    StructDeclaration {
        name: decl.name.clone(),
        is_pub: decl.is_pub,
        generic_params: decl.generic_params.clone(),
        facets: substitute_facets(&decl.facets, &inner),
        fields: substitute_struct_body_items(&decl.fields, &inner),
        span: decl.span,
    }
}

fn substitute_struct_body_items(
    items: &[StructBodyItem],
    substitutions: &HashMap<String, Expr>,
) -> Vec<StructBodyItem> {
    items
        .iter()
        .map(|item| match item {
            StructBodyItem::Field(field) => StructBodyItem::Field(StructField {
                name: substitute_spliced_name(&field.name, substitutions),
                ty: substitute_type_expr(&field.ty, substitutions),
                is_pub: field.is_pub,
                is_const: field.is_const,
                default: field.default.as_ref().map(|d| substitute_expr(d, substitutions)),
                span: field.span,
            }),

            StructBodyItem::For { var, source, body, span } => {
                let inner = without_shadowed(substitutions, std::iter::once(var.as_str()));

                StructBodyItem::For {
                    var: var.clone(),
                    source: substitute_expr(source, substitutions),
                    body: substitute_struct_body_items(body, &inner),
                    span: *span,
                }
            }

            StructBodyItem::If { condition, body, else_body, span } => StructBodyItem::If {
                condition: substitute_expr(condition, substitutions),
                body: substitute_struct_body_items(body, substitutions),
                else_body: else_body
                    .as_ref()
                    .map(|else_body| substitute_struct_body_items(else_body, substitutions)),
                span: *span,
            },
        })
        .collect()
}

fn substitute_spliced_name(
    parts: &[NamePart],
    substitutions: &HashMap<String, Expr>,
) -> Vec<NamePart> {
    parts
        .iter()
        .map(|part| match part {
            NamePart::Literal(text) => NamePart::Literal(text.clone()),
            NamePart::Splice(expr) => NamePart::Splice(substitute_expr(expr, substitutions)),
        })
        .collect()
}

fn substitute_type_alias(
    decl: &TypeAliasDeclaration,
    substitutions: &HashMap<String, Expr>,
) -> TypeAliasDeclaration {
    let inner = without_shadowed(substitutions, decl.generic_params.iter().map(generic_param_name));

    TypeAliasDeclaration {
        name: decl.name.clone(),
        is_pub: decl.is_pub,
        generic_params: decl.generic_params.clone(),
        facets: substitute_facets(&decl.facets, &inner),
        ty: substitute_type_expr(&decl.ty, &inner),
        span: decl.span,
    }
}

fn substitute_macro(decl: &MacroDeclaration, substitutions: &HashMap<String, Expr>) -> MacroDeclaration {
    // Param *types* and facets/return-type are evaluated in the outer
    // scope this macro is declared in, same as a real declaration — only
    // its own body is where the shadowing kicks in.
    let inner = without_shadowed(substitutions, decl.params.iter().map(|param| param.name.as_str()));

    MacroDeclaration {
        name: decl.name.clone(),
        is_pub: decl.is_pub,
        params: decl
            .params
            .iter()
            .map(|param| MacroParameter {
                name: param.name.clone(),
                ty: substitute_type_expr(&param.ty, substitutions),
                span: param.span,
            })
            .collect(),
        return_ty: decl.return_ty.as_ref().map(|ty| substitute_type_expr(ty, substitutions)),
        facets: substitute_facets(&decl.facets, substitutions),
        body: substitute_statements(&decl.body, &inner),
        span: decl.span,
    }
}

fn substitute_facets(facets: &[Facet], substitutions: &HashMap<String, Expr>) -> Vec<Facet> {
    facets
        .iter()
        .map(|facet| Facet {
            name: facet.name.clone(),
            payload: match &facet.payload {
                FacetPayload::Bare => FacetPayload::Bare,
                FacetPayload::Expr(expr) => FacetPayload::Expr(substitute_expr(expr, substitutions)),
                FacetPayload::Type(ty) => FacetPayload::Type(substitute_type_expr(ty, substitutions)),
                FacetPayload::Block(statements) => {
                    FacetPayload::Block(substitute_statements(statements, substitutions))
                }
            },
            span: facet.span,
        })
        .collect()
}

fn substitute_type_expr(ty: &TypeExpr, substitutions: &HashMap<String, Expr>) -> TypeExpr {
    match ty {
        TypeExpr::Named { .. } => ty.clone(),

        TypeExpr::Apply { base, args, span } => TypeExpr::Apply {
            base: Box::new(substitute_type_expr(base, substitutions)),
            args: args
                .iter()
                .map(|arg| match arg {
                    TypeArgument::Type(ty) => TypeArgument::Type(substitute_type_expr(ty, substitutions)),
                    TypeArgument::Const(expr) => TypeArgument::Const(substitute_expr(expr, substitutions)),
                })
                .collect(),
            span: *span,
        },
    }
}

pub fn substitute_expr(expr: &Expr, substitutions: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Identifier { name, .. } => substitutions.get(name).cloned().unwrap_or_else(|| expr.clone()),

        Expr::Integer { .. } | Expr::String { .. } | Expr::Here { .. } => expr.clone(),

        Expr::Member { object, member, span } => Expr::Member {
            object: Box::new(substitute_expr(object, substitutions)),
            member: member.clone(),
            span: *span,
        },

        Expr::Call { callee, arguments, span } => Expr::Call {
            callee: Box::new(substitute_expr(callee, substitutions)),
            arguments: arguments
                .iter()
                .map(|argument| CallArgument {
                    name: argument.name.clone(),
                    value: substitute_expr(&argument.value, substitutions),
                    span: argument.span,
                })
                .collect(),
            span: *span,
        },

        Expr::Unary { op, operand, span } => Expr::Unary {
            op: *op,
            operand: Box::new(substitute_expr(operand, substitutions)),
            span: *span,
        },

        Expr::Binary { left, op, right, span } => Expr::Binary {
            left: Box::new(substitute_expr(left, substitutions)),
            op: *op,
            right: Box::new(substitute_expr(right, substitutions)),
            span: *span,
        },

        Expr::Splice { inner, span } => Expr::Splice {
            inner: Box::new(substitute_expr(inner, substitutions)),
            span: *span,
        },

        Expr::Construct { callee, generic_args, fields, span } => Expr::Construct {
            callee: Box::new(substitute_expr(callee, substitutions)),
            generic_args: generic_args
                .iter()
                .map(|arg| match arg {
                    TypeArgument::Type(ty) => TypeArgument::Type(substitute_type_expr(ty, substitutions)),
                    TypeArgument::Const(expr) => TypeArgument::Const(substitute_expr(expr, substitutions)),
                })
                .collect(),
            fields: substitute_construct_items(fields, substitutions),
            span: *span,
        },

        Expr::As { value, ty, span } => Expr::As {
            value: Box::new(substitute_expr(value, substitutions)),
            ty: substitute_type_expr(ty, substitutions),
            span: *span,
        },

        Expr::Range { start, end, span } => Expr::Range {
            start: Box::new(substitute_expr(start, substitutions)),
            end: Box::new(substitute_expr(end, substitutions)),
            span: *span,
        },
    }
}

fn substitute_construct_items(
    items: &[ConstructItem],
    substitutions: &HashMap<String, Expr>,
) -> Vec<ConstructItem> {
    items
        .iter()
        .map(|item| match item {
            ConstructItem::Field { name, value, span } => ConstructItem::Field {
                name: substitute_spliced_name(name, substitutions),
                value: substitute_expr(value, substitutions),
                span: *span,
            },

            ConstructItem::For { var, source, body, span } => {
                let inner = without_shadowed(substitutions, std::iter::once(var.as_str()));

                ConstructItem::For {
                    var: var.clone(),
                    source: substitute_expr(source, substitutions),
                    body: substitute_construct_items(body, &inner),
                    span: *span,
                }
            }

            ConstructItem::If { condition, body, else_body, span } => ConstructItem::If {
                condition: substitute_expr(condition, substitutions),
                body: substitute_construct_items(body, substitutions),
                else_body: else_body
                    .as_ref()
                    .map(|else_body| substitute_construct_items(else_body, substitutions)),
                span: *span,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;

    fn parse(source: &str) -> Program {
        let tokens = lexer::lex(source).expect("source should lex");
        parser::parse(tokens).expect("source should parse")
    }

    #[test]
    fn substitutes_params_without_evaluating() {
        let program = parse("macro double(x: int) {\n    @emit x * 2\n}\n\ndouble 5\n");
        let table = MacroTable::from_program(&program);

        let Statement::Invocation(invocation) = &program.statements[1] else {
            panic!("expected an invocation");
        };

        let expanded = expand_invocation(&table, invocation, usize::MAX);

        assert_eq!(printer::print_statements(&expanded, 0), "@emit (5 * 2)");
    }

    #[test]
    fn depth_one_substitutes_but_does_not_expand_nested_invocations() {
        let program = parse(
            "macro helper(v: int) {\n    @emit v + 1\n}\n\n\
             macro outer(x: int) {\n    helper x\n}\n\nouter 5\n",
        );
        let table = MacroTable::from_program(&program);

        let Statement::Invocation(invocation) = &program.statements[2] else {
            panic!("expected an invocation");
        };

        let shallow = expand_invocation(&table, invocation, 1);
        assert_eq!(printer::print_statements(&shallow, 0), "helper 5");

        let deep = expand_invocation(&table, invocation, 2);
        assert_eq!(printer::print_statements(&deep, 0), "@emit (5 + 1)");
    }

    #[test]
    fn unknown_invocation_is_left_unexpanded() {
        let program = parse("mov r1, 7\n");
        let table = MacroTable::from_program(&program);

        let Statement::Invocation(invocation) = &program.statements[0] else {
            panic!("expected an invocation");
        };

        let expanded = expand_invocation(&table, invocation, usize::MAX);

        assert_eq!(printer::print_statements(&expanded, 0), "mov r1, 7");
    }

    #[test]
    fn nested_macro_param_shadows_outer_substitution() {
        let program = parse(
            "macro outer(x: int) {\n    macro inner(x: int) {\n        @emit x\n    }\n}\n",
        );
        let table = MacroTable::from_program(&program);

        let Statement::Macro(outer) = &program.statements[0] else {
            panic!("expected a macro declaration");
        };

        let mut substitutions = HashMap::new();
        substitutions.insert("x".to_string(), Expr::Integer { raw: "9".to_string(), span: Span::new(0, 0) });

        let substituted = substitute_statements(&outer.body, &substitutions);

        // `inner`'s own `x` param shadows the outer substitution, so its
        // body's `x` must stay a bare identifier, not become `9`.
        assert_eq!(
            printer::print_statements(&substituted, 0),
            "macro inner(x: int)\n{\n    @emit x\n}"
        );
        let _ = table;
    }

    #[test]
    fn expand_source_replaces_only_the_targeted_span_and_leaves_the_rest_untouched() {
        let source = "macro double(x: int) {\n    @emit x * 2\n}\n\nfoo\ndouble(5)\nbar\n";
        let program = parse(source);
        let table = MacroTable::from_program(&program);

        let result = expand_source(source, &program, &table, usize::MAX, None);

        assert!(result.contains("foo"));
        assert!(result.contains("bar"));
        assert!(result.contains("@emit (5 * 2)"));
        assert!(!result.contains("double(5)"));
    }

    #[test]
    fn expand_source_range_only_expands_the_targeted_invocation() {
        let source = "macro double(x: int) {\n    @emit x * 2\n}\n\ndouble(1)\ndouble(2)\n";
        let program = parse(source);
        let table = MacroTable::from_program(&program);

        // Byte offset of the second `double(2)` call only.
        let second_call_start = source.rfind("double(2)").unwrap();
        let range = second_call_start..source.len();

        let result = expand_source(source, &program, &table, usize::MAX, Some(range));

        assert!(result.contains("double(1)"));
        assert!(result.contains("@emit (2 * 2)"));
        assert!(!result.contains("double(2)"));
    }
}
