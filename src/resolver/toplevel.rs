//! Unrolls every top-level `@for`/`@if` into concrete statements *before*
//! symbol collection or any other resolution runs — [`collect_symbols`],
//! [`super::ConstEvaluator`], and `main::walk_top_level` all see an
//! ordinary, `@for`/`@if`-free statement list, exactly as they did before
//! this pass existed.
//!
//! The mechanism is a thin combination of two things that already exist
//! for other reasons: [`eval::eval`] (the pure, context-free `Int`
//! evaluator already used for top-level consts) picks a `@for`'s range or
//! an `@if`'s condition, and [`crate::expander::substitute_statements`]
//! (already used by the `bitterasm expand` command) literal-izes a
//! `@for`'s loop variable into each iteration's copy of the body, the same
//! way it literal-izes a macro's declared parameters at a call site.
//!
//! Deliberately restricted to what's staticaly evaluable this way: a
//! `@for`'s bounds or an `@if`'s condition may only reference *earlier*
//! top-level consts, tracked as this pass sweeps the program left to
//! right — no forward references, and nothing depending on `@here` or a
//! label's position (those don't exist until the real resolver runs).
//! This mirrors the restriction the const generic evaluator
//! ([`super::aliases::AliasResolver::eval_const_expr`]) already lives
//! under, just applied one level up, before symbol collection instead of
//! during it. A bound/condition that isn't reachable this way surfaces as
//! an ordinary [`ResolveError`] (`UnknownConstant`/`ExpectedConstantExpression`),
//! not a panic.

use std::collections::HashMap;

use crate::ast::{literal_name, Expr, MetaStatement, Program, Statement};
use crate::eval::{self, EvalError, Int};
use crate::expander;

use super::macro_body::MAX_FOR_ITERATIONS;
use super::ResolveError;

pub fn unroll_top_level(program: Program) -> Result<Program, ResolveError> {
    let mut consts: HashMap<String, Int> = HashMap::new();
    let statements = unroll_statements(&program.statements, &mut consts)?;

    Ok(Program { statements, span: program.span })
}

fn unroll_statements(
    statements: &[Statement],
    consts: &mut HashMap<String, Int>,
) -> Result<Vec<Statement>, ResolveError> {
    let mut out = Vec::new();

    for statement in statements {
        match statement {
            Statement::Meta(meta) => unroll_meta(meta, consts, &mut out)?,

            Statement::Const(decl) => {
                // Best-effort: this pass doesn't need to fully evaluate
                // the program, only track enough to unroll `@for`/`@if` —
                // a const whose name or value isn't staticaly evaluable
                // this way is simply not tracked, and any real error in it
                // surfaces later from `ConstEvaluator` instead.
                if let Some(name) = literal_name(&decl.name) {
                    if let Ok(value) = eval::eval(&decl.value, consts) {
                        consts.insert(name, value);
                    }
                }

                out.push(statement.clone());
            }

            other => out.push(other.clone()),
        }
    }

    Ok(out)
}

fn unroll_meta(
    meta: &MetaStatement,
    consts: &mut HashMap<String, Int>,
    out: &mut Vec<Statement>,
) -> Result<(), ResolveError> {
    match meta.name.as_str() {
        "for" => {
            let [var, source] = meta.args.as_slice() else {
                return Err(ResolveError::Internal {
                    message: "top-level `@for`'s args should always be [var, source] — \
                              the parser guarantees this shape"
                        .to_string(),
                    span: meta.span,
                });
            };

            let Expr::Identifier { name: var_name, .. } = var else {
                return Err(ResolveError::Internal {
                    message: "top-level `@for`'s loop variable should always be an \
                              identifier — the parser guarantees this shape"
                        .to_string(),
                    span: meta.span,
                });
            };

            // Unlike the other three `@for` sites, top-level `@for` runs
            // before any symbol/struct resolution exists, so it can only
            // ever unroll literal `start..end` range sugar — see the
            // module doc and `ResolveError::TopLevelForRequiresRange`'s
            // doc.
            let Expr::Range { start: start_expr, end: end_expr, .. } = source else {
                return Err(ResolveError::TopLevelForRequiresRange { span: source.span() });
            };

            let body = meta.body.as_ref().ok_or_else(|| ResolveError::Internal {
                message: "top-level `@for` should always carry a body — the parser \
                          guarantees this shape"
                    .to_string(),
                span: meta.span,
            })?;

            let start = eval_top_level_const(start_expr, consts)?;
            let end = eval_top_level_const(end_expr, consts)?;

            let mut i = start;
            let mut iterations: u64 = 0;

            while i < end {
                iterations += 1;

                if iterations > MAX_FOR_ITERATIONS {
                    return Err(ResolveError::ForLoopTooLarge { span: meta.span });
                }

                let mut substitutions = HashMap::new();
                substitutions.insert(
                    var_name.clone(),
                    Expr::Integer { raw: i.to_string(), span: meta.span },
                );

                let literalized = expander::substitute_statements(body, &substitutions);
                let unrolled = unroll_statements(&literalized, consts)?;
                out.extend(unrolled);

                i += Int::from(1);
            }

            Ok(())
        }

        "if" => {
            let [condition] = meta.args.as_slice() else {
                return Err(ResolveError::Internal {
                    message: "top-level `@if`'s args should always be a single condition — \
                              the parser guarantees this shape"
                        .to_string(),
                    span: meta.span,
                });
            };

            let truthy = eval_top_level_const(condition, consts)? != Int::from(0);
            let chosen = if truthy { meta.body.as_ref() } else { meta.else_body.as_ref() };

            if let Some(chosen) = chosen {
                let unrolled = unroll_statements(chosen, consts)?;
                out.extend(unrolled);
            }

            Ok(())
        }

        // Every other meta (`@emit`, `@return`, `@assert`, `@here`) only
        // makes sense inside a macro body — the same
        // `UnsupportedMacroStatement`-shaped rejection `walk_macro_body`
        // already gives it there, just reached at the top level instead.
        other => Err(ResolveError::UnsupportedMacroStatement {
            kind: format!("@{other}"),
            span: meta.span,
        }),
    }
}

