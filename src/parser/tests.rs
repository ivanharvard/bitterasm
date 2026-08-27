use super::*;
use crate::ast::literal_name;
use crate::lexer::lex;
use crate::types::StructBodyItem;

#[test]
fn parses_empty_program() {
    let program = parse(lex("").unwrap()).unwrap();

    assert!(program.statements.is_empty());
}

#[test]
fn parses_import_all() {
    let program =
        parse(lex("from tinycpu.native import *\n").unwrap()).unwrap();

    assert_eq!(program.statements.len(), 1);

    let Statement::Import(import) = &program.statements[0] else {
        panic!("expected import");
    };

    assert_eq!(
        import.module.segments,
        vec!["tinycpu".to_string(), "native".to_string()]
    );

    assert_eq!(import.module.relative_level, 0);
    assert_eq!(import.items, ImportItems::All);
}

#[test]
fn parses_named_imports() {
    let program =
        parse(lex("from tinycpu.native import mov, add\n").unwrap())
            .unwrap();

    let Statement::Import(import) = &program.statements[0] else {
        panic!("expected import");
    };

    assert_eq!(
        import.items,
        ImportItems::Names(vec![
            "mov".to_string(),
            "add".to_string(),
        ])
    );
}

#[test]
fn parses_relative_import() {
    let program =
        parse(lex("from .foobar import qux\n").unwrap()).unwrap();

    let Statement::Import(import) = &program.statements[0] else {
        panic!("expected import");
    };

    assert_eq!(import.module.relative_level, 1);
    assert_eq!(import.module.segments, vec!["foobar".to_string()]);
}

#[test]
fn parses_label() {
    let program = parse(lex("start:\n").unwrap()).unwrap();

    let Statement::Label(label) = &program.statements[0] else {
        panic!("expected label");
    };

    assert_eq!(label.name, "start");
}

