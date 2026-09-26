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
//! right — no forward references, and nothing depending on a label's
//! position (that doesn't exist until the real resolver runs).
//! This mirrors the restriction the const generic evaluator
//! ([`super::aliases::AliasResolver::eval_const_expr`]) already lives
//! under, just applied one level up, before symbol collection instead of
//! during it. A bound/condition that isn't reachable this way surfaces as
//! an ordinary [`ResolveError`] (`UnknownConstant`/`ExpectedConstantExpression`),
//! not a panic.

use std::collections::HashMap;

use crate::ast::{
    literal_name, CallArgument, Expr, MetaStatement, NamePart, Program, SplicedName, Statement,
    StructDeclaration, UnaryOp,
};
use crate::token::Span;
use crate::types::{StructBodyItem, StructField, TypeExpr};
use crate::eval::{self, EvalError, Int};
use crate::expander;

use super::fold::{NextTarget, PendingNext};
use super::macro_body::MAX_FOR_ITERATIONS;
use super::values::Value;
use super::ResolveError;

/// Like the single-`Program` form this used to be, but also returns which
/// module (index into `module_of`, one entry per `program.statements`)
/// declared each statement in the result — a `@for`/`@if` wrapper's own
/// module carries over onto everything it unrolls into, since all of it
/// traces back to the same file's source text regardless of how many
/// concrete statements it expands to. `module_of.len()` must equal
/// `program.statements.len()`.
pub fn unroll_top_level(
    program: Program,
    module_of: &[usize],
) -> Result<(Program, Vec<usize>), ResolveError> {
    let mut consts: HashMap<String, Int> = HashMap::new();
    let mut statements = Vec::new();
    let mut statement_modules = Vec::new();

    for (statement, &module) in program.statements.iter().zip(module_of) {
        let unrolled = unroll_statements(std::slice::from_ref(statement), &mut consts, &mut NextState::default())?;
        statement_modules.extend(std::iter::repeat(module).take(unrolled.len()));
        statements.extend(unrolled);
    }

    Ok((Program { statements, span: program.span }, statement_modules))
}

