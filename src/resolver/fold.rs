//! `@fold acc = init, ... @for var in source { body }` and `@next` in macro
//! bodies — see "`@fold`" in `docs/1.0/PROGRESS.md` for the rules this
//! implements.
//!
//! A fold walks `source` exactly like `@for` does, but each iteration's
//! scope also binds every accumulator to its current value. `@next` inside
//! the body records the next iteration's values in
//! [`AliasResolver::pending_next`] and ends the iteration early, the same
//! way `@return` ends a body through `MacroExpansion::returned`.
//!
//! Also home to the shared rule for where a value-producing construct that
//! `@emit`s may appear ([`AliasResolver::eval_value_keeping_emits`]): a
//! `@fold` and a macro call are treated identically.

use std::collections::HashMap;

use crate::ast::{Expr, FoldBinding, MetaStatement, NamePart, Statement, StructDeclaration};
use crate::token::Span;
use crate::types::{GenericParameter, StructBodyItem, StructField, TypeExpr};

use super::aliases::AliasResolver;
use super::macro_body::MacroExpansion;
use super::symbols::SymbolKind;
use super::types::{ResolvedGenericArg, ResolvedType};
use super::values::Value;
use super::ResolveError;

/// Where a `@next` reached right now would go. Saved and restored around
/// every fold body, plain `@for` body and macro call, so a `@next` always
/// belongs to the innermost fold in its own macro body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum NextTarget {
    /// Not inside any `@fold` body in this macro.
    #[default]
    None,
    /// Directly inside a `@fold` body (possibly under `@if`/`@match`).
    Fold,
    /// Inside a plain `@for` that is itself inside a `@fold` body.
    ForInsideFold,
}

/// A `@next`'s already-evaluated values, waiting for its fold to apply them.
#[derive(Debug, Clone)]
pub(super) struct PendingNext {
    pub(super) value: Option<Value>,
    pub(super) updates: Vec<(String, Value, Span)>,
    pub(super) span: Span,
}

/// What running one `@fold` produced.
pub(super) struct FoldOutcome {
    pub(super) emitted: Vec<Value>,
    pub(super) generated: Vec<Statement>,
    /// `Some` when the body left the enclosing macro early — through
    /// `@return` (`Some(returned)`) or a self tail call
    /// (`Some(None)`, with `pending_tail_call` set) — rather than finishing
    /// its iterations. `value` is meaningless then.
    pub(super) exited: Option<Option<Value>>,
    pub(super) value: Value,
}

/// A value computed where its `@emit`s and generated declarations are kept:
/// either it produced a value, or the enclosing macro must return now.
pub(super) enum KeptValue {
    Value(Value),
    Exited(Option<Value>),
}

impl<'a> AliasResolver<'a> {
    /// Runs one `@fold` meta statement against `scope`.
    pub(super) fn run_fold(
        &mut self,
        fold: &MetaStatement,
        scope: &HashMap<String, Value>,
        allowed_emits: &[ResolvedType],
    ) -> Result<FoldOutcome, ResolveError> {
        let [Expr::Identifier { name: var, .. }, source] = fold.args.as_slice() else {
            return Err(ResolveError::Internal {
                message: "`@fold`'s args should always be [var, source] — the parser guarantees this shape"
                    .to_string(),
                span: fold.span,
            });
        };
        let body = fold.body.as_deref().unwrap_or_default();

        check_accumulator_names(&fold.bindings, var)?;

        let mut accumulators = Vec::with_capacity(fold.bindings.len());
        for binding in &fold.bindings {
            accumulators.push((binding.name.clone(), self.eval_value(&binding.value, scope)?));
        }

        let elements = self.eval_for_source(source, scope)?;

        let mut emitted = Vec::new();
        let mut generated = Vec::new();

        for (_, element) in elements {
            let mut iter_scope = scope.clone();
            iter_scope.insert(var.clone(), element);
            for (name, value) in &accumulators {
                iter_scope.insert(name.clone(), value.clone());
            }

            let outer_target = std::mem::replace(&mut self.next_target, NextTarget::Fold);
            let nested = self.walk_macro_body(body, &iter_scope, allowed_emits);
            self.next_target = outer_target;
            let nested = nested?;

            emitted.extend(nested.emitted);
            generated.extend(nested.generated);

            if nested.returned.is_some() || self.pending_tail_call.is_some() {
                return Ok(FoldOutcome {
                    emitted,
                    generated,
                    exited: Some(nested.returned),
                    value: Value::Int(0.into()),
                });
            }

            if let Some(next) = self.pending_next.take() {
                apply_next(&mut accumulators, next)?;
            }
        }

        let value = self.fold_result(accumulators, fold.span)?;
        Ok(FoldOutcome { emitted, generated, exited: None, value })
    }

