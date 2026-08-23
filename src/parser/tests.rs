use super::*;
use crate::lexer::lex;

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
fn parses_const_declaration() {
    let program =
        parse(lex("const r1 = Reg(id = 1)\n").unwrap()).unwrap();

    let Statement::Const(declaration) = &program.statements[0] else {
        panic!("expected const declaration");
    };

    assert_eq!(declaration.name, "r1");

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
    assert_eq!(declaration.fields[0].name, "id");
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

    assert_eq!(declaration.name, "r0");
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