#[test]
fn parses_no_operand_invocation() {
    let program = parse(lex("nop\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    assert_eq!(invocation.name, "nop");
    assert!(invocation.operands.is_empty());
}

#[test]
fn parses_invocation() {
    let program = parse(lex("mov r1, 7\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    assert_eq!(invocation.name, "mov");
    assert_eq!(invocation.operands.len(), 2);

    assert!(matches!(
        &invocation.operands[0],
        Expr::Identifier { name, .. } if name == "r1"
    ));

    assert!(matches!(
        &invocation.operands[1],
        Expr::Integer { raw, .. } if raw == "7"
    ));
}

#[test]
fn parses_whole_example() {
    let source = r#"from tinycpu.native import *

start:
    mov r1, 7
    add r1, r2
    nop
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    assert_eq!(program.statements.len(), 5);

    assert!(matches!(
        program.statements[0],
        Statement::Import(_)
    ));

    assert!(matches!(
        program.statements[1],
        Statement::Label(_)
    ));

    assert!(matches!(
        program.statements[2],
        Statement::Invocation(_)
    ));

    assert!(matches!(
        program.statements[3],
        Statement::Invocation(_)
    ));

    assert!(matches!(
        program.statements[4],
        Statement::Invocation(_)
    ));
}

#[test]
fn accepts_no_final_newline() {
    let program = parse(lex("nop").unwrap()).unwrap();

    assert_eq!(program.statements.len(), 1);
}

#[test]
fn rejects_trailing_comma() {
    let error = parse(lex("mov r1,\n").unwrap()).unwrap_err();

    assert_eq!(error.message, "expected operand after comma");
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    let program =
        parse(lex("mov r1, 1 + 2 * 3\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Binary {
        op: BinaryOp::Add,
        left,
        right,
        ..
    } = &invocation.operands[1]
    else {
        panic!("expected addition");
    };

    assert!(matches!(
        left.as_ref(),
        Expr::Integer { raw, .. } if raw == "1"
    ));

    assert!(matches!(
        right.as_ref(),
        Expr::Binary {
            op: BinaryOp::Multiply,
            ..
        }
    ));
}

#[test]
fn parses_member_access() {
    let program =
        parse(lex("mov r1, foo.bar.baz\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Member { member, object, .. } =
        &invocation.operands[1]
    else {
        panic!("expected member access");
    };

    assert_eq!(member, "baz");

    assert!(matches!(
        object.as_ref(),
        Expr::Member { member, .. } if member == "bar"
    ));
}

#[test]
fn parses_function_call() {
    let program =
        parse(lex("mov r1, foo(1, 2)\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Call {
        callee,
        arguments,
        ..
    } = &invocation.operands[1]
    else {
        panic!("expected call");
    };

    assert!(matches!(
        callee.as_ref(),
        Expr::Identifier { name, .. } if name == "foo"
    ));

    assert_eq!(arguments.len(), 2);
}

#[test]
fn parses_call_arguments_spanning_multiple_lines() {
    let program = parse(
        lex("mov r1, foo(\n    1,\n    2,\n)\n").unwrap(),
    )
    .unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Call { arguments, .. } = &invocation.operands[1] else {
        panic!("expected call");
    };

    assert_eq!(arguments.len(), 2);
}

#[test]
fn parses_macro_parameters_spanning_multiple_lines() {
    let program = parse(
        lex("macro foo(\n    a: int,\n    b: int\n) {\n    @emit a\n}\n").unwrap(),
    )
    .unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    assert_eq!(decl.params.len(), 2);
}

#[test]
fn parses_generic_arguments_spanning_multiple_lines() {
    let program = parse(
        lex("const x: Reg<\n    64\n> = 1\n").unwrap(),
    )
    .unwrap();

    let Statement::Const(decl) = &program.statements[0] else {
        panic!("expected const declaration");
    };

    assert!(decl.ty.is_some());
}

#[test]
fn parses_named_arguments() {
    let program =
        parse(lex("mov r1, Reg(id = 7)\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Call { arguments, .. } =
        &invocation.operands[1]
    else {
        panic!("expected call");
    };

    assert_eq!(arguments.len(), 1);
    assert_eq!(arguments[0].name.as_deref(), Some("id"));

    assert!(matches!(
        &arguments[0].value,
        Expr::Integer { raw, .. } if raw == "7"
    ));
}

#[test]
fn parses_unary_expression() {
    let program =
        parse(lex("mov r1, -foo\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    assert!(matches!(
        &invocation.operands[1],
        Expr::Unary {
            op: UnaryOp::Negate,
            ..
        }
    ));
}

#[test]
fn parses_as_as_an_ordinary_expression_operator() {
    let program = parse(lex("const byte = value as Byte\n").unwrap()).unwrap();
    let Statement::Const(decl) = &program.statements[0] else {
        panic!("expected const declaration")
    };
    assert!(matches!(
        &decl.value,
        Expr::As { value, ty, .. }
            if matches!(value.as_ref(), Expr::Identifier { name, .. } if name == "value")
                && ty.name() == Some("Byte")
    ));
}

#[test]
fn rejects_the_old_at_as_spelling() {
    let error = parse(lex("const byte = value @as Byte\n").unwrap()).unwrap_err();
    assert!(error.message.contains("constant value"));
}

#[test]
fn parses_splice_expression() {
    let program =
        parse(lex("mov r1, `foo + 1`\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Splice { inner, .. } = &invocation.operands[1] else {
        panic!("expected splice");
    };

    assert!(matches!(inner.as_ref(), Expr::Binary { .. }));
}

#[test]
fn parses_here_expression_inline() {
    let program =
        parse(lex("mov r1, target - @here\n").unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[0] else {
        panic!("expected invocation");
    };

    let Expr::Binary { right, .. } = &invocation.operands[1] else {
        panic!("expected binary expression");
    };

    assert!(matches!(right.as_ref(), Expr::Here { .. }));
}

#[test]
fn here_expression_rejects_any_other_at_name() {
    let error = parse(lex("mov r1, @foo\n").unwrap()).unwrap_err();

    assert!(
        format!("{error}").contains("here"),
        "expected the error to mention `here`, got: {error}"
    );
}

#[test]
fn bare_here_statement_still_parses_as_a_meta_statement() {
    // `@here` only makes sense inline (`target - @here`); a bare `@here`
    // line parses fine (same generic `@`-directive grammar as `@emit`) but
    // is left for the resolver to reject — see
    // `resolver::macro_body::tests::bare_here_statement_is_unsupported`.
    let program = parse(
        lex("macro foo() {\n    @here\n}\n").unwrap(),
    )
    .unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    assert!(matches!(
        &decl.body[0],
        Statement::Meta(meta) if meta.name == "here"
    ));
}

#[test]
fn parses_for_meta_with_range_and_body() {
    let program = parse(
        lex("macro foo() {\n    @for i in 0..16 {\n        @emit i\n    }\n}\n").unwrap(),
    )
    .unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    let Statement::Meta(meta) = &decl.body[0] else {
        panic!("expected meta statement");
    };

    assert_eq!(meta.name, "for");
    assert!(meta.else_body.is_none());

    let [var, source] = meta.args.as_slice() else {
        panic!("expected [var, source] args, got {:?}", meta.args);
    };

    assert!(matches!(var, Expr::Identifier { name, .. } if name == "i"));

    let Expr::Range { start, end, .. } = source else {
        panic!("expected a Range source, got {source:?}");
    };

    assert!(matches!(start.as_ref(), Expr::Integer { raw, .. } if raw == "0"));
    assert!(matches!(end.as_ref(), Expr::Integer { raw, .. } if raw == "16"));

    let body = meta.body.as_ref().expect("@for should carry a body");
    assert_eq!(body.len(), 1);
    assert!(matches!(&body[0], Statement::Meta(inner) if inner.name == "emit"));
}

#[test]
fn parses_if_without_else() {
    let program = parse(
        lex("macro foo() {\n    @if x == 1 {\n        @emit 1\n    }\n}\n").unwrap(),
    )
    .unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    let Statement::Meta(meta) = &decl.body[0] else {
        panic!("expected meta statement");
    };

    assert_eq!(meta.name, "if");
    assert_eq!(meta.args.len(), 1);
    assert!(meta.body.is_some());
    assert!(meta.else_body.is_none());
}

#[test]
fn parses_if_else_on_the_same_line_as_the_closing_brace() {
    let program = parse(
        lex("macro foo() {\n    @if x == 1 {\n        @emit 1\n    } @else {\n        @emit 2\n    }\n}\n")
            .unwrap(),
    )
    .unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    let Statement::Meta(meta) = &decl.body[0] else {
        panic!("expected meta statement");
    };

    assert_eq!(meta.name, "if");

    let then_body = meta.body.as_ref().expect("@if should carry a body");
    let else_body = meta.else_body.as_ref().expect("expected an @else body");

    assert!(matches!(&then_body[0], Statement::Meta(inner) if inner.name == "emit"));
    assert!(matches!(&else_body[0], Statement::Meta(inner) if inner.name == "emit"));
}

#[test]
fn parses_nested_if_without_for() {
    let program = parse(
        lex("macro foo() {\n    @if a {\n        @if b {\n            @emit 1\n        } @else {\n            @emit 2\n        }\n    }\n}\n")
            .unwrap(),
    )
    .unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    let Statement::Meta(outer) = &decl.body[0] else {
        panic!("expected meta statement");
    };

    let outer_body = outer.body.as_ref().expect("outer @if should carry a body");
    assert!(matches!(&outer_body[0], Statement::Meta(inner) if inner.name == "if" && inner.else_body.is_some()));
}

#[test]
fn for_meta_requires_the_in_keyword() {
    let error = parse(lex("macro foo() {\n    @for i 0..16 {\n    }\n}\n").unwrap()).unwrap_err();

    assert!(
        format!("{error}").contains("in"),
        "expected the error to mention `in`, got: {error}"
    );
}

// `@for`'s `in`-clause is any expression, not just `start..end` sugar — a
// non-range source (like a bare `0` here) is syntactically fine; whether
// it's actually iterable (a `Value::Struct`) is a resolve-time question,
// not a parse-time one. See `resolver::generated::eval_for_source`.
#[test]
fn for_meta_accepts_any_expression_as_its_source() {
    let program = parse(lex("macro foo() {\n    @for i in 0 {\n    }\n}\n").unwrap()).unwrap();

    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro declaration");
    };

    let Statement::Meta(meta) = &decl.body[0] else {
        panic!("expected meta statement");
    };

    let [_, source] = meta.args.as_slice() else {
        panic!("expected [var, source] args, got {:?}", meta.args);
    };

    assert!(matches!(source, Expr::Integer { raw, .. } if raw == "0"));
}

#[test]
fn parses_match_with_rust_style_arms() {
    let program = parse(
        lex("macro foo(x: int) {\n    @match x {\n        0 => { @emit 1\n        },\n        _ => { @emit 2\n        }\n    }\n}\n")
            .unwrap(),
    )
    .unwrap();
    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected macro")
    };
    let Statement::Meta(meta) = &decl.body[0] else {
        panic!("expected meta")
    };
    assert_eq!(meta.name, "match");
    assert_eq!(meta.match_arms.len(), 2);
    assert!(matches!(meta.match_arms[0].pattern, Some(Expr::Integer { .. })));
    assert!(meta.match_arms[1].pattern.is_none());
}

#[test]
fn parses_const_declaration() {
    let program =
        parse(lex("const r1 = Reg(id = 1)\n").unwrap()).unwrap();

    let Statement::Const(declaration) = &program.statements[0] else {
        panic!("expected const declaration");
    };

    assert_eq!(literal_name(&declaration.name), Some("r1".to_string()));

    let Expr::Call {
        callee,
        arguments,
        ..
    } = &declaration.value
    else {
        panic!("expected call expression");
    };

    assert!(matches!(
        callee.as_ref(),
        Expr::Identifier { name, .. } if name == "Reg"
    ));

    assert_eq!(arguments.len(), 1);
    assert_eq!(arguments[0].name.as_deref(), Some("id"));
}

#[test]
fn parses_generic_struct() {
    let source = r#"
struct Reg<const width: uint> {
    id: bits<2>
}
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::Struct(declaration) =
        &program.statements[0]
    else {
        panic!("expected struct");
    };

    assert_eq!(declaration.name, "Reg");
    assert_eq!(declaration.generic_params.len(), 1);
    assert_eq!(declaration.fields.len(), 1);

    let StructBodyItem::Field(field) = &declaration.fields[0] else {
        panic!("expected a plain field");
    };

    assert_eq!(literal_name(&field.name), Some("id".to_string()));
}

#[test]
fn parses_struct_field_pub_const_and_default_modifiers() {
    let source = r#"
struct Reg<const width: int> {
    pub const id: bits<2>,
    pub len: int = width
}
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::Struct(declaration) = &program.statements[0] else {
        panic!("expected struct");
    };

    assert_eq!(declaration.fields.len(), 2);

    let StructBodyItem::Field(id_field) = &declaration.fields[0] else {
        panic!("expected a plain field");
    };

    assert_eq!(literal_name(&id_field.name), Some("id".to_string()));
    assert!(id_field.is_pub);
    assert!(id_field.is_const);
    assert!(id_field.default.is_none());

    let StructBodyItem::Field(len_field) = &declaration.fields[1] else {
        panic!("expected a plain field");
    };

    assert_eq!(literal_name(&len_field.name), Some("len".to_string()));
    assert!(len_field.is_pub);
    assert!(!len_field.is_const);
    assert!(matches!(
        len_field.default,
        Some(Expr::Identifier { ref name, .. }) if name == "width"
    ));
}

#[test]
fn parses_pub_enum() {
    let program = parse(lex("pub enum Endian {\n    Little,\n    Big\n}\n").unwrap()).unwrap();

    let Statement::Enum(decl) = &program.statements[0] else {
        panic!("expected enum declaration");
    };

    assert_eq!(decl.name, "Endian");
    assert!(decl.is_pub);
    assert_eq!(decl.variants.iter().map(|variant| variant.name.as_str()).collect::<Vec<_>>(), vec!["Little", "Big"]);
    assert!(decl.variants.iter().all(|variant| variant.payload.is_none()));
}

#[test]
fn parses_generic_enum_with_payload_variant() {
    let program = parse(lex("pub enum Option<T> {\n    Some: T,\n    None\n}\n").unwrap()).unwrap();
    let Statement::Enum(decl) = &program.statements[0] else { panic!("expected enum") };
    assert_eq!(decl.generic_params.len(), 1);
    assert_eq!(decl.variants[0].name, "Some");
    assert!(decl.variants[0].payload.is_some());
    assert_eq!(decl.variants[1].name, "None");
    assert!(decl.variants[1].payload.is_none());
}

#[test]
fn parses_type_alias() {
    let source = "type Reg64 = Reg<64>\n";

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::TypeAlias(alias) =
        &program.statements[0]
    else {
        panic!("expected type alias");
    };

    assert_eq!(alias.name, "Reg64");
}

#[test]
fn parses_typed_const() {
    let source =
        "const r0: Reg2 = Reg2(id = 0)\n";

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::Const(declaration) =
        &program.statements[0]
    else {
        panic!("expected const");
    };

    assert_eq!(literal_name(&declaration.name), Some("r0".to_string()));
    assert!(declaration.ty.is_some());
}

#[test]
fn parses_tinycpu_types() {
    let source = r#"
struct Reg<const width: uint> {
    id: bits<2>
}

type Reg2 = Reg<2>

const r0: Reg2 = Reg2(id = 0)
const r1: Reg2 = Reg2(id = 1)
const r2: Reg2 = Reg2(id = 2)
const r3: Reg2 = Reg2(id = 3)
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    assert_eq!(program.statements.len(), 6);
}
#[test]
fn resolves_identifier_const_generic_via_signature() {
    let source = r#"
struct Reg<const width: uint> {
    id: bits<2>
}

const w: uint = 4
type R = Reg<w>
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::TypeAlias(alias) = &program.statements[2] else {
        panic!("expected type alias");
    };

    let TypeExpr::Apply { args, .. } = &alias.ty else {
        panic!("expected generic application");
    };

    assert!(matches!(args[0], TypeArgument::Const(_)), "expected const arg, got {:?}", args[0]);
}

#[test]
fn negative_const_generic_argument() {
    let source = r#"
struct Foo<const n: int> {
    id: bits<2>
}

type Bar = Foo<-1>
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::TypeAlias(alias) = &program.statements[1] else {
        panic!("expected type alias");
    };

    let TypeExpr::Apply { args, .. } = &alias.ty else {
        panic!("expected generic application");
    };

    assert!(matches!(args[0], TypeArgument::Const(_)), "expected const arg, got {:?}", args[0]);
}

#[test]
fn shift_and_bitwise_ops_are_allowed_in_generic_arguments() {
    // Only `>`-shaped tokens are ambiguous with closing the argument list
    // (matching C++, where `Foo<vector<int>>` needs the `>>` split but
    // `Foo<1 << 2>` doesn't need parens) — `<<` and bitwise ops should
    // parse here without needing to be wrapped in parens.
    let source = "type A = bits<2 << 2>\n";

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::TypeAlias(alias) = &program.statements[0] else {
        panic!("expected type alias");
    };

    let TypeExpr::Apply { args, .. } = &alias.ty else {
        panic!("expected generic application");
    };

    let TypeArgument::Const(expr) = &args[0] else {
        panic!("expected const arg, got {:?}", args[0]);
    };

    assert!(
        matches!(expr, Expr::Binary { op: BinaryOp::ShiftLeft, .. }),
        "expected a shift-left expression, got {expr:?}",
    );
}

#[test]
fn bare_greater_than_in_generic_argument_still_closes_the_list() {
    // Unlike `<<`, a bare (unparenthesized) `>` genuinely is ambiguous with
    // closing the argument list, so it can't be treated as a comparison
    // here — matching how C++ requires `Foo<(a > b)>` for the same reason.
    let source = "type A = bits<4 > 2>\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn resolves_generic_argument_via_forward_declared_signature() {
    // `Reg<w>` is used before `Reg` and `w` are declared, so only the
    // pre-pass (which sees the whole file first) lets this resolve to a
    // Const argument instead of misclassifying `w` as a Type argument.
    let source = r#"
type R = Reg<w>

struct Reg<const width: uint> {
    id: bits<2>
}

const w: uint = 4
"#;

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::TypeAlias(alias) = &program.statements[0] else {
        panic!("expected type alias");
    };

    let TypeExpr::Apply { args, .. } = &alias.ty else {
        panic!("expected generic application");
    };

    assert!(matches!(args[0], TypeArgument::Const(_)), "expected const arg, got {:?}", args[0]);
}

#[test]
fn custom_syntax_invocation_matches_default_shape() {
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$, $value$\" {\n}\n\nmov r1, 7\n";

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[1] else {
        panic!("expected invocation");
    };

    assert_eq!(invocation.name, "mov");
    assert_eq!(invocation.operands.len(), 2);

    assert!(matches!(
        &invocation.operands[0],
        Expr::Identifier { name, .. } if name == "r1"
    ));

    assert!(matches!(
        &invocation.operands[1],
        Expr::Integer { raw, .. } if raw == "7"
    ));
}

#[test]
fn macro_without_syntax_facet_is_unaffected() {
    // A sibling macro with custom syntax must not leak its pattern onto a
    // different identifier.
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$, $value$\" {\n}\n\n\
                  macro add(a: int, b: int) {\n}\n\nadd r1, r2\n";

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[2] else {
        panic!("expected invocation");
    };

    assert_eq!(invocation.name, "add");
    assert_eq!(invocation.operands.len(), 2);
}

#[test]
fn custom_syntax_with_operator_shaped_literal_separator() {
    // `<-` lexes as `Less` then `Minus` (there's no dedicated `<-` token) —
    // capturing `$dst$` without a stop boundary would swallow `r1 <- 7` as
    // `r1 < (-7)` in one shot, leaving nothing for `$value$`.
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$ <- $value$\" {\n}\n\nmov r1 <- 7\n";

    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::Invocation(invocation) = &program.statements[1] else {
        panic!("expected invocation");
    };

    assert_eq!(invocation.name, "mov");

    assert!(matches!(
        &invocation.operands[0],
        Expr::Identifier { name, .. } if name == "r1"
    ));

    assert!(matches!(
        &invocation.operands[1],
        Expr::Integer { raw, .. } if raw == "7"
    ));
}

#[test]
fn syntax_facet_rejects_unbalanced_dollar() {
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst, $value$\" {\n}\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn syntax_facet_rejects_unknown_capture_name() {
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$, $bogus$\" {\n}\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn syntax_facet_rejects_uncaptured_param() {
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$\" {\n}\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn syntax_facet_rejects_duplicate_capture() {
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$, $dst$\" {\n}\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn syntax_facet_rejects_captures_with_no_literal_between_them() {
    let adjacent = "macro mov(dst: int, value: int) | syntax \"mov $dst$$value$\" {\n}\n";
    assert!(parse(lex(adjacent).unwrap()).is_err());

    let space_only = "macro mov(dst: int, value: int) | syntax \"mov $dst$ $value$\" {\n}\n";
    assert!(parse(lex(space_only).unwrap()).is_err());
}

#[test]
fn syntax_facet_rejects_pattern_not_starting_with_macro_name() {
    let source = "macro mov(dst: int, value: int) | syntax \"$dst$, mov\" {\n}\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn custom_syntax_call_site_mismatch_is_a_parse_error() {
    let source = "macro mov(dst: int, value: int) | syntax \"mov $dst$, $value$\" {\n}\n\nmov r1 - 7\n";

    assert!(parse(lex(source).unwrap()).is_err());
}

#[test]
fn type_alias_accepts_facets_on_following_lines() {
    let source = "type UByte = bits<8>\n    | invariant value >= 0\n    | invariant value < 256\nconst next = 1\n";
    let program = parse(lex(source).unwrap()).unwrap();

    let Statement::TypeAlias(alias) = &program.statements[0] else {
        panic!("expected a type alias");
    };
    assert_eq!(alias.facets.len(), 2);
    assert!(matches!(program.statements[1], Statement::Const(_)));
}

#[test]
fn pub_and_return_type_are_macro_signature_fields_not_facets() {
    let source = "pub macro encode(value: int) -> bits<8>\n    | syntax \"encode $value$\"\n{\n}\n";
    let program = parse(lex(source).unwrap()).unwrap();
    let Statement::Macro(decl) = &program.statements[0] else {
        panic!("expected a macro");
    };
    assert!(decl.is_pub);
    assert!(decl.return_ty.is_some());
    assert_eq!(decl.facets.len(), 1);
    assert_eq!(decl.facets[0].name, "syntax");

    assert!(parse(lex("macro encode()\n| pub\n{\n}\n").unwrap()).is_err());
    assert!(parse(lex("macro encode()\n| -> int\n{\n}\n").unwrap()).is_err());
}