fn unroll_statements(
    statements: &[Statement],
    consts: &mut HashMap<String, Int>,
    next: &mut NextState,
) -> Result<Vec<Statement>, ResolveError> {
    let mut out = Vec::new();

    for statement in statements {
        // A `@next` reached so far ends this fold iteration: nothing after
        // it is unrolled.
        if next.pending.is_some() {
            break;
        }

        match statement {
            Statement::Meta(meta) => unroll_meta(meta, consts, next, &mut out)?,

            // `const x = @fold ...`: the fold's body unrolls here, in
            // place, and `x` binds its final value — see `unroll_fold`.
            Statement::Const(decl) if matches!(decl.value, Expr::Fold { .. }) => {
                let Expr::Fold { fold, .. } = &decl.value else { unreachable!("matched above") };
                let mut decl = decl.clone();
                decl.name = fold_spliced_name(&decl.name, consts)?;

                let accumulators = unroll_fold(fold, consts, next, &mut out)?;
                let Some(name) = literal_name(&decl.name) else {
                    unreachable!("fold_spliced_name leaves only literal parts");
                };

                if let [(_, value)] = accumulators.as_slice() {
                    consts.insert(name, value.clone());
                    decl.value = int_literal(value, decl.span);
                } else {
                    // Several accumulators: a generated struct with an
                    // `int` field per accumulator, constructed here. Its
                    // name can't be written in source, so it can't clash.
                    let struct_name = format!("__fold${name}");
                    out.push(Statement::Struct(fold_result_struct(&struct_name, &accumulators, decl.span)));
                    decl.value = Expr::Call {
                        callee: Box::new(Expr::Identifier { name: struct_name, span: decl.span }),
                        arguments: accumulators
                            .iter()
                            .map(|(field, value)| CallArgument {
                                name: Some(field.clone()),
                                value: int_literal(value, decl.span),
                                span: decl.span,
                            })
                            .collect(),
                        span: decl.span,
                    };
                }

                out.push(Statement::Const(decl));
            }

            Statement::Const(decl) => {
                let mut decl = decl.clone();
                decl.name = fold_spliced_name(&decl.name, consts)?;

                // Best-effort: this pass doesn't need to fully evaluate
                // the program, only track enough to unroll `@for`/`@if` —
                // a const whose value isn't staticaly evaluable this way is
                // simply not tracked, and any real error in it surfaces
                // later from `ConstEvaluator` instead.
                if let Some(name) = literal_name(&decl.name) {
                    if let Ok(value) = eval::eval(&decl.value, consts) {
                        consts.insert(name, value);
                    }
                }

                out.push(Statement::Const(decl));
            }

            // A name built with `` `expr` `` splices (see
            // `ast::NamePart`) only makes sense once its splices are
            // evaluated to literal text — normally that happens against a
            // live macro invocation's scope
            // (`resolver::values::AliasResolver::resolve_spliced_name`),
            // which doesn't exist here. But a declaration copied out of a
            // top-level `@for` body already had its loop variable
            // literalized by `expander::substitute_statements` above, so any
            // splice left in its name is just an ordinary constant
            // expression over the same `consts` a `@for` bound can
            // reference — fold it the same way.
            Statement::Struct(decl) => {
                let mut decl = decl.clone();
                decl.name = fold_spliced_name(&decl.name, consts)?;
                out.push(Statement::Struct(decl));
            }

            Statement::Enum(decl) => {
                let mut decl = decl.clone();
                decl.name = fold_spliced_name(&decl.name, consts)?;
                out.push(Statement::Enum(decl));
            }

            Statement::TypeAlias(decl) => {
                let mut decl = decl.clone();
                decl.name = fold_spliced_name(&decl.name, consts)?;
                out.push(Statement::TypeAlias(decl));
            }

            Statement::Macro(decl) => {
                let mut decl = decl.clone();
                decl.name = fold_spliced_name(&decl.name, consts)?;
                out.push(Statement::Macro(decl));
            }

            other => out.push(other.clone()),
        }
    }

    Ok(out)
}

fn unroll_meta(
    meta: &MetaStatement,
    consts: &mut HashMap<String, Int>,
    next: &mut NextState,
    out: &mut Vec<Statement>,
) -> Result<(), ResolveError> {
    match meta.name.as_str() {
        // A statement fold: its body unrolls in place, its value is unused.
        "fold" => unroll_fold(meta, consts, next, out).map(|_| ()),

        "next" => {
            let span = meta.span;
            match next.target {
                NextTarget::Fold => {}
                NextTarget::None => {
                    return Err(ResolveError::Fold {
                        message: "`@next` can only be used inside a `@fold` body".to_string(),
                        span,
                    });
                }
                NextTarget::ForInsideFold => {
                    return Err(ResolveError::Fold {
                        message: "`@next` can't be used inside a plain `@for` nested in a `@fold` — \
                                  it would have to end both loops' iterations at once"
                            .to_string(),
                        span,
                    });
                }
            }

            let value = meta.args.first().map(|value| eval_top_level_const(value, consts).map(Value::Int)).transpose()?;
            let updates = meta
                .bindings
                .iter()
                .map(|update| Ok((update.name.clone(), Value::Int(eval_top_level_const(&update.value, consts)?), update.span)))
                .collect::<Result<Vec<_>, ResolveError>>()?;

            next.pending = Some(PendingNext { value, updates, span });
            Ok(())
        }
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
            let Expr::Range { start: start_expr, end: end_expr, inclusive, .. } = source else {
                return Err(ResolveError::TopLevelForRequiresRange { span: source.span() });
            };
            let inclusive = *inclusive;

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

            while if inclusive { i <= end } else { i < end } {
                iterations += 1;

                if iterations > MAX_FOR_ITERATIONS {
                    return Err(ResolveError::ForLoopTooLarge { span: meta.span });
                }

                let mut substitutions = HashMap::new();
                substitutions.insert(var_name.clone(), int_literal(&i, meta.span));

                let literalized = expander::substitute_statements(body, &substitutions);
                let outer = next.target;
                next.target = match outer {
                    NextTarget::None => NextTarget::None,
                    _ => NextTarget::ForInsideFold,
                };
                let unrolled = unroll_statements(&literalized, consts, next);
                next.target = outer;
                out.extend(unrolled?);

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
                let unrolled = unroll_statements(chosen, consts, next)?;
                out.extend(unrolled);
            }

            Ok(())
        }

        "match" => {
            let [scrutinee] = meta.args.as_slice() else {
                return Err(ResolveError::Internal {
                    message: "top-level `@match` should always have one scrutinee — the parser \
                              guarantees this shape"
                        .to_string(),
                    span: meta.span,
                });
            };
            let value = eval_top_level_const(scrutinee, consts)?;
            let mut chosen = None;
            for arm in &meta.match_arms {
                let matches = match &arm.pattern {
                    Some(pattern) => eval_top_level_const(pattern, consts)? == value,
                    None => true,
                };
                if matches {
                    chosen = Some(&arm.body);
                    break;
                }
            }
            if let Some(body) = chosen {
                out.extend(unroll_statements(body, consts, next)?);
            }
            Ok(())
        }

        // Every other meta (`@emit`, `@return`, `@assert`) only
        // makes sense inside a macro body — the same
        // `UnsupportedMacroStatement`-shaped rejection `walk_macro_body`
        // already gives it there, just reached at the top level instead.
        other => Err(ResolveError::UnsupportedMacroStatement {
            kind: format!("@{other}"),
            span: meta.span,
        }),
    }
}

