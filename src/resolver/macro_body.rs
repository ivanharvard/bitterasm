//! Walks a single [`MacroDeclaration`]'s body, given already-bound
//! [`Value`] arguments for its declared params, collecting `@emit`'s values
//! in program order and stopping early on `@return` — mirroring how
//! `@return` is meant to splice a value in and exit, except `@emit` never
//! splices anything, it only appends to the output stream.
//!
//! A body statement can also be a real [`Invocation`] (`mov r1, 7`, or —
//! same AST shape — a bare macro-to-macro call like `helper v`, since
//! nothing distinguishes "a real instruction" from "one macro calling
//! another" syntactically). [`AliasResolver::expand_invocation`] is the
//! entry point that binds one of those to its [`MacroDeclaration`] by name,
//! evaluates its operands, and expands it; nested invocations inside a
//! macro body go through the same machinery, sharing one recursion-guard
//! stack so a macro invoking itself (directly, or via another macro that
//! calls back) is a compile error instead of a stack overflow.
//!
//! Each operand's evaluated `Value` is also checked against its parameter's
//! declared type before binding (`resolve_type_expr`'d the same way a
//! struct field's type would be, then compared against the value's own
//! implicit type — see `values::value_type`) — a `mov 7, r1`-shaped
//! argument-order mistake is a `TypeMismatch` error, not a silent wrong
//! bind.
//!
//! Still out of scope: non-default invocation syntax (a macro always binds
//! via the plain `name arg, arg, ...` form for now).
//!
//! A body statement that isn't `@emit`/`@return`/an invocation is ordinary
//! top-level-shaped BitterASM — `struct`, `type`, `macro`, `const`, a
//! label — since a macro body is exactly that: BitterASM the macro
//! generates at its call site, same as `@emit` generates a value there.
//! `Statement::Const` is the one kind with a private/`pub` distinction that
//! matters here too: a bare (non-`pub`) `const` evaluates immediately and
//! becomes a scope-local binding for the rest of *this* expansion, the same
//! role a `let` would play, while `pub const` (like every other supported
//! kind) is captured into [`MacroExpansion::generated`] to be spliced back
//! into the program wherever this expansion's call site was — nothing
//! actually performs that splice yet, so `generated` is inert until a
//! driver exists to consume it (see `crate::main`, which doesn't call
//! macro expansion at all today).
//!
//! A generated declaration's `` `expr` `` splices are evaluated now, against
//! this expansion's scope, and rewritten in place into the literal `Expr`
//! they produced ([`AliasResolver::splice_expr`]) — everything *outside* a
//! splice is left exactly as written, to be resolved later, in whatever
//! scope the generated declaration ends up in. Every declaration kind's own
//! *name* gets this same treatment (`self.resolve_spliced_name`, rewritten
//! back to a single-part literal `SplicedName`); only `Const`'s body also
//! carries a spliceable `value: Expr` (via `splice_const`) — a
//! struct/enum/type-alias/macro's body isn't spliced at all, only its name,
//! so those four kinds are otherwise captured verbatim.
//!
//! `Statement::Import` is a hard, permanent error rather than an
//! unimplemented one: imports are resolved by `crate::loader` before the
//! resolver ever runs, against the importing *file's* path — a macro body
//! has no file of its own for a relative import to resolve against, and by
//! the time macro expansion happens the whole program is already flattened.

use std::collections::{HashMap, HashSet};

use crate::ast::{
    literal_name, CallArgument, ConstDeclaration, ConstructItem, Expr, Invocation,
    MacroDeclaration, NamePart, Statement,
};
use crate::eval::Int;
use crate::token::Span;
use crate::types::{TypeArgument, TypeExpr};

use super::aliases::{AliasResolver, GenericBinding};
use super::structs::{describe_type, param_name};
use super::symbols::SymbolId;
use super::types::{ResolvedGenericArg, ResolvedType};
use super::values::Value;
use super::ResolveError;

/// Safety cap on `@for`'s iteration count — `Int` is arbitrary-precision,
/// so an unbounded or accidentally-huge range (`@for i in 0..N` with a
/// mis-set `N`) would otherwise hang rather than fail fast. Shared by
/// every `@for` unroller (a struct body's, in `structs.rs`; a top-level
/// one, in `toplevel.rs`), not just this one.
pub(super) const MAX_FOR_ITERATIONS: u64 = 1_000_000;