fn eval_top_level_const(expr: &Expr, consts: &HashMap<String, Int>) -> Result<Int, ResolveError> {
    eval::eval(expr, consts).map_err(|error| match error {
        EvalError::UnknownConstant { name, span } => ResolveError::UnknownConstant { name, span },
        EvalError::NotConstant { span } => ResolveError::ExpectedConstantExpression { span },
        EvalError::DivisionByZero { span } => ResolveError::DivisionByZero { span },
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer;
    use crate::parser;

    use super::*;

    fn unroll(source: &str) -> Result<Program, ResolveError> {
        let tokens = lexer::lex(source).expect("fixture should lex");
        let program = parser::parse(tokens).expect("fixture should parse");
        unroll_top_level(program)
    }

    fn invocation_names(program: &Program) -> Vec<&str> {
        program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) => Some(invocation.name.as_str()),
                _ => None,
            })
            .collect()
    }

    fn invocation_first_arg_raw(program: &Program, index: usize) -> &str {
        let Statement::Invocation(invocation) =
            program.statements.iter().filter(|s| matches!(s, Statement::Invocation(_))).nth(index).unwrap()
        else {
            unreachable!();
        };

        let Expr::Integer { raw, .. } = &invocation.operands[0] else {
            panic!("expected an integer operand");
        };

        raw.as_str()
    }

    #[test]
    fn for_unrolls_into_one_invocation_per_iteration_with_the_loop_var_literalized() {
        let program = unroll("@for i in 0..3 {\n    make_reg(i)\n}\n").unwrap();

        assert_eq!(invocation_names(&program), vec!["make_reg", "make_reg", "make_reg"]);
        assert_eq!(invocation_first_arg_raw(&program, 0), "0");
        assert_eq!(invocation_first_arg_raw(&program, 1), "1");
        assert_eq!(invocation_first_arg_raw(&program, 2), "2");
    }

    #[test]
    fn for_over_an_empty_range_produces_nothing() {
        let program = unroll("@for i in 0..0 {\n    make_reg(i)\n}\n").unwrap();
        assert!(invocation_names(&program).is_empty());
    }

    #[test]
    fn for_bound_may_reference_an_earlier_top_level_const() {
        let program = unroll("const n = 2\n@for i in 0..n {\n    make_reg(i)\n}\n").unwrap();
        assert_eq!(invocation_names(&program), vec!["make_reg", "make_reg"]);
    }

    #[test]
    fn for_bound_referencing_a_later_const_is_rejected_as_a_forward_reference() {
        let error = unroll("@for i in 0..n {\n    make_reg(i)\n}\nconst n = 2\n").unwrap_err();
        assert!(matches!(error, ResolveError::UnknownConstant { .. }));
    }

    // Top-level `@for` is the one call site that stays restricted to
    // `start..end` sugar (see the module doc and
    // `ResolveError::TopLevelForRequiresRange`'s doc) — a deliberate,
    // confirmed exception to the other three sites' "iterate any struct's
    // pub fields" generality, since this pass runs before any symbol/struct
    // resolution exists to make that possible.
    #[test]
    fn for_over_a_non_range_source_is_rejected_with_a_dedicated_error() {
        let error = unroll("@for i in some_name {\n    make_reg(i)\n}\n").unwrap_err();
        assert!(matches!(error, ResolveError::TopLevelForRequiresRange { .. }));
    }

    #[test]
    fn if_true_keeps_the_then_branch_and_drops_the_else_branch() {
        let program = unroll("@if 1 {\n    then_branch\n} @else {\n    else_branch\n}\n").unwrap();
        assert_eq!(invocation_names(&program), vec!["then_branch"]);
    }

    #[test]
    fn if_false_keeps_the_else_branch() {
        let program = unroll("@if 0 {\n    then_branch\n} @else {\n    else_branch\n}\n").unwrap();
        assert_eq!(invocation_names(&program), vec!["else_branch"]);
    }

    #[test]
    fn nested_for_inside_for_unrolls_fully() {
        let program = unroll("@for i in 0..2 {\n    @for j in 0..2 {\n        pair(i)\n    }\n}\n").unwrap();
        assert_eq!(invocation_names(&program), vec!["pair", "pair", "pair", "pair"]);
    }

    #[test]
    fn a_bare_top_level_emit_is_rejected() {
        let error = unroll("@emit 5\n").unwrap_err();
        assert!(matches!(error, ResolveError::UnsupportedMacroStatement { .. }));
    }
}