/// Where a top-level `@next` would go, and one waiting for its fold — the
/// pre-resolution counterpart of `AliasResolver::next_target` /
/// `pending_next` (see `super::fold`).
#[derive(Default)]
struct NextState {
    target: NextTarget,
    pending: Option<PendingNext>,
}

/// Unrolls a `@fold` at top level into `out`, returning each accumulator's
/// final value. Like top-level `@for`, this runs before resolution: the
/// source must be a literal range, and every initial and `@next` value an
/// integer constant expression. Each iteration's copy of the body has the
/// loop variable and every accumulator substituted as integer literals.
fn unroll_fold(
    fold: &MetaStatement,
    consts: &mut HashMap<String, Int>,
    next: &mut NextState,
    out: &mut Vec<Statement>,
) -> Result<Vec<(String, Int)>, ResolveError> {
    let [Expr::Identifier { name: var, .. }, source] = fold.args.as_slice() else {
        return Err(ResolveError::Internal {
            message: "top-level `@fold`'s args should always be [var, source] — the parser guarantees this shape"
                .to_string(),
            span: fold.span,
        });
    };
    let Expr::Range { start, end, inclusive, .. } = source else {
        return Err(ResolveError::TopLevelForRequiresRange { span: source.span() });
    };
    let body = fold.body.as_deref().unwrap_or_default();

    super::fold::check_accumulator_names(&fold.bindings, var)?;

    let mut accumulators = fold
        .bindings
        .iter()
        .map(|binding| {
            let value = eval_top_level_const(&binding.value, consts).map_err(|error| match error {
                ResolveError::ExpectedConstantExpression { span } => ResolveError::Fold {
                    message: format!(
                        "a top-level `@fold`'s accumulators must be integer constants, since top level \
                         unrolls before anything else is resolved — `{}` isn't; accumulate other values \
                         in a `@fold` inside a macro instead",
                        binding.name,
                    ),
                    span,
                },
                other => other,
            })?;
            Ok((binding.name.clone(), Value::Int(value)))
        })
        .collect::<Result<Vec<_>, ResolveError>>()?;

    let start = eval_top_level_const(start, consts)?;
    let end = eval_top_level_const(end, consts)?;

    let mut i = start;
    let mut iterations: u64 = 0;

    while if *inclusive { i <= end } else { i < end } {
        iterations += 1;
        if iterations > MAX_FOR_ITERATIONS {
            return Err(ResolveError::ForLoopTooLarge { span: fold.span });
        }

        let mut substitutions = HashMap::new();
        substitutions.insert(var.clone(), int_literal(&i, fold.span));
        for (name, value) in &accumulators {
            let Value::Int(value) = value else { unreachable!("top-level accumulators are integers") };
            substitutions.insert(name.clone(), int_literal(value, fold.span));
        }

        let literalized = expander::substitute_statements(body, &substitutions);
        let outer = std::mem::replace(&mut next.target, NextTarget::Fold);
        let unrolled = unroll_statements(&literalized, consts, next);
        next.target = outer;
        out.extend(unrolled?);

        if let Some(pending) = next.pending.take() {
            super::fold::apply_next(&mut accumulators, pending)?;
        }

        i += Int::from(1);
    }

    Ok(accumulators
        .into_iter()
        .map(|(name, value)| match value {
            Value::Int(value) => (name, value),
            _ => unreachable!("top-level accumulators are integers"),
        })
        .collect())
}