    /// One accumulator's final value, or a struct of all of them — one
    /// `pub` field per accumulator, in declaration order, each typed by its
    /// own generic parameter so accumulators of any type fit. Synthesized
    /// the same way `start..end` synthesizes a `__range#N` struct.
    fn fold_result(&mut self, mut accumulators: Vec<(String, Value)>, span: Span) -> Result<Value, ResolveError> {
        if accumulators.len() == 1 {
            return Ok(accumulators.pop().expect("checked length").1);
        }

        let name = format!("__fold#{}", self.generated_symbols.len());
        let params: Vec<String> = (0..accumulators.len()).map(|index| format!("T{index}")).collect();

        let decl = Statement::Struct(StructDeclaration {
            name: vec![NamePart::Literal(name.clone())],
            is_pub: false,
            generic_params: params
                .iter()
                .map(|param| GenericParameter::Type { name: param.clone(), bound: None, span })
                .collect(),
            facets: Vec::new(),
            fields: accumulators
                .iter()
                .zip(&params)
                .map(|((field, _), param)| {
                    StructBodyItem::Field(StructField {
                        name: vec![NamePart::Literal(field.clone())],
                        ty: TypeExpr::Named { path: vec![param.clone()], span },
                        is_pub: true,
                        is_skip: false,
                        default: None,
                        span,
                    })
                })
                .collect(),
            span,
        });

        self.register_generated(&decl)?;
        let symbol = self.lookup_symbol(&name).expect("register_generated just inserted this name");

        let args = accumulators
            .iter()
            .map(|(_, value)| Ok(ResolvedGenericArg::Type(Box::new(self.value_type(value)?))))
            .collect::<Result<Vec<_>, ResolveError>>()?;

        Ok(Value::Struct { symbol, args, fields: accumulators, nominal: None })
    }

    /// Evaluates `@next`'s values in the current iteration's scope and
    /// records them for the enclosing fold, or errors if there's no fold
    /// for them to go to.
    pub(super) fn record_next(
        &mut self,
        next: &MetaStatement,
        scope: &HashMap<String, Value>,
    ) -> Result<(), ResolveError> {
        match self.next_target {
            NextTarget::Fold => {}
            NextTarget::None => {
                return Err(ResolveError::Fold {
                    message: "`@next` can only be used inside a `@fold` body".to_string(),
                    span: next.span,
                });
            }
            NextTarget::ForInsideFold => {
                return Err(ResolveError::Fold {
                    message: "`@next` can't be used inside a plain `@for` nested in a `@fold` — \
                              it would have to end both loops' iterations at once"
                        .to_string(),
                    span: next.span,
                });
            }
        }

        let value = match next.args.as_slice() {
            [] => None,
            [value] => Some(self.eval_value(value, scope)?),
            _ => {
                return Err(ResolveError::Internal {
                    message: "a positional `@next` has at most one value — the parser guarantees this shape"
                        .to_string(),
                    span: next.span,
                });
            }
        };

        let mut updates = Vec::with_capacity(next.bindings.len());
        for update in &next.bindings {
            updates.push((update.name.clone(), self.eval_value(&update.value, scope)?, update.span));
        }

        self.pending_next = Some(PendingNext { value, updates, span: next.span });
        Ok(())
    }