/// Best-effort name for an error message: a macro reaching real expansion
/// always has a literal name by now (`collect_symbols`/`register_generated`
/// already reject anything else), so this is really just an infallible
/// unwrap with a defensive fallback instead of a `panic!`.
fn macro_display_name(name: &[NamePart]) -> String {
    literal_name(name).unwrap_or_else(|| "<generated>".to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroExpansion {
    pub emitted: Vec<Value>,

    /// Declarations this expansion produced — a `pub const`, or any
    /// `struct`/`type`/`macro`/label found in the body — in program order,
    /// including any bubbled up from nested invocations. Not yet spliced
    /// back into a resolvable program anywhere; see the module doc.
    pub generated: Vec<Statement>,

    pub returned: Option<Value>,
}

impl<'a> AliasResolver<'a> {
    /// Resolves `invocation` to a [`MacroDeclaration`] by name, evaluates
    /// its operands against `scope`, and expands it. This is the entry
    /// point for a genuinely new top-level expansion — each call gets its
    /// own fresh recursion stack. Nested statement and expression calls
    /// share the resolver-owned stack while that expansion is running.
    pub fn expand_invocation(
        &mut self,
        invocation: &Invocation,
        scope: &HashMap<String, Value>,
    ) -> Result<MacroExpansion, ResolveError> {
        let mut stack = Vec::new();
        self.expand_invocation_inner(invocation, scope, &mut stack)
    }

    fn expand_invocation_inner(
        &mut self,
        invocation: &Invocation,
        scope: &HashMap<String, Value>,
        stack: &mut Vec<SymbolId>,
    ) -> Result<MacroExpansion, ResolveError> {
        let arguments = invocation
            .operands
            .iter()
            .map(|operand| self.eval_value(operand, scope))
            .collect::<Result<Vec<_>, _>>()?;

        let (symbol, declaration) =
            self.resolve_macro_overload(&invocation.name, &arguments, invocation.span)?;

        self.run_macro_body(symbol, &declaration, arguments, stack)
    }

    /// Selects the unique overload whose arity and fully resolved parameter
    /// types exactly match the already-evaluated arguments.
    pub(super) fn resolve_macro_overload(
        &mut self,
        name: &str,
        arguments: &[Value],
        span: Span,
    ) -> Result<(SymbolId, MacroDeclaration), ResolveError> {
        let ids = self.lookup_symbols(name);
        if ids.is_empty() {
            return Err(ResolveError::UnknownMacro { name: name.to_string(), span });
        }
        if ids.iter().any(|id| self.get_symbol(*id).kind != super::symbols::SymbolKind::Macro) {
            return Err(ResolveError::ExpectedMacro { name: name.to_string(), span });
        }

        let mut declarations = Vec::with_capacity(ids.len());
        for id in ids {
            declarations.push((id, self.find_macro_declaration(id)?.clone()));
        }

        // Keep the established, specific arity/type diagnostics for a
        // non-overloaded macro. `run_macro_body` performs those checks.
        if declarations.len() == 1 {
            return Ok(declarations.remove(0));
        }

        let actual_types = arguments
            .iter()
            .map(|value| self.value_type(value))
            .collect::<Result<Vec<_>, _>>()?;
        let mut matches = Vec::new();
        for (id, declaration) in declarations {
            let required = declaration.params.iter().filter(|param| param.default.is_none()).count();
            if actual_types.len() < required || actual_types.len() > declaration.params.len() {
                continue;
            }

            let accepted = if declaration.generic_params.is_empty() {
                let mut expected_types = Vec::with_capacity(declaration.params.len());
                for param in declaration.params.iter().take(actual_types.len()) {
                    expected_types.push(self.resolve_type_expr(&param.ty)?);
                }
                expected_types.iter().zip(&actual_types).all(|(expected, actual)| expected.accepts(actual))
            } else {
                // A candidate's own generic params don't need to fully
                // resolve for it to be ruled *in* or *out* here — only
                // `bind_macro_arguments` (run once the overload is chosen)
                // needs every one actually bound.
                let generic_names: HashSet<&str> =
                    declaration.generic_params.iter().map(|param| param_name(param)).collect();
                let mut generic_scope = HashMap::new();
                declaration
                    .params
                    .iter()
                    .take(actual_types.len())
                    .zip(&actual_types)
                    .all(|(param, actual)| {
                        self.unify_type_expr(&param.ty, actual, &generic_names, &mut generic_scope, param.span)
                            .is_ok()
                    })
            };

            if accepted {
                matches.push((id, declaration));
            }
        }

        let actual = actual_types
            .iter()
            .map(|ty| describe_type(ty, self.symbols))
            .collect();
        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(ResolveError::NoMatchingMacroOverload {
                name: name.to_string(), actual, span,
            }),
            _ => Err(ResolveError::AmbiguousMacroOverload {
                name: name.to_string(), actual, span,
            }),
        }
    }

    /// Runs `declaration` with a caller-visible stack for compatibility with
    /// direct resolver users. All nested calls use `macro_call_stack`, so
    /// expression-position calls cannot accidentally reset the guard.
    pub fn run_macro_body(
        &mut self,
        symbol: SymbolId,
        declaration: &MacroDeclaration,
        arguments: Vec<Value>,
        stack: &mut Vec<SymbolId>,
    ) -> Result<MacroExpansion, ResolveError> {
        let was_top_level = self.macro_call_stack.is_empty();
        if was_top_level {
            self.macro_call_stack.clone_from(stack);
        }
        let result = self.run_macro_body_inner(symbol, declaration, arguments);
        if was_top_level {
            stack.clone_from(&self.macro_call_stack);
            self.macro_call_stack.clear();
        }
        result
    }

    pub(super) fn run_macro_body_inner(
        &mut self,
        symbol: SymbolId,
        declaration: &MacroDeclaration,
        arguments: Vec<Value>,
    ) -> Result<MacroExpansion, ResolveError> {
        // Macro frames are deliberately rich (scopes, emitted/generated
        // values, hooks), so keep this below the host thread's stack limit.
        const MAX_MACRO_CALL_DEPTH: usize = 32;
        if self.macro_call_stack.len() >= MAX_MACRO_CALL_DEPTH {
            return Err(self.make_macro_depth_error(symbol, MAX_MACRO_CALL_DEPTH));
        }

        // Installed fresh from each iteration's `bind_macro_arguments` below
        // (a generic macro's own `T`/`const N`, inferred from this call's
        // arguments) and restored to whatever it was before this call once
        // the closure returns, the same push/pop-around-a-closure idiom
        // `macro_call_stack` already uses — correctly nests across a
        // recursive/nested macro call, generic or not.
        let previous_generic_scope = std::mem::take(&mut self.generic_scope);

        self.macro_call_stack.push(symbol);
        let result = (|| {
            let mut arguments = arguments;
            let mut expansion = MacroExpansion { emitted: Vec::new(), generated: Vec::new(), returned: None };
            let mut tail_calls = 0usize;

            loop {
                let (scope, generic_bindings) = self.bind_macro_arguments(declaration, arguments)?;
                self.generic_scope = generic_bindings;

                for hook in crate::facets::extract_exprs(&declaration.facets, "before") {
                    let hook_result = self.run_hook_template(&hook, &scope)?;
                    expansion.emitted.extend(hook_result.emitted);
                    expansion.generated.extend(hook_result.generated);
                }

                let body_result = self.walk_macro_body(&declaration.body, &scope)?;
                expansion.emitted.extend(body_result.emitted);
                expansion.generated.extend(body_result.generated);
                if let Some(next_arguments) = self.pending_tail_call.take() {
                    const MAX_TAIL_CALLS: usize = 4096;
                    tail_calls += 1;
                    if tail_calls > MAX_TAIL_CALLS {
                        return Err(ResolveError::MacroTailCallLimitExceeded {
                            name: macro_display_name(&declaration.name),
                            max_iterations: MAX_TAIL_CALLS,
                            span: declaration.span,
                        });
                    }
                    arguments = next_arguments;
                    continue;
                }
                expansion.returned = body_result.returned;

                let after_hooks = crate::facets::extract_exprs(&declaration.facets, "after");
                if !after_hooks.is_empty() {
                    let mut after_scope = scope.clone();
                    let fields = expansion
                        .returned
                        .clone()
                        .map(|value| vec![("returned".to_string(), value)])
                        .unwrap_or_default();
                    after_scope.insert(
                        "result".to_string(),
                        Value::Struct { symbol, args: Vec::new(), fields, nominal: None },
                    );

                    for hook in after_hooks {
                        let hook_result = self.run_hook_template(&hook, &after_scope)?;
                        expansion.emitted.extend(hook_result.emitted);
                        expansion.generated.extend(hook_result.generated);
                    }
                }
                return Ok(expansion);
            }
        })();

        self.macro_call_stack.pop();
        self.generic_scope = previous_generic_scope;
        result
    }

    /// Binds `declaration`'s params against already-evaluated `arguments`,
    /// returning both the resulting value scope and — for a generic macro
    /// — the bindings its own `T`/`const N` params picked up along the way,
    /// inferred from each argument's actual type (see `unify_type_expr`).
    /// Empty for a non-generic macro, which type-checks exactly as before.
    fn bind_macro_arguments(
        &mut self,
        declaration: &MacroDeclaration,
        arguments: Vec<Value>,
    ) -> Result<(HashMap<String, Value>, HashMap<String, GenericBinding>), ResolveError> {
        let required = declaration.params.iter().filter(|param| param.default.is_none()).count();
        if arguments.len() < required || arguments.len() > declaration.params.len() {
            return Err(ResolveError::InvalidArgumentCount {
                name: macro_display_name(&declaration.name), expected: declaration.params.len(),
                actual: arguments.len(), span: declaration.span,
            });
        }

        let generic_names: HashSet<&str> =
            declaration.generic_params.iter().map(|param| param_name(param)).collect();

        let mut scope = HashMap::new();
        let mut generic_scope: HashMap<String, GenericBinding> = HashMap::new();
        let mut arguments = arguments.into_iter();

        for param in &declaration.params {
            let value = match arguments.next() {
                Some(value) => value,
                None => self.eval_value(
                    param.default.as_ref().expect("arity check guarantees a default"), &scope,
                )?,
            };
            let actual = self.value_type(&value)?;

            if generic_names.is_empty() {
                let expected = self.resolve_type_expr(&param.ty)?;
                if !expected.accepts(&actual) {
                    return Err(ResolveError::TypeMismatch {
                        name: param.name.clone(), expected: describe_type(&expected, self.symbols),
                        actual: describe_type(&actual, self.symbols), span: param.span,
                    });
                }
            } else {
                self.unify_type_expr(&param.ty, &actual, &generic_names, &mut generic_scope, param.span)?;
            }

            scope.insert(param.name.clone(), value);
        }

        for param in &declaration.generic_params {
            let name = param_name(param);
            if !generic_scope.contains_key(name) {
                return Err(ResolveError::UnresolvedMacroGenericParam {
                    name: name.to_string(),
                    macro_name: macro_display_name(&declaration.name),
                    span: declaration.span,
                });
            }
        }

        // A bound `const` generic param is also an ordinary value inside
        // the body — mirrors `eval_call_value`'s `default_scope` doing the
        // same for a struct's own bound const generic args.
        for (name, binding) in &generic_scope {
            if let GenericBinding::Const(Some(value)) = binding {
                scope.insert(name.clone(), Value::Int(value.clone()));
            }
        }

        Ok((scope, generic_scope))
    }

    /// Binds `expected`'s free occurrences of one of `declaration`'s own
    /// generic params (`generic_names`) against the concrete `actual` type
    /// a call-site argument actually has — inference, the same way Rust
    /// infers a generic function's type params from its arguments, since a
    /// macro call has no `<...>` slot to supply them explicitly. A second,
    /// conflicting binding for an already-bound name, or any structural
    /// mismatch (wrong struct/enum, wrong arity, a non-generic leaf that
    /// doesn't match), is a `TypeMismatch`.
    fn unify_type_expr(
        &mut self,
        expected: &TypeExpr,
        actual: &ResolvedType,
        generic_names: &HashSet<&str>,
        scope: &mut HashMap<String, GenericBinding>,
        span: Span,
    ) -> Result<(), ResolveError> {
        if let TypeExpr::Named { path, .. } = expected {
            if path.len() == 1 && generic_names.contains(path[0].as_str()) {
                let name = &path[0];
                return match scope.get(name) {
                    Some(GenericBinding::Type(bound)) if bound == actual => Ok(()),
                    Some(bound_wrong_kind @ GenericBinding::Const(_)) => Err(ResolveError::TypeMismatch {
                        name: name.clone(),
                        expected: format!("{bound_wrong_kind:?}"),
                        actual: describe_type(actual, self.symbols),
                        span,
                    }),
                    Some(GenericBinding::Type(bound)) => Err(ResolveError::TypeMismatch {
                        name: name.clone(),
                        expected: describe_type(bound, self.symbols),
                        actual: describe_type(actual, self.symbols),
                        span,
                    }),
                    None => {
                        scope.insert(name.clone(), GenericBinding::Type(actual.clone()));
                        Ok(())
                    }
                };
            }
        }

        if let TypeExpr::Apply { base, args, .. } = expected {
            let mismatch = || ResolveError::TypeMismatch {
                name: expected.name().unwrap_or("<generic argument>").to_string(),
                expected: format!("{base:?}<...>"),
                actual: describe_type(actual, self.symbols),
                span,
            };

            let base_resolved = self.resolve_type_expr(base)?;
            let actual_args: &[ResolvedGenericArg] = match (&base_resolved, actual) {
                (ResolvedType::Struct { symbol, .. }, ResolvedType::Struct { symbol: actual_symbol, args })
                    if symbol == actual_symbol => args,
                (ResolvedType::Enum { symbol, .. }, ResolvedType::Enum { symbol: actual_symbol, args })
                    if symbol == actual_symbol => args,
                _ => return Err(mismatch()),
            };

            if args.len() != actual_args.len() {
                return Err(mismatch());
            }

            for (arg, actual_arg) in args.iter().zip(actual_args) {
                match (arg, actual_arg) {
                    (TypeArgument::Type(inner), ResolvedGenericArg::Type(actual_inner)) => {
                        self.unify_type_expr(inner, actual_inner, generic_names, scope, span)?;
                    }

                    (TypeArgument::Const(expr), ResolvedGenericArg::Const(value)) => {
                        if let Expr::Identifier { name, .. } = expr {
                            if generic_names.contains(name.as_str()) {
                                match scope.get(name) {
                                    Some(GenericBinding::Const(Some(bound))) if bound == value => {}
                                    Some(_) => return Err(mismatch()),
                                    None => {
                                        scope.insert(name.clone(), GenericBinding::Const(Some(value.clone())));
                                    }
                                }
                                continue;
                            }
                        }

                        if self.eval_const_expr(expr)? != *value {
                            return Err(mismatch());
                        }
                    }

                    (TypeArgument::Wildcard(_), _) => {}

                    _ => return Err(mismatch()),
                }
            }

            return Ok(());
        }

        // An ordinary, non-generic leaf type (`int`, a concrete struct with
        // no generics referenced, etc.) — checked exactly like a
        // non-generic macro's parameter always has been.
        let resolved = self.resolve_type_expr(expected)?;
        if resolved.accepts(actual) {
            Ok(())
        } else {
            Err(ResolveError::TypeMismatch {
                name: expected.name().unwrap_or("<type>").to_string(),
                expected: describe_type(&resolved, self.symbols),
                actual: describe_type(actual, self.symbols),
                span,
            })
        }
    }

    fn run_hook_template(
        &mut self,
        template: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<MacroExpansion, ResolveError> {
        let Expr::Call { callee, arguments, span } = template else {
            return Err(ResolveError::ExpectedValueExpression { span: template.span() });
        };
        let Expr::Identifier { name, .. } = callee.as_ref() else {
            return Err(ResolveError::ExpectedValueExpression { span: *span });
        };
        if arguments.iter().any(|argument| argument.name.is_some()) {
            return Err(ResolveError::ExpectedValueExpression { span: *span });
        }
        let values = arguments
            .iter()
            .map(|argument| self.eval_value(&argument.value, scope))
            .collect::<Result<Vec<_>, _>>()?;
        let (hook_symbol, hook) = self.resolve_macro_overload(name, &values, *span)?;
        self.run_macro_body_inner(hook_symbol, &hook, values)
    }

    fn walk_macro_body(
        &mut self,
        body: &[Statement],
        initial_scope: &HashMap<String, Value>,
    ) -> Result<MacroExpansion, ResolveError> {
        // Owned and mutable, unlike `initial_scope` — a bare (non-`pub`)
        // `const` extends this for the rest of the body, the same way a
        // `let` would; nothing outside this expansion ever sees it.
        let mut scope = initial_scope.clone();

        let mut emitted = Vec::new();
        let mut generated = Vec::new();

        for statement in body {
            match statement {
                Statement::Meta(meta) => match meta.name.as_str() {
                    "emit" => match meta.args.as_slice() {
                        [expr] => {
                            emitted.push(self.eval_value(expr, &scope)?);
                            // Advances the shared, whole-program-persistent
                            // counter `@here` reads — see
                            // `AliasResolver::values_emitted`. A nested
                            // invocation's own `@emit`s bump this same
                            // field through the shared `&mut self`, so
                            // there's nothing extra to do where nested
                            // `emitted`/`generated` get folded in below.
                            self.values_emitted += Int::from(1);
                        }

                        other => {
                            return Err(ResolveError::InvalidArgumentCount {
                                name: "@emit".to_string(),
                                expected: 1,
                                actual: other.len(),
                                span: meta.span,
                            })
                        }
                    },

                    "return" => {
                        if let [expr] = meta.args.as_slice() {
                            if let Some(arguments) = self.eval_direct_self_tail_call(expr, &scope)? {
                                self.pending_tail_call = Some(arguments);
                                return Ok(MacroExpansion { emitted, generated, returned: None });
                            }
                        }
                        let value = match meta.args.as_slice() {
                            [] => None,
                            [expr] => Some(self.eval_value(expr, &scope)?),

                            other => {
                                return Err(ResolveError::InvalidArgumentCount {
                                    name: "@return".to_string(),
                                    expected: 1,
                                    actual: other.len(),
                                    span: meta.span,
                                })
                            }
                        };

                        return Ok(MacroExpansion { emitted, generated, returned: value });
                    }

                    "assert" => super::metas::assert::check(self, &meta.args, &scope, meta.span)?,

                    "if" => {
                        let [condition] = meta.args.as_slice() else {
                            return Err(ResolveError::Internal {
                                message: "`@if`'s args should always be a single condition — \
                                          the parser guarantees this shape"
                                    .to_string(),
                                span: meta.span,
                            });
                        };

                        let chosen = if self.eval_truthy(condition, &scope)? {
                            meta.body.as_ref()
                        } else {
                            meta.else_body.as_ref()
                        };

                        if let Some(chosen_body) = chosen {
                            let nested = self.walk_macro_body(chosen_body, &scope)?;
                            emitted.extend(nested.emitted);
                            generated.extend(nested.generated);

                            if nested.returned.is_some() || self.pending_tail_call.is_some() {
                                return Ok(MacroExpansion {
                                    emitted,
                                    generated,
                                    returned: nested.returned,
                                });
                            }
                        }
                    }

                    "match" => {
                        let [scrutinee] = meta.args.as_slice() else {
                            return Err(ResolveError::Internal {
                                message: "`@match` should always have one scrutinee — the parser \
                                          guarantees this shape"
                                    .to_string(),
                                span: meta.span,
                            });
                        };

                        let value = self.eval_value(scrutinee, &scope)?;
                        let mut chosen = None;
                        for arm in &meta.match_arms {
                            let bindings = match &arm.pattern {
                                Some(pattern) => self.match_pattern(pattern, &value, &scope)?,
                                None => Some(HashMap::new()),
                            };
                            if let Some(bindings) = bindings {
                                chosen = Some((&arm.body, bindings));
                                break;
                            }
                        }

                        if let Some((chosen_body, bindings)) = chosen {
                            let mut arm_scope = scope.clone();
                            arm_scope.extend(bindings);
                            let nested = self.walk_macro_body(chosen_body, &arm_scope)?;
                            emitted.extend(nested.emitted);
                            generated.extend(nested.generated);
                            if nested.returned.is_some() || self.pending_tail_call.is_some() {
                                return Ok(MacroExpansion {
                                    emitted,
                                    generated,
                                    returned: nested.returned,
                                });
                            }
                        }
                    }

                    "for" => {
                        let [var, source_expr] = meta.args.as_slice() else {
                            return Err(ResolveError::Internal {
                                message: "`@for`'s args should always be [var, source] — \
                                          the parser guarantees this shape"
                                    .to_string(),
                                span: meta.span,
                            });
                        };

                        let Expr::Identifier { name: var_name, .. } = var else {
                            return Err(ResolveError::Internal {
                                message: "`@for`'s loop variable should always be an \
                                          identifier — the parser guarantees this shape"
                                    .to_string(),
                                span: meta.span,
                            });
                        };

                        let for_body = meta.body.as_ref().ok_or_else(|| ResolveError::Internal {
                            message: "`@for` should always carry a body — the parser \
                                      guarantees this shape"
                                .to_string(),
                            span: meta.span,
                        })?;

                        let bindings = self.eval_for_source(source_expr, &scope)?;

                        for (_, value) in bindings {
                            let mut iter_scope = scope.clone();
                            iter_scope.insert(var_name.clone(), value);

                            let nested = self.walk_macro_body(for_body, &iter_scope)?;
                            emitted.extend(nested.emitted);
                            generated.extend(nested.generated);

                            if nested.returned.is_some() || self.pending_tail_call.is_some() {
                                return Ok(MacroExpansion {
                                    emitted,
                                    generated,
                                    returned: nested.returned,
                                });
                            }
                        }
                    }

                    other => {
                        return Err(ResolveError::UnsupportedMacroStatement {
                            kind: format!("@{other}"),
                            span: meta.span,
                        })
                    }
                },

                Statement::Invocation(invocation) => {
                    // A bare invocation has no binding point for a return
                    // value — same as a real instruction line not producing
                    // a usable result elsewhere — so only its emitted (and
                    // generated) output is spliced into ours, in order;
                    // `returned` is discarded.
                    let arguments = invocation
                        .operands
                        .iter()
                        .map(|operand| self.eval_value(operand, &scope))
                        .collect::<Result<Vec<_>, _>>()?;
                    let (symbol, declaration) = self.resolve_macro_overload(
                        &invocation.name,
                        &arguments,
                        invocation.span,
                    )?;
                    let nested = self.run_macro_body_inner(symbol, &declaration, arguments)?;
                    emitted.extend(nested.emitted);
                    generated.extend(nested.generated);
                }

                Statement::Const(decl) if decl.is_pub => {
                    let spliced = Statement::Const(self.splice_const(decl, &scope)?);
                    self.register_generated(&spliced)?;
                    generated.push(spliced);
                }

                Statement::Const(decl) => {
                    let value = self.eval_value(&decl.value, &scope)?;

                    let value = match &decl.ty {
                        Some(ty) => {
                            let target = self.resolve_type_expr(ty)?;
                            self.convert_to(value, &target, decl.span)?
                        }
                        None => value,
                    };

                    let name = self.resolve_spliced_name(&decl.name, &scope)?;
                    scope.insert(name, value);
                }

                // Only the declaration's own name is spliced (see the
                // module doc) — everything else about it is captured
                // verbatim, to be resolved later wherever this expansion's
                // call site spliced it back into the program.
                Statement::Struct(decl) => {
                    let mut decl = decl.clone();
                    decl.name = self.splice_decl_name(&decl.name, &scope)?;
                    let spliced = Statement::Struct(decl);
                    self.register_generated(&spliced)?;
                    generated.push(spliced);
                }

                Statement::Enum(decl) => {
                    let mut decl = decl.clone();
                    decl.name = self.splice_decl_name(&decl.name, &scope)?;
                    let spliced = Statement::Enum(decl);
                    self.register_generated(&spliced)?;
                    generated.push(spliced);
                }

                Statement::TypeAlias(decl) => {
                    let mut decl = decl.clone();
                    decl.name = self.splice_decl_name(&decl.name, &scope)?;
                    let spliced = Statement::TypeAlias(decl);
                    self.register_generated(&spliced)?;
                    generated.push(spliced);
                }

                Statement::Macro(decl) => {
                    let mut decl = decl.clone();
                    decl.name = self.splice_decl_name(&decl.name, &scope)?;
                    let spliced = Statement::Macro(decl);
                    self.register_generated(&spliced)?;
                    generated.push(spliced);
                }

                Statement::Label(_) => {
                    // Label names aren't spliceable name positions at all
                    // (a plain `String`, not a `SplicedName`) — captured
                    // verbatim.
                    self.register_generated(statement)?;
                    generated.push(statement.clone());
                }

                Statement::Import(import) => {
                    return Err(ResolveError::UnsupportedMacroStatement {
                        kind: "import".to_string(),
                        span: import.span,
                    });
                }

                // Unreachable in practice — the parser only ever
                // recognizes `syntax name(...) = { ... }` at true top
                // level (`Parser::block_depth`), never inside a macro
                // body — but matched for exhaustiveness with the same
                // treatment `import` gets.
                Statement::SyntaxOverride(override_statement) => {
                    return Err(ResolveError::UnsupportedMacroStatement {
                        kind: "syntax override".to_string(),
                        span: override_statement.span,
                    });
                }
            }
        }

        Ok(MacroExpansion { emitted, generated, returned: None })
    }

    /// Recognizes only an exact `@return current_macro(...)`. Calls wrapped
    /// in arithmetic or conversion remain ordinary, depth-counted calls.
    /// Macros with `after` hooks are excluded because eliminating their
    /// frames would change the hooks' unwind-time behavior.
    fn eval_direct_self_tail_call(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<Option<Vec<Value>>, ResolveError> {
        let Some(current) = self.macro_call_stack.last().copied() else {
            return Ok(None);
        };
        let Expr::Call { callee, arguments, span } = expr else {
            return Ok(None);
        };
        let Expr::Identifier { name, .. } = callee.as_ref() else {
            return Ok(None);
        };
        if name != &self.get_symbol(current).name
            || arguments.iter().any(|argument| argument.name.is_some())
        {
            return Ok(None);
        }
        let current_declaration = self.find_macro_declaration(current)?.clone();
        if !crate::facets::extract_exprs(&current_declaration.facets, "after").is_empty() {
            return Ok(None);
        }
        let values = arguments
            .iter()
            .map(|argument| self.eval_value(&argument.value, scope))
            .collect::<Result<Vec<_>, _>>()?;
        let (selected, _) = self.resolve_macro_overload(name, &values, *span)?;
        Ok((selected == current).then_some(values))
    }

    fn match_pattern(
        &mut self,
        pattern: &Expr,
        value: &Value,
        scope: &HashMap<String, Value>,
    ) -> Result<Option<HashMap<String, Value>>, ResolveError> {
        if let Value::Enum { variant, payload, .. } = value {
            match pattern {
                Expr::Identifier { name, .. } if name == variant && payload.is_none() => {
                    return Ok(Some(HashMap::new()));
                }
                Expr::Call { callee, arguments, .. } => {
                    let Expr::Identifier { name, .. } = callee.as_ref() else {
                        return Ok(None);
                    };
                    if name != variant || arguments.len() != 1 {
                        return Ok(None);
                    }
                    let Some(payload) = payload.as_deref() else {
                        return Ok(None);
                    };
                    if let Expr::Identifier { name: binding, .. } = &arguments[0].value {
                        let mut bindings = HashMap::new();
                        if binding != "_" {
                            bindings.insert(binding.clone(), payload.clone());
                        }
                        return Ok(Some(bindings));
                    }
                    return Ok((self.eval_value(&arguments[0].value, scope)? == *payload)
                        .then(HashMap::new));
                }
                Expr::EnumVariant { .. } => {
                    return Ok((self.eval_value(pattern, scope)? == *value).then(HashMap::new));
                }
                _ => return Ok(None),
            }
        }

        Ok((self.eval_value(pattern, scope)? == *value).then(HashMap::new))
    }

    /// Resolves a nested struct/enum/type-alias/macro declaration's own
    /// name against this expansion's scope, the `SplicedName` sibling of
    /// `splice_const`'s name handling — the rest of the declaration isn't
    /// spliced (see the module doc), only this.
    fn splice_decl_name(
        &mut self,
        name: &[NamePart],
        scope: &HashMap<String, Value>,
    ) -> Result<crate::ast::SplicedName, ResolveError> {
        let literal = self.resolve_spliced_name(name, scope)?;
        Ok(vec![NamePart::Literal(literal)])
    }

    fn splice_const(
        &mut self,
        decl: &ConstDeclaration,
        scope: &HashMap<String, Value>,
    ) -> Result<ConstDeclaration, ResolveError> {
        let name = self.resolve_spliced_name(&decl.name, scope)?;

        Ok(ConstDeclaration {
            name: vec![NamePart::Literal(name)],
            is_pub: decl.is_pub,
            ty: decl.ty.clone(),
            value: self.splice_expr(&decl.value, scope)?,
            span: decl.span,
        })
    }

    /// Rewrites `expr` for a generated declaration: every `` `inner` ``
    /// splice is evaluated now against `scope` and replaced with the
    /// literal `Expr` that value reifies to; everything else — including a
    /// bare identifier that isn't a splice — is left exactly as written,
    /// to be resolved later wherever the generated declaration ends up.
    fn splice_expr(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<Expr, ResolveError> {
        match expr {
            Expr::Splice { inner, span } => {
                let value = self.eval_value(inner, scope)?;
                reify_value(&value, *span)
            }

            Expr::Identifier { .. } | Expr::Integer { .. } | Expr::String { .. } | Expr::Here { .. } => {
                Ok(expr.clone())
            }

            Expr::Member { object, member, span } => Ok(Expr::Member {
                object: Box::new(self.splice_expr(object, scope)?),
                member: member.clone(),
                span: *span,
            }),

            Expr::Call { callee, arguments, span } => Ok(Expr::Call {
                callee: Box::new(self.splice_expr(callee, scope)?),
                arguments: arguments
                    .iter()
                    .map(|argument| {
                        Ok(CallArgument {
                            name: argument.name.clone(),
                            value: self.splice_expr(&argument.value, scope)?,
                            span: argument.span,
                        })
                    })
                    .collect::<Result<_, ResolveError>>()?,
                span: *span,
            }),

            Expr::EnumVariant { enum_name, generic_args, variant, payload, span } => {
                Ok(Expr::EnumVariant {
                    enum_name: enum_name.clone(),
                    generic_args: generic_args.clone(),
                    variant: variant.clone(),
                    payload: payload
                        .as_ref()
                        .map(|value| self.splice_expr(value, scope).map(Box::new))
                        .transpose()?,
                    span: *span,
                })
            }

            Expr::Unary { op, operand, span } => Ok(Expr::Unary {
                op: *op,
                operand: Box::new(self.splice_expr(operand, scope)?),
                span: *span,
            }),

            Expr::Binary { left, op, right, span } => Ok(Expr::Binary {
                left: Box::new(self.splice_expr(left, scope)?),
                op: *op,
                right: Box::new(self.splice_expr(right, scope)?),
                span: *span,
            }),

            // Same treatment as `Call`'s callee/arguments just above:
            // spliced now against `scope`, everything else left as
            // written. A generic `Type` argument isn't dug into for
            // nested splices — mirrors `resolver::consts::referenced_identifiers`'s
            // identical scoping choice, for the same reason: a type
            // position isn't where an "already evaluated, splice this in"
            // value is expected to live.
            Expr::Construct { callee, generic_args, fields, span } => Ok(Expr::Construct {
                callee: Box::new(self.splice_expr(callee, scope)?),
                generic_args: generic_args
                    .iter()
                    .map(|arg| match arg {
                        TypeArgument::Type(ty) => Ok(TypeArgument::Type(ty.clone())),
                        TypeArgument::Const(expr) => Ok(TypeArgument::Const(self.splice_expr(expr, scope)?)),
                        TypeArgument::Wildcard(span) => Ok(TypeArgument::Wildcard(*span)),
                    })
                    .collect::<Result<_, ResolveError>>()?,
                fields: self.splice_construct_items(fields, scope)?,
                span: *span,
            }),

            // Same scoping choice as `Construct` just above — the type
            // isn't dug into for splices.
            Expr::As { value, ty, span } => Ok(Expr::As {
                value: Box::new(self.splice_expr(value, scope)?),
                ty: ty.clone(),
                span: *span,
            }),

            Expr::Range { start, end, span } => Ok(Expr::Range {
                start: Box::new(self.splice_expr(start, scope)?),
                end: Box::new(self.splice_expr(end, scope)?),
                span: *span,
            }),
        }
    }

    // `splice_expr`'s counterpart for a construction's own field list —
    // each field's name is fully resolved now (mirrors `splice_const`
    // resolving a generated `pub const`'s name), its value spliced the same
    // way `splice_expr` handles any other value position.
    fn splice_construct_items(
        &mut self,
        items: &[ConstructItem],
        scope: &HashMap<String, Value>,
    ) -> Result<Vec<ConstructItem>, ResolveError> {
        items
            .iter()
            .map(|item| match item {
                ConstructItem::Field { name, value, span } => Ok(ConstructItem::Field {
                    name: vec![NamePart::Literal(self.resolve_spliced_name(name, scope)?)],
                    value: self.splice_expr(value, scope)?,
                    span: *span,
                }),

                ConstructItem::For { var, source, body, span } => Ok(ConstructItem::For {
                    var: var.clone(),
                    source: self.splice_expr(source, scope)?,
                    body: self.splice_construct_items(body, scope)?,
                    span: *span,
                }),

                ConstructItem::If { condition, body, else_body, span } => Ok(ConstructItem::If {
                    condition: self.splice_expr(condition, scope)?,
                    body: self.splice_construct_items(body, scope)?,
                    else_body: else_body
                        .as_ref()
                        .map(|else_body| self.splice_construct_items(else_body, scope))
                        .transpose()?,
                    span: *span,
                }),
            })
            .collect()
    }

    fn make_macro_depth_error(&self, next: SymbolId, max_depth: usize) -> ResolveError {
        let mut call_chain: Vec<String> = self
            .macro_call_stack
            .iter()
            .map(|id| self.get_symbol(*id).name.clone())
            .collect();
        call_chain.push(self.get_symbol(next).name.clone());
        ResolveError::MacroCallDepthExceeded {
            call_chain,
            max_depth,
            span: self.get_symbol(next).span,
        }
    }
}

// A splice's evaluated `Value` reified back into source-shaped `Expr`, for
// a generated declaration's rewritten `value` — see
// `AliasResolver::splice_expr`.
fn reify_value(value: &Value, span: Span) -> Result<Expr, ResolveError> {
    match value {
        Value::Int(int) => Ok(Expr::Integer { raw: int.to_string(), span }),
        Value::Struct { .. } | Value::Enum { .. } => {
            Err(ResolveError::UnsupportedSpliceValue { span })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use std::path::Path;

    use crate::ast::{literal_name, Expr, Invocation, MacroDeclaration, Program, Statement};
    use crate::eval::Int;
    use crate::lexer;
    use crate::parser;
    use crate::resolver::{collect_symbols, AliasResolver, LabelMode, ResolveError, ResolvedGenericArg};

    use super::{MacroExpansion, Value};

    fn parse_fixture(name: &str) -> Program {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/emit")
            .join(name);

        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()));

        let tokens = lexer::lex(&source).expect("fixture should lex");
        parser::parse(tokens).expect("fixture should parse")
    }

    fn find_macro<'a>(program: &'a Program, name: &str) -> &'a MacroDeclaration {
        program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Macro(decl) if literal_name(&decl.name).as_deref() == Some(name) => Some(decl),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected a macro named `{name}`"))
    }

    fn find_invocation<'a>(program: &'a Program, name: &str) -> &'a Invocation {
        program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Invocation(invocation) if invocation.name == name => Some(invocation),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected a top-level invocation of `{name}`"))
    }

    #[test]
    fn emit_and_return_combine_and_return_stops_the_body() {
        let program = parse_fixture("combo.basm");

        let declaration = find_macro(&program, "combo");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("combo").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(5))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(5)), Value::Int(Int::from(10))],
                generated: vec![],
                returned: Some(Value::Int(Int::from(15))),
            }
        );
    }

    #[test]
    fn match_selects_the_first_equal_arm_or_wildcard() {
        let program = parse_fixture("match.basm");
        let declaration = find_macro(&program, "classify");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("classify").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        for (input, expected) in [(0, 10), (2, 20), (9, 30)] {
            let result = resolver
                .run_macro_body(
                    symbol,
                    declaration,
                    vec![Value::Int(Int::from(input))],
                    &mut Vec::new(),
                )
                .unwrap();
            assert_eq!(result.emitted, vec![Value::Int(Int::from(expected))]);
        }
    }

    #[test]
    fn option_variants_construct_and_destructure() {
        let program = parse_fixture("option.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let some_decl = find_macro(&program, "make_some");
        let some_symbol = symbols.lookup("make_some").unwrap();
        let some = resolver
            .run_macro_body(
                some_symbol,
                some_decl,
                vec![Value::Int(Int::from(42))],
                &mut Vec::new(),
            )
            .unwrap()
            .returned
            .unwrap();

        let unwrap_decl = find_macro(&program, "unwrap_or");
        let unwrap_symbol = symbols.lookup("unwrap_or").unwrap();
        let unwrapped = resolver
            .run_macro_body(
                unwrap_symbol,
                unwrap_decl,
                vec![some, Value::Int(Int::from(7))],
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(unwrapped.returned, Some(Value::Int(Int::from(42))));

        let none_decl = find_macro(&program, "make_none");
        let none_symbol = symbols.lookup("make_none").unwrap();
        let none = resolver
            .run_macro_body(none_symbol, none_decl, vec![], &mut Vec::new())
            .unwrap()
            .returned
            .unwrap();
        let fallback = resolver
            .run_macro_body(
                unwrap_symbol,
                unwrap_decl,
                vec![none, Value::Int(Int::from(7))],
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(fallback.returned, Some(Value::Int(Int::from(7))));
    }

    #[test]
    fn to_and_from_facets_convert_struct_fields_and_sources() {
        let program = parse_fixture("conversion_facets.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let cartesian_symbol = symbols.lookup("Cartesian").unwrap();
        let cartesian = Value::Struct {
            symbol: cartesian_symbol,
            args: vec![],
            fields: vec![
                ("x".to_string(), Value::Int(Int::from(2))),
                ("y".to_string(), Value::Int(Int::from(3))),
            ],
            nominal: None,
        };
        let convert = find_macro(&program, "convert_cartesian");
        let result = resolver
            .run_macro_body(
                symbols.lookup("convert_cartesian").unwrap(),
                convert,
                vec![cartesian],
                &mut Vec::new(),
            )
            .unwrap()
            .returned
            .unwrap();
        assert!(matches!(result, Value::Struct { ref fields, .. }
            if fields[0].1 == Value::Int(Int::from(5))));

        let convert = find_macro(&program, "convert_int");
        let result = resolver
            .run_macro_body(
                symbols.lookup("convert_int").unwrap(),
                convert,
                vec![Value::Int(Int::from(7))],
                &mut Vec::new(),
            )
            .unwrap()
            .returned
            .unwrap();
        assert!(matches!(result, Value::Struct { ref fields, .. }
            if fields[0].1 == Value::Int(Int::from(7))));

        let convert = find_macro(&program, "convert_int_to_sized");
        let result = resolver
            .run_macro_body(
                symbols.lookup("convert_int_to_sized").unwrap(),
                convert,
                vec![Value::Int(Int::from(11))],
                &mut Vec::new(),
            )
            .unwrap()
            .returned
            .unwrap();
        assert!(matches!(result, Value::Struct { ref fields, .. }
            if fields[0].1 == Value::Int(Int::from(11))
                && fields[1].1 == Value::Int(Int::from(7))));
    }

    #[test]
    fn before_and_after_hooks_see_parameters_and_returned_value() {
        let program = parse_fixture("macro_hooks.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let declaration = find_macro(&program, "increment");
        let symbol = symbols.lookup("increment").unwrap();

        let result = resolver
            .run_macro_body(
                symbol,
                declaration,
                vec![Value::Int(Int::from(4))],
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(result.returned, Some(Value::Int(Int::from(5))));

        let error = resolver
            .run_macro_body(
                symbol,
                declaration,
                vec![Value::Int(Int::from(0))],
                &mut Vec::new(),
            )
            .unwrap_err();
        assert!(matches!(error, ResolveError::AssertionFailed { .. }));
    }

    #[test]
    fn struct_via_generic_alias_carries_resolved_generic_args() {
        let program = parse_fixture("generic_alias.basm");

        let declaration = find_macro(&program, "make_byte");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_byte").unwrap();
        let bits_id = symbols.lookup("bits").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Struct {
                    symbol: bits_id,
                    args: vec![ResolvedGenericArg::Const(Int::from(8))],
                    fields: vec![("value".to_string(), Value::Int(Int::from(3)))],
                    nominal: None,
                }],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn rejects_unsupported_statement_in_macro_body() {
        let program = parse_fixture("unsupported_statement.basm");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("bad").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();

        assert!(matches!(
            resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack),
            Err(ResolveError::UnsupportedMacroStatement { .. })
        ));
    }

    #[test]
    fn expands_a_real_top_level_invocation() {
        let program = parse_fixture("top_level_invocation.basm");

        let invocation = find_invocation(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let result = resolver
            .expand_invocation(invocation, &HashMap::new())
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(10))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn nested_invocation_splices_callee_emits_in_order() {
        let program = parse_fixture("nested_invocation.basm");

        let declaration = find_macro(&program, "outer");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("outer").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(5))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![
                    Value::Int(Int::from(5)),
                    Value::Int(Int::from(6)),
                    Value::Int(Int::from(50)),
                ],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn rejects_operand_type_mismatch() {
        let program = parse_fixture("read_id.basm");

        let declaration = find_macro(&program, "read_id");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("read_id").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        // `dst` is declared `Reg`; passing a bare Int should be rejected
        // before the body even runs, the same way `mov 7, r1` (args
        // swapped) should be.
        let mut stack = Vec::new();

        match resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(5))], &mut stack) {
            Err(ResolveError::TypeMismatch { name, expected, actual, .. }) => {
                assert_eq!(name, "dst");
                assert_eq!(expected, "Reg");
                assert_eq!(actual, "int");
            }
            other => panic!("expected a type mismatch error, got {other:?}"),
        }
    }

    #[test]
    fn bounds_runaway_direct_recursion() {
        let program = parse_fixture("self_recursive.basm");

        let declaration = find_macro(&program, "loopy");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("loopy").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack);

        match result {
            Err(ResolveError::MacroCallDepthExceeded { call_chain, max_depth, .. }) => {
                assert_eq!(max_depth, 32);
                assert_eq!(call_chain.len(), 33);
                assert!(call_chain.iter().all(|name| name == "loopy"));
            }
            other => panic!("expected a cyclic macro expansion error, got {other:?}"),
        }
    }

    #[test]
    fn bounds_runaway_mutual_recursion() {
        let program = parse_fixture("mutual_recursion.basm");

        let declaration = find_macro(&program, "ping");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("ping").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack);

        match result {
            Err(ResolveError::MacroCallDepthExceeded { call_chain, max_depth, .. }) => {
                assert_eq!(max_depth, 32);
                assert_eq!(call_chain.len(), 33);
                assert_eq!(call_chain.first().map(String::as_str), Some("ping"));
                assert_eq!(call_chain.last().map(String::as_str), Some("ping"));
                assert!(call_chain.windows(2).all(|pair| pair[0] != pair[1]));
            }
            other => panic!("expected a cyclic macro expansion error, got {other:?}"),
        }
    }

    #[test]
    fn terminating_expression_recursion_returns_a_value() {
        let source = "macro factorial(n: int) -> int {\n\
                          @if n <= 1 {\n\
                              @return 1\n\
                          }\n\
                          @return n * factorial(n - 1)\n\
                      }\n\
                      macro calculate() { @emit factorial(6)\n }\n\
                      calculate\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocation = find_invocation(&program, "calculate");
        assert_eq!(
            resolver.expand_invocation(invocation, &HashMap::new()).unwrap().emitted,
            vec![Value::Int(Int::from(720))]
        );
    }

    #[test]
    fn direct_self_tail_calls_reuse_the_current_frame() {
        let source = "macro sum_to(n: int, acc: int = 0) -> int {\n\
                          @if n == 0 { @return acc\n }\n\
                          @return sum_to(n - 1, acc + n)\n\
                      }\n\
                      macro calculate() { @emit sum_to(100)\n }\n\
                      calculate\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocation = find_invocation(&program, "calculate");
        assert_eq!(
            resolver.expand_invocation(invocation, &HashMap::new()).unwrap().emitted,
            vec![Value::Int(Int::from(5050))]
        );
    }

    #[test]
    fn runaway_tail_calls_have_an_iteration_limit() {
        let source = "macro forever(n: int) -> int { @return forever(n)\n }\n\
                      macro calculate() { @emit forever(0)\n }\n\
                      calculate\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocation = find_invocation(&program, "calculate");
        assert!(matches!(
            resolver.expand_invocation(invocation, &HashMap::new()),
            Err(ResolveError::MacroTailCallLimitExceeded { name, max_iterations: 4096, .. })
                if name == "forever"
        ));
    }

    #[test]
    fn rejects_invocation_naming_a_non_macro_symbol() {
        let program = parse_fixture("invocation_names_struct.basm");

        let invocation = find_invocation(&program, "Reg");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        assert!(matches!(
            resolver.expand_invocation(invocation, &HashMap::new()),
            Err(ResolveError::ExpectedMacro { .. })
        ));
    }

    #[test]
    fn rejects_invocation_naming_nothing() {
        let program = parse_fixture("unknown_invocation.basm");

        let invocation = find_invocation(&program, "ghost");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        assert!(matches!(
            resolver.expand_invocation(invocation, &HashMap::new()),
            Err(ResolveError::UnknownMacro { .. })
        ));
    }

    #[test]
    fn non_pub_const_extends_scope_without_leaking() {
        let program = parse_fixture("local_const.basm");

        let declaration = find_macro(&program, "doubles");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("doubles").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(5))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(10))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn pub_const_with_a_spliced_name_resolves_a_distinct_name_per_invocation() {
        let program = parse_fixture("spliced_const_name.basm");

        let declaration = find_macro(&program, "make_reg64");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_reg64").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack)
            .unwrap();

        assert_eq!(result.generated.len(), 1);

        let Statement::Const(decl) = &result.generated[0] else {
            panic!("expected a generated const declaration");
        };

        assert_eq!(literal_name(&decl.name), Some("r3".to_string()));
    }

    #[test]
    fn pub_const_is_generated_with_splices_evaluated() {
        let program = parse_fixture("generated_pub_const.basm");

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(7))], &mut stack)
            .unwrap();

        assert_eq!(result.generated.len(), 1);

        let Statement::Const(decl) = &result.generated[0] else {
            panic!("expected a generated const declaration");
        };

        assert_eq!(literal_name(&decl.name), Some("the_reg".to_string()));
        assert!(decl.is_pub);

        // `Reg64(id = ...)` itself is untouched (still an unevaluated call,
        // not folded into a Value) — only the `` `v` `` splice inside it
        // was evaluated and rewritten.
        let Expr::Call { callee, arguments, .. } = &decl.value else {
            panic!("expected the generated const's value to still be an unevaluated call");
        };

        assert!(matches!(callee.as_ref(), Expr::Identifier { name, .. } if name == "Reg64"));
        assert_eq!(arguments.len(), 1);
        assert_eq!(arguments[0].name.as_deref(), Some("id"));

        assert!(matches!(
            &arguments[0].value,
            Expr::Integer { raw, .. } if raw == "7"
        ));
    }

    #[test]
    fn nested_declarations_are_captured_verbatim() {
        let program = parse_fixture("generated_declarations.basm");

        let declaration = find_macro(&program, "make_stuff");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_stuff").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack)
            .unwrap();

        assert_eq!(result.emitted, vec![Value::Int(Int::from(1))]);
        assert_eq!(result.generated.len(), 4);

        assert!(matches!(&result.generated[0], Statement::Struct(decl) if literal_name(&decl.name).as_deref() == Some("Foo")));
        assert!(matches!(&result.generated[1], Statement::TypeAlias(decl) if literal_name(&decl.name).as_deref() == Some("Bar")));
        assert!(matches!(&result.generated[2], Statement::Macro(decl) if literal_name(&decl.name).as_deref() == Some("helper")));
        assert!(matches!(&result.generated[3], Statement::Label(label) if label.name == "start"));

        // Top-level-only scoping: `collect_symbols` never descends into a
        // macro body, so this nested `start:` — captured verbatim into
        // `generated` above, still unresolved — was never registered as a
        // `SymbolKind::Label` and stays completely uninvolved in label
        // resolution.
        assert_eq!(symbols.lookup("start"), None);
    }

    #[test]
    fn nested_declaration_name_is_spliced_per_invocation() {
        let program = parse_fixture("generated_declaration_spliced_name.basm");

        let declaration = find_macro(&program, "make_reg_type");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_reg_type").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack)
            .unwrap();

        assert_eq!(result.generated.len(), 1);
        assert!(matches!(
            &result.generated[0],
            Statement::Struct(decl) if literal_name(&decl.name).as_deref() == Some("Reg3")
        ));
    }

    #[test]
    fn generic_macro_infers_type_param_from_a_real_array_argument() {
        let program = parse_fixture("generic_macro_array_updated.basm");

        let invocation = find_invocation(&program, "updated");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let result = resolver.expand_invocation(invocation, &HashMap::new()).unwrap();

        let Some(Value::Struct { fields, .. }) = result.returned else {
            panic!("expected `updated` to return a struct value");
        };

        let field = |name: &str| {
            fields
                .iter()
                .find(|(field_name, _)| field_name == name)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("expected a `{name}` field"))
        };

        assert_eq!(field("__el0"), Value::Int(Int::from(10)));
        assert_eq!(field("__el1"), Value::Int(Int::from(99)));
        assert_eq!(field("__el2"), Value::Int(Int::from(30)));
    }

    #[test]
    fn generated_declarations_bubble_up_through_nested_invocations() {
        let program = parse_fixture("nested_invocation_generates.basm");

        let declaration = find_macro(&program, "outer");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("outer").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(9))], &mut stack)
            .unwrap();

        assert_eq!(result.generated.len(), 1);

        let Statement::Const(decl) = &result.generated[0] else {
            panic!("expected a generated const declaration");
        };

        assert_eq!(literal_name(&decl.name), Some("the_const".to_string()));
        assert!(matches!(&decl.value, Expr::Integer { raw, .. } if raw == "9"));
    }

    #[test]
    fn two_invocations_generating_the_same_declaration_name_collide() {
        let program = parse_fixture("generated_duplicate_name.basm");

        let declaration = find_macro(&program, "outer_twice");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("outer_twice").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![], &mut stack);

        assert!(matches!(
            result,
            Err(ResolveError::DuplicateSymbol { name, .. }) if name == "the_const"
        ));
    }

    #[test]
    fn generated_const_leaves_non_spliced_identifiers_alone() {
        let program = parse_fixture("generated_const_leaves_bare_identifiers.basm");

        let declaration = find_macro(&program, "make_ref");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_ref").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![], &mut stack)
            .unwrap();

        let Statement::Const(decl) = &result.generated[0] else {
            panic!("expected a generated const declaration");
        };

        // No splice around `other_global` — left exactly as written, to be
        // resolved later wherever this generated const lands, not as a
        // reference into this expansion's now-gone scope.
        assert!(matches!(
            &decl.value,
            Expr::Identifier { name, .. } if name == "other_global"
        ));
    }

    // =============
    // @here / labels
    // =============

    #[test]
    fn at_here_counts_values_emitted_so_far() {
        let program = parse_fixture("here_basic.basm");

        let declaration = find_macro(&program, "emits_here");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("emits_here").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![], &mut stack).unwrap();

        // Nothing has emitted yet when `@here` is reached, so it reads 0 —
        // the index the very next `@emit` (99) will land at.
        assert_eq!(result.emitted, vec![Value::Int(Int::from(0)), Value::Int(Int::from(99))]);
    }

    #[test]
    fn at_here_reflects_nested_invocation_emits() {
        let program = parse_fixture("here_reflects_nested_invocation.basm");

        let declaration = find_macro(&program, "outer");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("outer").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![], &mut stack).unwrap();

        // outer's own `@emit 0`, then helper's two `@emit`s (1, 2) flatten
        // into the same shared counter before outer's own `@here` is
        // reached, so `@here == 3` — not just "how many statements outer
        // itself has run so far".
        assert_eq!(
            result.emitted,
            vec![
                Value::Int(Int::from(0)),
                Value::Int(Int::from(1)),
                Value::Int(Int::from(2)),
                Value::Int(Int::from(3)),
            ]
        );
    }

    #[test]
    fn bare_here_statement_is_unsupported() {
        let tokens = lexer::lex("macro foo() {\n    @here\n}\n").expect("fixture should lex");
        let program = parser::parse(tokens).expect("fixture should parse");

        let declaration = find_macro(&program, "foo");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("foo").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let error = resolver.run_macro_body(symbol, declaration, vec![], &mut stack).unwrap_err();

        assert!(matches!(
            error,
            ResolveError::UnsupportedMacroStatement { kind, .. } if kind == "@here"
        ));
    }

    #[test]
    fn backward_label_reference_resolves_to_recorded_position() {
        let program = parse_fixture("backward_label.basm");

        let declaration = find_macro(&program, "reads_label");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("reads_label").unwrap();
        let label_id = symbols.lookup("mylabel").unwrap();
        let consts = HashMap::new();

        let mut positions = HashMap::new();
        positions.insert(label_id, Int::from(0));

        let mut resolver =
            AliasResolver::new(&program, &symbols, &consts, LabelMode::Strict, positions);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![], &mut stack).unwrap();

        assert_eq!(result.emitted, vec![Value::Int(Int::from(0))]);
    }

    #[test]
    fn unknown_identifier_still_errors_under_tolerant_label_mode() {
        let program = parse_fixture("unknown_identifier_in_body.basm");

        let declaration = find_macro(&program, "reads_nothing");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("reads_nothing").unwrap();
        let consts = HashMap::new();

        // Tolerant mode only substitutes a placeholder for a *known*
        // label whose position isn't recorded yet — a name that isn't a
        // symbol at all keeps erroring immediately, in either mode.
        let mut resolver =
            AliasResolver::new(&program, &symbols, &consts, LabelMode::Tolerant, HashMap::new());

        let mut stack = Vec::new();
        let error = resolver.run_macro_body(symbol, declaration, vec![], &mut stack).unwrap_err();

        assert!(matches!(
            error,
            ResolveError::UnknownConstant { name, .. } if name == "does_not_exist"
        ));
    }

    #[test]
    fn label_name_colliding_with_a_const_is_a_duplicate_symbol() {
        let program = parse_fixture("label_collides_with_const.basm");

        let error = collect_symbols(&program).unwrap_err();

        assert!(matches!(
            error,
            ResolveError::DuplicateSymbol { name, .. } if name == "dup"
        ));
    }

    #[test]
    fn forward_and_backward_label_references_resolve_via_two_pass_discovery() {
        let program = parse_fixture("labels_forward_and_backward.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();

        // Pass 1 (position discovery, tolerant): mirrors
        // `main::resolve_and_expand`'s own two-pass driver — walk every
        // top-level statement, expanding invocations for real and
        // recording each label's position as it's reached. Forward
        // references get a silent placeholder; only the resulting
        // position map is kept.
        let mut discovery = AliasResolver::new(
            &program,
            &symbols,
            &consts,
            LabelMode::Tolerant,
            HashMap::new(),
        );

        for statement in &program.statements {
            match statement {
                Statement::Invocation(invocation) => {
                    discovery.expand_invocation(invocation, &HashMap::new()).unwrap();
                }

                Statement::Label(label) => {
                    let id = symbols.lookup(&label.name).unwrap();
                    discovery.record_label_position(id);
                }

                _ => {}
            }
        }

        let label_positions = discovery.into_label_positions();

        // Pass 2 (real, strict): rerun the identical walk, this time
        // keeping the emitted output.
        let mut resolver = AliasResolver::new(
            &program,
            &symbols,
            &consts,
            LabelMode::Strict,
            label_positions,
        );

        let mut emitted = Vec::new();

        for statement in &program.statements {
            match statement {
                Statement::Invocation(invocation) => {
                    let expansion = resolver.expand_invocation(invocation, &HashMap::new()).unwrap();
                    emitted.extend(expansion.emitted);
                }

                Statement::Label(label) => {
                    let id = symbols.lookup(&label.name).unwrap();
                    resolver.record_label_position(id);
                }

                _ => {}
            }
        }

        // noop -> 1; reads_target loop_start (backward, -1 instruction —
        // the exact worked proof from the design conversation); reads_target
        // skip_target (forward, +2 instructions — this is the case a
        // single-pass resolver would get wrong, since `skip_target` isn't
        // known yet the first time it's referenced); noop -> 1.
        assert_eq!(
            emitted,
            vec![
                Value::Int(Int::from(1)),
                Value::Int(Int::from(-1)),
                Value::Int(Int::from(2)),
                Value::Int(Int::from(1)),
            ]
        );
    }

    #[test]
    fn assert_passes_silently_and_body_continues() {
        let program = parse_fixture("assert_condition.basm");

        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("double").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(6))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn assert_failure_aborts_with_no_message() {
        let program = parse_fixture("assert_condition.basm");

        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("double").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(-1))], &mut stack);

        assert!(matches!(result, Err(ResolveError::AssertionFailed { message: None, .. })));
    }

    #[test]
    fn assert_failure_carries_its_message() {
        let program = parse_fixture("assert_with_message.basm");

        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("double").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(0))], &mut stack);

        match result {
            Err(ResolveError::AssertionFailed { message: Some(message), .. }) => {
                assert_eq!(message, "x must be positive");
            }
            other => panic!("expected an AssertionFailed with a message, got {other:?}"),
        }
    }

    #[test]
    fn assert_rejects_a_non_string_message() {
        let program = parse_fixture("assert_non_string_message.basm");

        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("double").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack);

        assert!(matches!(result, Err(ResolveError::InvalidAssertMessage { .. })));
    }

    #[test]
    fn assert_rejects_wrong_arity() {
        let program = parse_fixture("assert_wrong_arity.basm");

        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("double").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack);

        assert!(matches!(
            result,
            Err(ResolveError::InvalidArgumentCount { expected: 2, actual: 3, .. })
        ));
    }

    #[test]
    fn assert_rejects_a_struct_valued_condition() {
        let program = parse_fixture("assert_non_int_condition.basm");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("bad").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack);

        assert!(matches!(result, Err(ResolveError::ExpectedIntValue { .. })));
    }

    #[test]
    fn for_emits_each_value_in_the_range() {
        let program = parse_fixture("for_basic.basm");

        let declaration = find_macro(&program, "emit_range");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("emit_range").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(4))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![
                    Value::Int(Int::from(0)),
                    Value::Int(Int::from(1)),
                    Value::Int(Int::from(2)),
                    Value::Int(Int::from(3)),
                ],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn for_over_an_empty_range_emits_nothing() {
        let program = parse_fixture("for_basic.basm");

        let declaration = find_macro(&program, "emit_range");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("emit_range").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(0))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion { emitted: vec![], generated: vec![], returned: None }
        );
    }

    #[test]
    fn for_over_a_struct_value_visits_only_pub_fields_in_declaration_order() {
        let program = parse_fixture("for_over_struct_pub_fields.basm");

        let declaration = find_macro(&program, "emit_pub_fields");
        let symbols = collect_symbols(&program).unwrap();
        let macro_symbol = symbols.lookup("emit_pub_fields").unwrap();
        let mixed_symbol = symbols.lookup("Mixed").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let arg = Value::Struct {
            symbol: mixed_symbol,
            args: vec![],
            fields: vec![
                ("a".to_string(), Value::Int(Int::from(1))),
                ("b".to_string(), Value::Int(Int::from(2))),
                ("c".to_string(), Value::Int(Int::from(3))),
            ],
            nominal: None,
        };

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(macro_symbol, declaration, vec![arg], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(1)), Value::Int(Int::from(3))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn return_inside_for_stops_the_whole_body_not_just_the_iteration() {
        let program = parse_fixture("for_with_return.basm");

        let declaration = find_macro(&program, "loop_then_return");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("loop_then_return").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(3))], &mut stack)
            .unwrap();

        // Only the first iteration runs — its `@return` exits the whole
        // body, so neither later iterations nor the trailing `@emit 999`
        // after the loop ever run.
        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(0))],
                generated: vec![],
                returned: Some(Value::Int(Int::from(0))),
            }
        );
    }

    #[test]
    fn if_true_takes_the_then_branch() {
        let program = parse_fixture("if_else.basm");

        let declaration = find_macro(&program, "sign");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("sign").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(5))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(1))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn if_false_takes_the_else_branch() {
        let program = parse_fixture("if_else.basm");

        let declaration = find_macro(&program, "sign");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("sign").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(-5))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(-1))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn if_with_no_else_and_a_false_condition_falls_through() {
        let program = parse_fixture("if_no_else.basm");

        let declaration = find_macro(&program, "maybe_emit");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("maybe_emit").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(-1))], &mut stack)
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![Value::Int(Int::from(0))],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn if_nested_inside_for_picks_a_branch_per_iteration() {
        let program = parse_fixture("for_if_nested.basm");

        let declaration = find_macro(&program, "replace_at");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("replace_at").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(
                symbol,
                declaration,
                vec![Value::Int(Int::from(4)), Value::Int(Int::from(2)), Value::Int(Int::from(99))],
                &mut stack,
            )
            .unwrap();

        assert_eq!(
            result,
            MacroExpansion {
                emitted: vec![
                    Value::Int(Int::from(0)),
                    Value::Int(Int::from(1)),
                    Value::Int(Int::from(99)),
                    Value::Int(Int::from(3)),
                ],
                generated: vec![],
                returned: None,
            }
        );
    }

    #[test]
    fn macro_overloads_dispatch_by_resolved_argument_type() {
        let source = "struct Reg { pub id: int }\n\
                      macro encode(value: int) { @emit 11\n }\n\
                      macro encode(value: Reg) { @emit 22\n }\n\
                      encode 7\nencode Reg(3)\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        assert_eq!(symbols.lookup_all("encode").len(), 2);

        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocations: Vec<_> = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) => Some(invocation),
                _ => None,
            })
            .collect();

        assert_eq!(
            resolver.expand_invocation(invocations[0], &HashMap::new()).unwrap().emitted,
            vec![Value::Int(Int::from(11))]
        );
        assert_eq!(
            resolver.expand_invocation(invocations[1], &HashMap::new()).unwrap().emitted,
            vec![Value::Int(Int::from(22))]
        );
    }

    #[test]
    fn macro_overloads_dispatch_by_arity() {
        let source = "macro pick(value: int) { @emit 1\n }\n\
                      macro pick(left: int, right: int) { @emit 2\n }\n\
                      pick 3\npick 3, 4\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let emitted: Vec<_> = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) => Some(invocation),
                _ => None,
            })
            .map(|invocation| {
                resolver.expand_invocation(invocation, &HashMap::new()).unwrap().emitted[0].clone()
            })
            .collect();
        assert_eq!(emitted, vec![Value::Int(Int::from(1)), Value::Int(Int::from(2))]);
    }

    #[test]
    fn macro_defaults_are_evaluated_left_to_right() {
        let source = "macro scale(value: int, places: int = 2, factor: int = 10 * places) { @emit value * factor\n }\n\
                      scale 3\nscale 3, 4\nscale 3, 4, 5\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let emitted: Vec<_> = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) => Some(invocation),
                _ => None,
            })
            .map(|invocation| resolver.expand_invocation(invocation, &HashMap::new()).unwrap().emitted[0].clone())
            .collect();
        assert_eq!(emitted, vec![Value::Int(Int::from(60)), Value::Int(Int::from(120)), Value::Int(Int::from(15))]);
    }

    #[test]
    fn expression_macro_calls_apply_defaults() {
        let source = "macro precision(value: int = 10) -> int { @return value\n }\n\
                      macro use_default() { @emit precision()\n }\n\
                      use_default\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocation = find_invocation(&program, "use_default");
        assert_eq!(
            resolver.expand_invocation(invocation, &HashMap::new()).unwrap().emitted,
            vec![Value::Int(Int::from(10))]
        );
    }

    #[test]
    fn overload_matching_accepts_omitted_default_arguments() {
        let source = "struct Reg { pub id: int }\n\
                      macro choose(value: int, precision: int = 10) { @emit 1\n }\n\
                      macro choose(value: Reg, precision: int = 10) { @emit 2\n }\n\
                      choose 3\nchoose Reg(4)\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let emitted: Vec<_> = program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) => Some(invocation),
                _ => None,
            })
            .map(|invocation| resolver.expand_invocation(invocation, &HashMap::new()).unwrap().emitted[0].clone())
            .collect();
        assert_eq!(emitted, vec![Value::Int(Int::from(1)), Value::Int(Int::from(2))]);
    }

    #[test]
    fn identical_macro_overloads_are_ambiguous_at_the_call_site() {
        let source = "macro choose(value: int) { @emit 1\n }\n\
                      macro choose(other: int) { @emit 2\n }\n\
                      choose 3\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocation = find_invocation(&program, "choose");

        assert!(matches!(
            resolver.expand_invocation(invocation, &HashMap::new()),
            Err(ResolveError::AmbiguousMacroOverload { name, .. }) if name == "choose"
        ));
    }

    #[test]
    fn overloaded_macro_without_a_matching_signature_is_rejected() {
        let source = "struct Reg { pub id: int }\n\
                      macro choose(value: int) { @emit 1\n }\n\
                      macro choose(value: Reg) { @emit 2\n }\n\
                      choose 1, 2\n";
        let program = parser::parse(lexer::lex(source).unwrap()).unwrap();
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);
        let invocation = find_invocation(&program, "choose");

        assert!(matches!(
            resolver.expand_invocation(invocation, &HashMap::new()),
            Err(ResolveError::NoMatchingMacroOverload { name, actual, .. })
                if name == "choose" && actual == ["int", "int"]
        ));
    }

    #[test]
    fn if_rejects_a_struct_valued_condition() {
        let program = parse_fixture("if_non_int_condition.basm");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("bad").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack);

        assert!(matches!(result, Err(ResolveError::ExpectedIntValue { .. })));
    }
}