// `struct <name> { pub acc: int, ... }` — a top-level fold's result type
// when it has several accumulators.
fn fold_result_struct(name: &str, accumulators: &[(String, Int)], span: Span) -> StructDeclaration {
    StructDeclaration {
        name: vec![NamePart::Literal(name.to_string())],
        is_pub: false,
        generic_params: Vec::new(),
        facets: Vec::new(),
        fields: accumulators
            .iter()
            .map(|(field, _)| {
                StructBodyItem::Field(StructField {
                    name: vec![NamePart::Literal(field.clone())],
                    ty: TypeExpr::Named { path: vec!["int".to_string()], span },
                    is_pub: true,
                    is_skip: false,
                    default: None,
                    span,
                })
            })
            .collect(),
        span,
    }
}

// An integer as source: a negative one is `-(n)`, not a literal with a
// sign in it.
fn int_literal(value: &Int, span: Span) -> Expr {
    if *value < Int::from(0) {
        Expr::Unary {
            op: UnaryOp::Negate,
            operand: Box::new(Expr::Integer { raw: (-value.clone()).to_string(), span }),
            span,
        }
    } else {
        Expr::Integer { raw: value.to_string(), span }
    }
}

fn fold_spliced_name(
    parts: &[NamePart],
    consts: &HashMap<String, Int>,
) -> Result<SplicedName, ResolveError> {
    parts
        .iter()
        .map(|part| match part {
            NamePart::Literal(text) => Ok(NamePart::Literal(text.clone())),
            NamePart::Splice(expr) => {
                Ok(NamePart::Literal(eval_top_level_const(expr, consts)?.to_string()))
            }
        })
        .collect()
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
        let module_of = vec![0; program.statements.len()];
        unroll_top_level(program, &module_of).map(|(program, _)| program)
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

    fn const_names(program: &Program) -> Vec<String> {
        program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Const(decl) => literal_name(&decl.name),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn for_folds_a_spliced_const_name_using_the_literalized_loop_var() {
        let program = unroll("@for i in 0..3 {\n    pub const x`i` = i\n}\n").unwrap();
        assert_eq!(const_names(&program), vec!["x0", "x1", "x2"]);
    }

    #[test]
    fn for_over_an_inclusive_range_includes_the_end_bound() {
        let program = unroll("@for i in 0..=2 {\n    make_reg(i)\n}\n").unwrap();

        assert_eq!(invocation_names(&program), vec!["make_reg", "make_reg", "make_reg"]);
        assert_eq!(invocation_first_arg_raw(&program, 0), "0");
        assert_eq!(invocation_first_arg_raw(&program, 1), "1");
        assert_eq!(invocation_first_arg_raw(&program, 2), "2");
    }

    #[test]
    fn for_over_an_empty_inclusive_range_produces_one_iteration() {
        let program = unroll("@for i in 0..=0 {\n    make_reg(i)\n}\n").unwrap();
        assert_eq!(invocation_names(&program), vec!["make_reg"]);
    }
}