    /// Evaluates `expr` somewhere its `@emit`s and generated declarations
    /// have a place to go — as a statement, a `const`'s whole value or
    /// `@return`'s whole value. A `@fold` or a macro call there keeps them,
    /// returned alongside the value; anything else is an ordinary
    /// expression, where either one that emits is an error (see
    /// [`AliasResolver::reject_expression_emits`]).
    pub(super) fn eval_value_keeping_emits(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, Value>,
        allowed_emits: &[ResolvedType],
        emitted: &mut Vec<Value>,
        generated: &mut Vec<Statement>,
    ) -> Result<KeptValue, ResolveError> {
        match expr {
            Expr::Fold { fold, .. } => {
                let outcome = self.run_fold(fold, scope, allowed_emits)?;
                emitted.extend(outcome.emitted);
                generated.extend(outcome.generated);
                Ok(match outcome.exited {
                    Some(returned) => KeptValue::Exited(returned),
                    None => KeptValue::Value(outcome.value),
                })
            }

            Expr::Call { callee, arguments, span } => {
                if let Expr::Identifier { name, .. } = callee.as_ref() {
                    let bound = match scope.get(name) {
                        Some(Value::Macro(symbol)) => Some(*symbol),
                        _ => None,
                    };
                    let is_macro = bound.is_some()
                        || self.lookup_symbol(name).is_some_and(|id| self.get_symbol(id).kind == SymbolKind::Macro);

                    if is_macro {
                        let expansion = self.run_macro_call(name, bound, arguments, *span, scope)?;
                        emitted.extend(expansion.emitted);
                        generated.extend(expansion.generated);
                        return expansion
                            .returned
                            .map(KeptValue::Value)
                            .ok_or(ResolveError::ExpectedValueExpression { span: *span });
                    }
                }

                self.eval_value(expr, scope).map(KeptValue::Value)
            }

            _ => self.eval_value(expr, scope).map(KeptValue::Value),
        }
    }

    /// A `@fold` or macro call evaluated inside a larger expression, where
    /// anything it emits would have nowhere to go: only its value is kept,
    /// and emitting at all is an error. Silently dropping the values used
    /// to leave labels after the call pointing past where they should.
    pub(super) fn reject_expression_emits(
        &self,
        what: &str,
        expansion: &MacroExpansion,
        span: Span,
    ) -> Result<(), ResolveError> {
        if expansion.emitted.is_empty() {
            return Ok(());
        }

        Err(ResolveError::Fold {
            message: format!(
                "{what} emits values, so it can only be used as a statement, as a `const`'s whole \
                 value, or as `@return`'s whole value — inside a larger expression its emitted \
                 values would have nowhere to go"
            ),
            span,
        })
    }
}

fn check_accumulator_names(accumulators: &[FoldBinding], var: &str) -> Result<(), ResolveError> {
    for (index, accumulator) in accumulators.iter().enumerate() {
        if accumulator.name == var {
            return Err(ResolveError::Fold {
                message: format!("`{var}` is both the loop variable and an accumulator of this `@fold`"),
                span: accumulator.span,
            });
        }
        if accumulators[..index].iter().any(|earlier| earlier.name == accumulator.name) {
            return Err(ResolveError::Fold {
                message: format!("accumulator `{}` is declared twice in this `@fold`", accumulator.name),
                span: accumulator.span,
            });
        }
    }
    Ok(())
}

fn apply_next(accumulators: &mut [(String, Value)], next: PendingNext) -> Result<(), ResolveError> {
    if let Some(value) = next.value {
        if accumulators.len() != 1 {
            return Err(ResolveError::Fold {
                message: format!(
                    "a positional `@next` needs exactly one accumulator, but this `@fold` has {} — \
                     name the ones to update (`@next {} = ...`)",
                    accumulators.len(),
                    accumulators[0].0,
                ),
                span: next.span,
            });
        }
        accumulators[0].1 = value;
        return Ok(());
    }

    for (index, (name, value, span)) in next.updates.iter().enumerate() {
        if next.updates[..index].iter().any(|(earlier, ..)| earlier == name) {
            return Err(ResolveError::Fold {
                message: format!("`{name}` is updated twice in one `@next`"),
                span: *span,
            });
        }

        let Some(slot) = accumulators.iter_mut().find(|(accumulator, _)| accumulator == name) else {
            let names = accumulators.iter().map(|(name, _)| format!("`{name}`")).collect::<Vec<_>>();
            return Err(ResolveError::Fold {
                message: format!("`{name}` isn't an accumulator of this `@fold` (it has {})", names.join(", ")),
                span: *span,
            });
        };
        slot.1 = value.clone();
    }

    Ok(())
}
