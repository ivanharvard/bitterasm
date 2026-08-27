//! Renders AST nodes back into BitterASM source text — the inverse of
//! `crate::parser`. Nothing before `expand` needed this: every earlier
//! stage only ever went source → AST → either more AST or a `Debug` dump.
//! This isn't a formatter for *existing* source (it doesn't preserve
//! comments or original spacing) — it's for printing freshly-built AST that
//! has no original source text of its own, e.g. a macro body after
//! parameter substitution.
//!
//! One real gap: [`crate::ast::Invocation`] only records a bound `name` and
//! `operands`, not which surface syntax (default `name arg, arg` or a
//! `syntax` facet's custom pattern) produced it — so every invocation
//! prints in the default form, even one that was originally written with a
//! custom pattern.

use crate::ast::{
    BinaryOp, CallArgument, ConstructItem, Expr, Facet, FacetPayload, ImportItems,
    MacroDeclaration, MacroParameter, NamePart, Statement, StructDeclaration, UnaryOp,
};
use crate::types::{GenericParameter, StructBodyItem, StructField, TypeArgument, TypeExpr};

const INDENT: &str = "    ";

pub fn print_statements(statements: &[Statement], indent: usize) -> String {
    statements
        .iter()
        .map(|statement| print_statement(statement, indent))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn print_statement(statement: &Statement, indent: usize) -> String {
    let pad = INDENT.repeat(indent);

    match statement {
        Statement::Import(import) => {
            let dots = ".".repeat(import.module.relative_level);
            let path = import.module.segments.join(".");

            let items = match &import.items {
                ImportItems::All => "*".to_string(),
                ImportItems::Names(names) => names.join(", "),
            };

            format!("{pad}from {dots}{path} import {items}")
        }

        Statement::Struct(decl) => print_struct(decl, indent),

        Statement::Enum(decl) => {
            let pub_kw = if decl.is_pub { "pub " } else { "" };
            let generics = print_generic_params(&decl.generic_params);
            let variants = decl.variants.iter().map(|variant| match &variant.payload {
                Some(ty) => format!("{}: {}", variant.name, print_type_expr(ty)),
                None => variant.name.clone(),
            }).collect::<Vec<_>>().join(", ");

            format!("{pad}{pub_kw}enum {name}{generics} {{ {variants} }}", name = decl.name)
        }

        Statement::TypeAlias(decl) => {
            let pub_kw = if decl.is_pub { "pub " } else { "" };
            let generics = print_generic_params(&decl.generic_params);
            let facets = print_facet_list(&decl.facets);

            format!(
                "{pad}{pub_kw}type {name}{generics} = {ty}{facets}",
                name = decl.name,
                ty = print_type_expr(&decl.ty),
            )
        }

        Statement::Const(decl) => {
            let pub_kw = if decl.is_pub { "pub " } else { "" };

            let ty = match &decl.ty {
                Some(ty) => format!(": {}", print_type_expr(ty)),
                None => String::new(),
            };

            format!(
                "{pad}{pub_kw}const {name}{ty} = {value}",
                name = print_spliced_name(&decl.name),
                value = print_expr(&decl.value),
            )
        }

        Statement::Label(label) => format!("{pad}{name}:", name = label.name),

        Statement::Invocation(invocation) => {
            if invocation.operands.is_empty() {
                format!("{pad}{name}", name = invocation.name)
            } else {
                let operands = invocation
                    .operands
                    .iter()
                    .map(print_expr)
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{pad}{name} {operands}", name = invocation.name)
            }
        }

        Statement::Macro(decl) => print_macro(decl, indent),

        Statement::Meta(meta) => {
            let args = meta.args.iter().map(print_expr).collect::<Vec<_>>().join(", ");

            if meta.name == "match" {
                let arms = meta.match_arms.iter().map(|arm| {
                    let pattern = arm.pattern.as_ref().map(print_expr).unwrap_or_else(|| "_".to_string());
                    format!("{}{} => {{\n{}\n{}}}", INDENT.repeat(indent + 1), pattern,
                        print_statements(&arm.body, indent + 2), INDENT.repeat(indent + 1))
                }).collect::<Vec<_>>().join(",\n");
                return format!("{pad}@match {args} {{\n{arms}\n{pad}}}");
            }

            match &meta.body {
                None if args.is_empty() => format!("{pad}@{}", meta.name),
                None => format!("{pad}@{} {args}", meta.name),

                Some(body) => {
                    let then_block = format!(
                        "{pad}@{name} {args} {{\n{body}\n{pad}}}",
                        name = meta.name,
                        body = print_statements(body, indent + 1),
                    );

                    match &meta.else_body {
                        Some(else_body) => format!(
                            "{then_block} @else {{\n{else_body}\n{pad}}}",
                            else_body = print_statements(else_body, indent + 1),
                        ),

                        None => then_block,
                    }
                }
            }
        }
    }
}

fn print_spliced_name(parts: &[NamePart]) -> String {
    parts
        .iter()
        .map(|part| match part {
            NamePart::Literal(text) => text.clone(),
            NamePart::Splice(expr) => format!("`{}`", print_expr(expr)),
        })
        .collect()
}

fn print_struct(decl: &StructDeclaration, indent: usize) -> String {
    let pad = INDENT.repeat(indent);
    let pub_kw = if decl.is_pub { "pub " } else { "" };
    let generics = print_generic_params(&decl.generic_params);
    let facets = print_facet_list(&decl.facets);

    let fields = print_struct_body_items(&decl.fields, indent + 1);

    format!(
        "{pad}{pub_kw}struct {name}{generics}{facets}\n{pad}{{\n{fields}\n{pad}}}",
        name = decl.name,
    )
}

fn print_struct_body_items(items: &[StructBodyItem], indent: usize) -> String {
    items
        .iter()
        .map(|item| print_struct_body_item(item, indent))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn print_struct_body_item(item: &StructBodyItem, indent: usize) -> String {
    let pad = INDENT.repeat(indent);

    match item {
        StructBodyItem::Field(field) => print_struct_field(field, indent),

        StructBodyItem::For { var, source, body, .. } => format!(
            "{pad}@for {var} in {source} {{\n{body}\n{pad}}}",
            source = print_expr(source),
            body = print_struct_body_items(body, indent + 1),
        ),

        StructBodyItem::If { condition, body, else_body, .. } => {
            let then_block = format!(
                "{pad}@if {condition} {{\n{body}\n{pad}}}",
                condition = print_expr(condition),
                body = print_struct_body_items(body, indent + 1),
            );

            match else_body {
                Some(else_body) => format!(
                    "{then_block} @else {{\n{else_body}\n{pad}}}",
                    else_body = print_struct_body_items(else_body, indent + 1),
                ),

                None => then_block,
            }
        }
    }
}

fn print_struct_field(field: &StructField, indent: usize) -> String {
    format!(
        "{pad}{name}: {ty}",
        pad = INDENT.repeat(indent),
        name = print_spliced_name(&field.name),
        ty = print_type_expr(&field.ty),
    )
}

fn print_macro(decl: &MacroDeclaration, indent: usize) -> String {
    let pad = INDENT.repeat(indent);
    let pub_kw = if decl.is_pub { "pub " } else { "" };

    let params = decl
        .params
        .iter()
        .map(print_macro_param)
        .collect::<Vec<_>>()
        .join(", ");

    let return_ty = match &decl.return_ty {
        Some(ty) => format!(" -> {}", print_type_expr(ty)),
        None => String::new(),
    };

    let facets = print_facet_list(&decl.facets);
    let body = print_statements(&decl.body, indent + 1);

    format!(
        "{pad}{pub_kw}macro {name}({params}){return_ty}{facets}\n{pad}{{\n{body}\n{pad}}}",
        name = decl.name,
    )
}

fn print_macro_param(param: &MacroParameter) -> String {
    format!("{name}: {ty}", name = param.name, ty = print_type_expr(&param.ty))
}

fn print_generic_params(params: &[GenericParameter]) -> String {
    if params.is_empty() {
        return String::new();
    }

    let rendered = params
        .iter()
        .map(|param| match param {
            GenericParameter::Const { name, ty, .. } => {
                format!("const {name}: {}", print_type_expr(ty))
            }

            GenericParameter::Type { name, .. } => name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("<{rendered}>")
}

fn print_facet_list(facets: &[Facet]) -> String {
    facets
        .iter()
        .map(|facet| format!(" | {}", print_facet(facet)))
        .collect()
}

fn print_facet(facet: &Facet) -> String {
    match &facet.payload {
        FacetPayload::Bare => facet.name.clone(),
        FacetPayload::Expr(expr) => format!("{} {}", facet.name, print_expr(expr)),
        FacetPayload::Type(ty) => format!("{} {}", facet.name, print_type_expr(ty)),

        FacetPayload::Block(statements) => format!(
            "{name} {{\n{body}\n}}",
            name = facet.name,
            body = print_statements(statements, 1),
        ),
    }
}

pub fn print_type_expr(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Named { path, .. } => path.join("."),

        TypeExpr::Apply { base, args, .. } => {
            let args = args
                .iter()
                .map(|arg| match arg {
                    TypeArgument::Type(ty) => print_type_expr(ty),
                    TypeArgument::Const(expr) => print_expr(expr),
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("{}<{args}>", print_type_expr(base))
        }
    }
}

pub fn print_expr(expr: &Expr) -> String {
    match expr {
        Expr::Identifier { name, .. } => name.clone(),
        Expr::Integer { raw, .. } => raw.clone(),
        Expr::String { value, .. } => format!("{value:?}"),

        Expr::Member { object, member, .. } => format!("{}.{member}", print_expr(object)),

        Expr::Call { callee, arguments, .. } => {
            let args = arguments.iter().map(print_call_argument).collect::<Vec<_>>().join(", ");

            format!("{}({args})", print_expr(callee))
        }

        Expr::EnumVariant { enum_name, generic_args, variant, payload, .. } => {
            let generics = print_generic_arguments(generic_args);
            match payload {
                Some(value) => format!("{enum_name}{generics}.{variant}({})", print_expr(value)),
                None => format!("{enum_name}{generics}.{variant}"),
            }
        }

        Expr::Unary { op, operand, .. } => format!("{}{}", print_unary_op(*op), print_expr(operand)),

        Expr::Binary { left, op, right, .. } => {
            format!("({} {} {})", print_expr(left), print_binary_op(*op), print_expr(right))
        }

        Expr::Splice { inner, .. } => format!("`{}`", print_expr(inner)),

        Expr::As { value, ty, .. } => format!("{} as {}", print_expr(value), print_type_expr(ty)),

        Expr::Here { .. } => "@here".to_string(),

        Expr::Construct { callee, generic_args, fields, .. } => {
            let generics = print_generic_arguments(generic_args);
            let body = fields.iter().map(print_construct_item).collect::<Vec<_>>().join(", ");

            format!("{}{generics} {{ {body} }}", print_expr(callee))
        }

        Expr::Range { start, end, .. } => format!("{}..{}", print_expr(start), print_expr(end)),
    }
}

fn print_generic_arguments(args: &[TypeArgument]) -> String {
    if args.is_empty() {
        return String::new();
    }

    let rendered = args
        .iter()
        .map(|arg| match arg {
            TypeArgument::Type(ty) => print_type_expr(ty),
            TypeArgument::Const(expr) => print_expr(expr),
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!("<{rendered}>")
}

fn print_construct_item(item: &ConstructItem) -> String {
    match item {
        ConstructItem::Field { name, value, .. } => {
            format!("{}: {}", print_spliced_name(name), print_expr(value))
        }

        ConstructItem::For { var, source, body, .. } => format!(
            "@for {var} in {} {{ {} }}",
            print_expr(source),
            body.iter().map(print_construct_item).collect::<Vec<_>>().join(", "),
        ),

        ConstructItem::If { condition, body, else_body, .. } => {
            let then_block = format!(
                "@if {} {{ {} }}",
                print_expr(condition),
                body.iter().map(print_construct_item).collect::<Vec<_>>().join(", "),
            );

            match else_body {
                Some(else_body) => format!(
                    "{then_block} @else {{ {} }}",
                    else_body.iter().map(print_construct_item).collect::<Vec<_>>().join(", "),
                ),

                None => then_block,
            }
        }
    }
}

fn print_call_argument(argument: &CallArgument) -> String {
    match &argument.name {
        Some(name) => format!("{name} = {}", print_expr(&argument.value)),
        None => print_expr(&argument.value),
    }
}

fn print_unary_op(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "-",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    }
}

fn print_binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Remainder => "%",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitXor => "^",
        BinaryOp::BitOr => "|",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}
