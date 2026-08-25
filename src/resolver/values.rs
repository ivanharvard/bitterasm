//! Evaluates an [`Expr`] to a concrete [`Value`] — either a leaf [`Int`] or
//! a named struct built out of more `Value`s — for `@emit`. This is a
//! generalization of [`crate::eval::eval`] (which only ever produces an
//! `Int` and explicitly rejects [`Expr::Member`]/[`Expr::Call`]) rather than
//! a replacement for it: arithmetic subtrees still go through `eval::eval`
//! unchanged, this module only adds struct construction (`Expr::Call`
//! resolved against the symbol table, the same way a type position would)
//! and field access (`Expr::Member`) on top.
//!
//! `Value::Struct` deliberately carries the same [`ResolvedGenericArg`]s a
//! [`ResolvedType::Struct`] would, rather than any notion of "width" or
//! "bits" — what those args *mean* is entirely up to whichever `.basm`
//! package declared the struct and whichever exporter reads the emitted
//! value; this evaluator only threads them through.

use std::collections::HashMap;

use crate::ast::{literal_name, CallArgument, ConstructItem, Expr, NamePart, Statement};
use crate::eval::{self, EvalError, Int};
use crate::token::Span;
use crate::types::{GenericParameter, StructBodyItem, TypeArgument};

use super::aliases::{AliasResolver, LabelMode};
use super::consts::find_const_declaration;
use super::structs::describe_type;
use super::symbols::{SymbolId, SymbolKind};
use super::types::{BuiltinType, ResolvedGenericArg, ResolvedType};
use super::ResolveError;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Int),
    Struct {
        symbol: SymbolId,
        args: Vec<ResolvedGenericArg>,
        fields: Vec<(String, Value)>,

        /// The outermost nominal (invariant-bearing) `type` alias this
        /// value was produced *as*, if any — set only by `@as`
        /// (`AliasResolver::convert_to`), `None` everywhere else (a struct
        /// built by `eval_call_value`/`eval_construct_value` directly, even
        /// through an alias name, isn't tagged — see those functions' own
        /// comments). Re-resolving this symbol (`AliasResolver::resolve_alias`,
        /// already memoized) reconstructs the full `ResolvedType::Alias` —
        /// including any *further* nesting, since "holds all the way down"
        /// means the outermost alias's own resolution already carries its
        /// whole chain — so this is the only piece `Value` itself needs to
        /// remember; see `AliasResolver::value_type`.
        nominal: Option<SymbolId>,
    },
}

// Lazy-resolve-and-memoize state for a top-level const's `Value`, mirroring
// `aliases::AliasState`/`consts::ConstState` — see
// `AliasResolver::resolve_const_value`.
#[derive(Debug, Clone)]
pub(super) enum ConstValueState {
    Unvisited,
    Visiting,
    Resolved(Value),
}

impl<'a> AliasResolver<'a> {
    pub fn eval_value(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<Value, ResolveError> {
        match expr {
            // Arithmetic subtrees are handed to the existing Int evaluator
            // whole, against an Int-only projection of `scope` — this is
            // why an expression like `dst.id + 1` isn't supported yet (see
            // module doc): a Member/Call leaf inside a Binary/Unary tree
            // never reaches this match arm to be resolved as a Value.
            // `@here` has the same problem in principle (`eval::eval`
            // doesn't know what expansion is in progress) but is common
            // enough to be worth the small fix: substitute every `@here`
            // leaf with its current value as a literal first, so `target -
            // @here` still folds through the ordinary Int evaluator.
            Expr::Integer { .. } | Expr::Unary { .. } | Expr::Binary { .. } => {
                let rewritten = substitute_here(expr, &self.values_emitted);
                let int_scope = int_only_scope(scope);

                eval::eval(&rewritten, &int_scope)
                    .map(Value::Int)
                    .map_err(|error| into_value_error(error, scope))
            }

            // A name not bound in the current scope (a macro's own params,
            // or nothing at all at the top level) falls back to a
            // top-level `const` of the same name — `zero`/`x1`-style named
            // constants need to resolve the same way whether they're used
            // to build another const's value or passed as an invocation
            // operand, not just when some enclosing macro happens to have
            // bound that exact name itself.
            Expr::Identifier { name, span } => match scope.get(name) {
                Some(value) => Ok(value.clone()),

                None => match self.lookup_symbol(name) {
                    Some(id) if self.get_symbol(id).kind == SymbolKind::Label => {
                        self.resolve_label_value(id, *span)
                    }

                    _ => self.resolve_const_value(name, *span),
                },
            },

            // How many values have been `@emit`'d so far, in whole-program
            // order — see `AliasResolver::values_emitted`.
            Expr::Here { .. } => Ok(Value::Int(self.values_emitted.clone())),

            Expr::Member { object, member, span } => match self.eval_value(object, scope)? {
                Value::Struct { symbol, fields, .. } => fields
                    .into_iter()
                    .find(|(name, _)| name == member)
                    .map(|(_, value)| value)
                    .ok_or_else(|| ResolveError::UnknownField {
                        type_name: self.get_symbol(symbol).name.clone(),
                        field: member.clone(),
                        span: *span,
                    }),

                Value::Int(_) => Err(ResolveError::ExpectedStructValue { span: *span }),
            },

            Expr::Call { callee, arguments, span } => {
                self.eval_call_value(callee, arguments, *span, scope)
            }

            Expr::Construct { callee, generic_args, fields, span } => {
                self.eval_construct_value(callee, generic_args, fields, *span, scope)
            }

            Expr::As { value, ty, span } => {
                let value = self.eval_value(value, scope)?;
                let target = self.resolve_type_expr(ty)?;

                self.convert_to(value, &target, *span)
            }

            // A string literal is just a (possibly large) `Int` — its bytes
            // packed big-endian into one arbitrary-precision integer, the
            // same way a char literal already desugars to its codepoint at
            // lex time (`lexer::lex_char`). Nothing about *length* survives
            // this: `"\0A"` and `"A"` pack to the identical value, which is
            // exactly why interpreting a packed int as "a string of length
            // N" always requires an explicit `N` (`@as String<N>`) rather
            // than ever being inferred back out of the number itself.
            Expr::String { value, .. } => {
                Ok(Value::Int(Int::from_bytes_be(num_bigint::Sign::Plus, value.as_bytes())))
            }

            // Transparent here too — `@emit`'s argument is already always
            // evaluated, so a splice around it changes nothing.
            Expr::Splice { inner, .. } => self.eval_value(inner, scope),

            // `start..end` sugar — synthesizes a private struct value with
            // one pub field per element (see
            // `resolver::generated::eval_range_value`). `@for`'s four call
            // sites all consume the result uniformly, like any other
            // struct-valued `in`-expression.
            Expr::Range { start, end, span } => self.eval_range_value(start, end, *span, scope),
        }
    }

    fn eval_call_value(
        &mut self,
        callee: &Expr,
        arguments: &[CallArgument],
        span: Span,
        scope: &HashMap<String, Value>,
    ) -> Result<Value, ResolveError> {
        let Expr::Identifier { name, span: callee_span } = callee else {
            return Err(ResolveError::UnsupportedCallExpression { span: callee.span() });
        };

        let resolved = self.resolve_named_type(std::slice::from_ref(name), *callee_span)?;

        // A callee naming a nominal alias (`Percent(value = 50)` where
        // `Percent` carries its own `invariant`) still constructs the
        // underlying struct — unwrapping here just preserves that this
        // already worked for a *plain* alias before nominal wrapping
        // existed. It does mean the alias's own invariant isn't checked
        // through this path yet (only the underlying struct's, via
        // `check_struct_invariants` below) — only `@as`/a checked `const`
        // check an alias's own invariant, once those exist.
        let ResolvedType::Struct { symbol, args } = resolved.strip_alias().clone() else {
            return Err(ResolveError::ExpectedStructCallee {
                name: name.clone(),
                span: *callee_span,
            });
        };

        let declared_fields: Vec<(String, Option<Expr>)> = self
            .find_struct_declaration(symbol)?
            .fields
            .iter()
            .filter_map(|item| match item {
                StructBodyItem::Field(field) => {
                    literal_name(&field.name).map(|name| (name, field.default.clone()))
                }

                // `@for`/`@if`-generated fields aren't visible through
                // this paren-call construction path yet — it predates
                // generative struct bodies and doesn't set up the
                // generic-const scope their `@for`/`@if` would need to
                // unroll against. Brace-literal construction is where that
                // support belongs.
                StructBodyItem::For { .. } | StructBodyItem::If { .. } => None,
            })
            .collect();

        let field_names: Vec<String> = declared_fields.iter().map(|(name, _)| name.clone()).collect();
        let required_count = declared_fields.iter().filter(|(_, default)| default.is_none()).count();

        if arguments.len() < required_count || arguments.len() > field_names.len() {
            return Err(ResolveError::InvalidArgumentCount {
                name: name.clone(),
                expected: field_names.len(),
                actual: arguments.len(),
                span,
            });
        }

        let mut by_name: HashMap<String, Value> = HashMap::new();

        for (index, argument) in arguments.iter().enumerate() {
            let field_name = match &argument.name {
                Some(explicit) => {
                    if !field_names.contains(explicit) {
                        return Err(ResolveError::UnknownField {
                            type_name: name.clone(),
                            field: explicit.clone(),
                            span: argument.span,
                        });
                    }

                    explicit.clone()
                }

                None => field_names[index].clone(),
            };

            let value = self.eval_value(&argument.value, scope)?;
            by_name.insert(field_name, value);
        }

        // A field the caller didn't supply falls back to its own declared
        // default, evaluated against the struct's own bound generic const
        // args only — never sibling field values, so field-to-field
        // dependencies (and the evaluation-order question they'd raise)
        // never come up.
        let default_scope: HashMap<String, Value> = self
            .find_struct_declaration(symbol)?
            .generic_params
            .clone()
            .iter()
            .zip(&args)
            .filter_map(|(param, arg)| match (param, arg) {
                (GenericParameter::Const { name, .. }, ResolvedGenericArg::Const(value)) => {
                    Some((name.clone(), Value::Int(value.clone())))
                }
                _ => None,
            })
            .collect();

        let struct_ty = ResolvedType::Struct { symbol, args: args.clone() };
        let mut fields = Vec::with_capacity(field_names.len());

        for (field_name, default) in declared_fields {
            let value = match by_name.remove(&field_name) {
                Some(value) => value,
                None => {
                    let default = default.ok_or_else(|| ResolveError::Internal {
                        message: format!(
                            "struct call to `{name}` passed arity/default checks but field \
                             `{field_name}` has neither an argument nor a default",
                        ),
                        span,
                    })?;

                    self.eval_value(&default, &default_scope)?
                }
            };

            let expected = self.field_type(&struct_ty, &field_name, span)?;
            let actual = self.value_type(&value)?;

            if actual != expected {
                return Err(ResolveError::TypeMismatch {
                    name: field_name,
                    expected: describe_type(&expected, self.symbols),
                    actual: describe_type(&actual, self.symbols),
                    span,
                });
            }

            fields.push((field_name, value));
        }

        self.check_struct_invariants(symbol, &args, &fields, span)?;

        // Constructing directly through a nominal alias's own name (rather
        // than `@as`) doesn't tag the result — see the comment where its
        // callee gets resolved, above. `nominal: None` here is that
        // decision made concrete, not an oversight.
        Ok(Value::Struct { symbol, args, fields, nominal: None })
    }

    /// `Array<u8, N> { field: value, ... }` — the brace-literal counterpart
    /// of `eval_call_value`, which predates generative struct bodies and
    /// can only see a struct's flat, non-`@for`/`@if` fields (see that
    /// function's comment). This one goes through
    /// `structs::AliasResolver::instantiate_struct_fields_named` instead, so
    /// every field the concrete instantiation actually generates —
    /// including `@for`/`@if`-produced ones — is visible to match against.
    fn eval_construct_value(
        &mut self,
        callee: &Expr,
        generic_args: &[TypeArgument],
        fields: &[ConstructItem],
        span: Span,
        scope: &HashMap<String, Value>,
    ) -> Result<Value, ResolveError> {
        let Expr::Identifier { name, span: callee_span } = callee else {
            return Err(ResolveError::UnsupportedCallExpression { span: callee.span() });
        };

        let resolved = self.resolve_named_type(std::slice::from_ref(name), *callee_span)?;

        // See the identical unwrap in `eval_call_value` just above — same
        // reasoning applies to brace-literal construction through an alias
        // name.
        let ResolvedType::Struct { symbol, args: bare_args } = resolved.strip_alias().clone() else {
            return Err(ResolveError::ExpectedStructCallee {
                name: name.clone(),
                span: *callee_span,
            });
        };

        let args = if generic_args.is_empty() {
            bare_args
        } else {
            self.resolve_construct_generic_args(symbol, generic_args, span, scope)?
        };

        let declared = self.instantiate_struct_fields_named(symbol, &args)?;
        let provided = self.unroll_construct_items(fields, scope)?;

        let required_count = declared.iter().filter(|(_, _, _, default)| default.is_none()).count();

        if provided.len() < required_count || provided.len() > declared.len() {
            return Err(ResolveError::InvalidArgumentCount {
                name: name.clone(),
                expected: declared.len(),
                actual: provided.len(),
                span,
            });
        }

        let mut by_name: HashMap<String, Value> = HashMap::new();

        for (field_name, value) in provided {
            if !declared.iter().any(|(declared_name, ..)| *declared_name == field_name) {
                return Err(ResolveError::UnknownField {
                    type_name: name.clone(),
                    field: field_name,
                    span,
                });
            }

            by_name.insert(field_name, value);
        }

        // Same default-eval scope choice as `eval_call_value`: the struct's
        // own bound generic const args only, never sibling field values.
        let default_scope: HashMap<String, Value> = self
            .find_struct_declaration(symbol)?
            .generic_params
            .clone()
            .iter()
            .zip(&args)
            .filter_map(|(param, arg)| match (param, arg) {
                (GenericParameter::Const { name, .. }, ResolvedGenericArg::Const(value)) => {
                    Some((name.clone(), Value::Int(value.clone())))
                }
                _ => None,
            })
            .collect();

        let mut result_fields = Vec::with_capacity(declared.len());

        for (field_name, expected, _is_pub, default) in declared {
            let value = match by_name.remove(&field_name) {
                Some(value) => value,
                None => {
                    let default = default.ok_or_else(|| ResolveError::Internal {
                        message: format!(
                            "brace construction of `{name}` passed arity/default checks but field \
                             `{field_name}` has neither a provided value nor a default",
                        ),
                        span,
                    })?;

                    self.eval_value(&default, &default_scope)?
                }
            };

            let actual = self.value_type(&value)?;

            if actual != expected {
                return Err(ResolveError::TypeMismatch {
                    name: field_name,
                    expected: describe_type(&expected, self.symbols),
                    actual: describe_type(&actual, self.symbols),
                    span,
                });
            }

            result_fields.push((field_name, value));
        }

        self.check_struct_invariants(symbol, &args, &result_fields, span)?;

        // See the identical comment in `eval_call_value` — not tagged.
        Ok(Value::Struct { symbol, args, fields: result_fields, nominal: None })
    }

    // Resolves a brace-literal construction's already-parsed generic
    // arguments (`Array<u8, N>`'s `<u8, N>`) into `ResolvedGenericArg`s.
    // Deliberately doesn't reuse `aliases::AliasResolver::resolve_generic_args`
    // (the type-position equivalent): a const argument here — `N` in
    // `Array<u8, arr.len>` — may reference a bound macro parameter
    // (`arr.len`, a member access on one), which only exists in this live
    // `scope: Value` map, not in the consts/generic-scope-only world
    // `eval_const_expr` evaluates against. `eval_int` (below) is what
    // already threads a macro's own arguments through arithmetic, so a
    // construction's generic const args go through it too.
    fn resolve_construct_generic_args(
        &mut self,
        symbol: SymbolId,
        args: &[TypeArgument],
        span: Span,
        scope: &HashMap<String, Value>,
    ) -> Result<Vec<ResolvedGenericArg>, ResolveError> {
        let expected = self.find_struct_declaration(symbol)?.generic_params.len();

        if args.len() != expected {
            let name = self.get_symbol(symbol).name.clone();

            return Err(ResolveError::InvalidGenericArity {
                name,
                expected,
                actual: args.len(),
                span,
            });
        }

        args.iter()
            .map(|arg| match arg {
                TypeArgument::Type(ty) => {
                    Ok(ResolvedGenericArg::Type(Box::new(self.resolve_type_expr(ty)?)))
                }

                TypeArgument::Const(expr) => Ok(ResolvedGenericArg::Const(self.eval_int(expr, scope)?)),
            })
            .collect()
    }

    // Expands a construction's own `@for`/`@if` items into concrete
    // `(field_name, Value)` pairs, given the live macro invocation `scope`
    // (not `self.generic_scope` — a construction's `@for`/`@if` bounds and
    // field values are ordinary expressions evaluated against bound
    // parameters, e.g. `@for i in 0..arr.len`, the same as a macro body's
    // own `@for`/`@if` in `macro_body::walk_macro_body`, which this mirrors
    // exactly). Each field's value is evaluated *eagerly*, inside its own
    // iteration's scope — unlike `structs::unroll_struct_body`, which
    // defers resolving a field's type until after unrolling completes, a
    // field's *value* commonly depends directly on the loop variable
    // (`__el\`i\`: i * 2`), so there's no scope left to resolve it in once
    // unrolling is done.
    fn unroll_construct_items(
        &mut self,
        items: &[ConstructItem],
        scope: &HashMap<String, Value>,
    ) -> Result<Vec<(String, Value)>, ResolveError> {
        let mut fields = Vec::new();

        for item in items {
            match item {
                ConstructItem::Field { name, value, .. } => {
                    let field_name = self.resolve_spliced_name(name, scope)?;
                    let field_value = self.eval_value(value, scope)?;
                    fields.push((field_name, field_value));
                }

                ConstructItem::For { var, source, body, .. } => {
                    let bindings = self.eval_for_source(source, scope)?;

                    for (_, value) in bindings {
                        let mut iter_scope = scope.clone();
                        iter_scope.insert(var.clone(), value);

                        fields.extend(self.unroll_construct_items(body, &iter_scope)?);
                    }
                }

                ConstructItem::If { condition, body, else_body, .. } => {
                    let chosen = if self.eval_truthy(condition, scope)? {
                        Some(body)
                    } else {
                        else_body.as_ref()
                    };

                    if let Some(chosen) = chosen {
                        fields.extend(self.unroll_construct_items(chosen, scope)?);
                    }
                }
            }
        }

        Ok(fields)
    }

    // `@as`'s entire job: convert `value` into `target`, which may be a
    // chain of nominal `ResolvedType::Alias` layers wrapping an eventual
    // `Builtin`/`Struct`. Each alias layer's own `invariant`(s) are checked
    // against `value` *as given* (never against some already-wrapped
    // shape — see this session's "\0A" vs "A" discussion: a layer's binder
    // means "the value being converted right now," consistently, no matter
    // how deep the chain goes), using that layer's own chosen binder name
    // (`crate::facets::invariant`'s module doc). Once every alias layer is
    // unwrapped, the terminal struct is handled two ways: `value` is
    // already that exact struct (nothing to do — its own invariant was
    // already checked when it was built), or it isn't, in which case it's
    // auto-wrapped into the struct's one field (recursively — "holds all
    // the way down") and the struct's own `invariant` is checked
    // (`check_struct_invariants`, the same check every other construction
    // path already goes through).
    pub(super) fn convert_to(
        &mut self,
        value: Value,
        target: &ResolvedType,
        span: Span,
    ) -> Result<Value, ResolveError> {
        match target {
            ResolvedType::Alias { symbol, binder, invariants, underlying } => {
                let mut layer_scope: HashMap<String, Value> = HashMap::new();

                if let Some(binder) = binder {
                    layer_scope.insert(binder.clone(), value.clone());
                }

                for invariant in invariants {
                    if !self.eval_truthy(invariant, &layer_scope)? {
                        return Err(ResolveError::InvariantViolated {
                            type_name: self.get_symbol(*symbol).name.clone(),
                            invariant: crate::printer::print_expr(invariant),
                            span,
                        });
                    }
                }

                let converted = self.convert_to(value, underlying, span)?;

                Ok(tag_nominal(converted, *symbol))
            }

            ResolvedType::Struct { symbol, args } => match &value {
                Value::Struct { symbol: value_symbol, args: value_args, .. }
                    if value_symbol == symbol && value_args == args =>
                {
                    Ok(value)
                }

                _ => {
                    let declared = self.instantiate_struct_fields_named(*symbol, args)?;

                    let [(field_name, field_ty, ..)] = declared.as_slice() else {
                        return Err(ResolveError::CannotCoerce {
                            type_name: self.get_symbol(*symbol).name.clone(),
                            span,
                        });
                    };

                    let wrapped = self.convert_to(value, field_ty, span)?;
                    let fields = vec![(field_name.clone(), wrapped)];

                    self.check_struct_invariants(*symbol, args, &fields, span)?;

                    Ok(Value::Struct { symbol: *symbol, args: args.clone(), fields, nominal: None })
                }
            },

            ResolvedType::Builtin(BuiltinType::Int) => match value {
                Value::Int(_) => Ok(value),
                Value::Struct { .. } => Err(ResolveError::ExpectedIntValue { span }),
            },

            ResolvedType::TypeParameter { name } => {
                Err(ResolveError::ExpectedType { name: name.clone(), span })
            }
        }
    }

    /// Resolves a bare name that isn't bound in the current scope against
    /// a top-level `const` of the same name, lazily and memoized — same
    /// lazy-resolve-and-memoize shape as `AliasResolver`'s own type-alias
    /// resolution (`const_value_states`/`const_value_stack`, tracked
    /// separately from that mechanism's own `states`/`stack`; see their
    /// declaration for why). A name that isn't a symbol at all, or is a
    /// symbol but not a const (a struct or type alias used where a value
    /// was expected), is `UnknownConstant` — the same error a truly
    /// unbound name would produce, since neither case has a `Value` to
    /// offer.
    pub(super) fn resolve_const_value(&mut self, name: &str, span: Span) -> Result<Value, ResolveError> {
        let Some(id) = self.lookup_symbol(name) else {
            return Err(ResolveError::UnknownConstant { name: name.to_string(), span });
        };

        if self.get_symbol(id).kind != SymbolKind::Const {
            return Err(ResolveError::UnknownConstant { name: name.to_string(), span });
        }

        match self.const_value_states.get(&id) {
            Some(ConstValueState::Resolved(value)) => return Ok(value.clone()),
            Some(ConstValueState::Visiting) => return Err(self.make_const_value_cycle_error(id)),
            Some(ConstValueState::Unvisited) => {}

            // Not pre-seeded by `AliasResolver::new` (which only ever saw
            // `self.symbols`, before this `id` existed) — a symbol
            // discovered mid-resolution starts out `Unvisited` the first
            // time anything actually references it, exactly as if it had
            // been seeded from the start.
            None => {
                self.const_value_states.insert(id, ConstValueState::Unvisited);
            }
        }

        self.const_value_states.insert(id, ConstValueState::Visiting);
        self.const_value_stack.push(id);

        let declaration = match find_const_declaration(self.program, name, self.symbols) {
            Ok(declaration) => Ok(declaration.clone()),
            Err(error) => self
                .generated
                .iter()
                .find_map(|statement| match statement {
                    Statement::Const(decl) if literal_name(&decl.name).as_deref() == Some(name) => {
                        Some(decl.clone())
                    }
                    _ => None,
                })
                .ok_or(error),
        };

        let result = declaration.and_then(|declaration| {
            let value = declaration.value.clone();
            let ty = declaration.ty.clone();
            let value = self.eval_value(&value, &HashMap::new())?;

            match ty {
                Some(ty) => {
                    let target = self.resolve_type_expr(&ty)?;
                    self.convert_to(value, &target, declaration.span)
                }
                None => Ok(value),
            }
        });

        self.const_value_stack.pop();

        match result {
            Ok(value) => {
                self.const_value_states.insert(id, ConstValueState::Resolved(value.clone()));
                Ok(value)
            }

            Err(error) => {
                self.const_value_states.insert(id, ConstValueState::Unvisited);
                Err(error)
            }
        }
    }

    /// Resolves a top-level label's `SymbolId` to the value-count position
    /// it was recorded at (see `AliasResolver::record_label_position`).
    /// `id` is already known to be `SymbolKind::Label` by the caller.
    pub(super) fn resolve_label_value(&mut self, id: SymbolId, span: Span) -> Result<Value, ResolveError> {
        match self.label_positions.get(&id) {
            Some(position) => Ok(Value::Int(position.clone())),

            None => match self.label_mode {
                // Position-discovery pass: this label's own `foo:` line
                // hasn't been walked yet (a forward reference) — a
                // placeholder lets expansion keep running long enough to
                // discover every label's position, including this one.
                // The value itself is meaningless; only the *count* of
                // values this expansion goes on to emit matters, and that
                // count is unaffected by which placeholder is used (see
                // the module doc on why that invariant holds today).
                LabelMode::Tolerant => Ok(Value::Int(Int::from(0))),

                // The real pass: every top-level label's position was
                // already recorded by a completed discovery pass, so
                // reaching here means this resolver was seeded with an
                // incomplete map — a bug in the two-pass driver, not a
                // BitterASM source error.
                LabelMode::Strict => Err(ResolveError::Internal {
                    message: format!(
                        "label `{}` has no recorded position — the position-discovery pass \
                         should have found every top-level label before this pass ran",
                        self.get_symbol(id).name,
                    ),
                    span,
                }),
            },
        }
    }

    /// Evaluates `expr` and requires the result to be an `Int` — shared by
    /// every caller that needs a plain integer rather than the general
    /// `Value` (`@for`'s range bounds; `@if`/`@assert`'s condition goes
    /// through [`Self::eval_truthy`] instead, which also needs this).
    pub(super) fn eval_int(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<Int, ResolveError> {
        match self.eval_value(expr, scope)? {
            Value::Int(value) => Ok(value),
            Value::Struct { .. } => Err(ResolveError::ExpectedIntValue { span: expr.span() }),
        }
    }

    /// Evaluates `expr` and checks it under the language's `0`/`1` `Int`
    /// convention for booleans (see the [`crate::eval`] module doc) — used
    /// by `@if` and `@assert` alike.
    pub(super) fn eval_truthy(
        &mut self,
        expr: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<bool, ResolveError> {
        Ok(self.eval_int(expr, scope)? != Int::from(0))
    }

    /// Resolves a (possibly spliced) name to a literal string against a
    /// live macro invocation's `Value` scope — the counterpart, for a
    /// macro-body-generated `pub const`'s name, of
    /// `structs::AliasResolver::resolve_spliced_name_as_const` for a
    /// struct field's name (which has no macro-body scope to evaluate
    /// against, only `generic_scope`/top-level consts).
    pub(super) fn resolve_spliced_name(
        &mut self,
        parts: &[NamePart],
        scope: &HashMap<String, Value>,
    ) -> Result<String, ResolveError> {
        let mut out = String::new();

        for part in parts {
            match part {
                NamePart::Literal(text) => out.push_str(text),
                NamePart::Splice(expr) => out.push_str(&self.eval_int(expr, scope)?.to_string()),
            }
        }

        Ok(out)
    }

    fn make_const_value_cycle_error(&self, repeated: SymbolId) -> ResolveError {
        let start = self.const_value_stack.iter().position(|id| *id == repeated).unwrap_or(0);

        let mut cycle: Vec<String> = self.const_value_stack[start..]
            .iter()
            .map(|id| self.get_symbol(*id).name.clone())
            .collect();

        cycle.push(self.get_symbol(repeated).name.clone());

        ResolveError::CyclicConstant {
            cycle,
            span: self.get_symbol(repeated).span,
        }
    }

    // A `Value`'s type is already implicit in what it is — an Int, or which
    // struct (identity + resolved generic args) — except for `nominal`,
    // which needs re-resolving (`resolve_alias`, already memoized) to
    // reconstruct the full `ResolvedType::Alias` a tagged value's type
    // actually is — see `Value::Struct::nominal`'s doc.
    pub(super) fn value_type(&mut self, value: &Value) -> Result<ResolvedType, ResolveError> {
        Ok(match value {
            Value::Int(_) => ResolvedType::Builtin(BuiltinType::Int),

            Value::Struct { symbol, args, nominal: Some(alias), .. } => {
                let resolved = self.resolve_alias(*alias)?;

                debug_assert!(
                    matches!(&resolved, ResolvedType::Alias { .. }),
                    "a Value tagged `nominal` should only ever be tagged with a symbol that \
                     actually resolves to ResolvedType::Alias — {symbol:?}/{args:?} tagged with \
                     {alias:?}, which resolved to {resolved:?} instead",
                );

                resolved
            }

            Value::Struct { symbol, args, nominal: None, .. } => ResolvedType::Struct {
                symbol: *symbol,
                args: args.clone(),
            },
        })
    }
}

// `AliasResolver::convert_to`'s final step: mark `value` as having been
// produced *as* the nominal alias `symbol`, once every layer's invariant
// along the way already checked out. A plain `Value::Int` has nowhere to
// remember this — unlike `Value::Struct`, it carries no `nominal` slot — so
// tagging one is a no-op: the invariant was still enforced right now by the
// caller, but the *value* itself can't carry "this is a PositiveInt"
// forward the way a tagged struct can, meaning a macro param/field typed as
// such an alias needs its own `@as` at that point too. A known scope
// boundary of today's `Value` shape, not an oversight — see
// `Value::Struct::nominal`'s doc.
fn tag_nominal(value: Value, symbol: SymbolId) -> Value {
    match value {
        Value::Struct { symbol: inner_symbol, args, fields, .. } => {
            Value::Struct { symbol: inner_symbol, args, fields, nominal: Some(symbol) }
        }

        Value::Int(_) => value,
    }
}

// Rewrites every `@here` leaf in an arithmetic subtree into a literal
// `Expr::Integer` holding `values_emitted`'s current value, so the
// resulting tree can be folded by `eval::eval` — which has no notion of a
// live expansion in progress — the same way any other literal would be.
fn substitute_here(expr: &Expr, values_emitted: &Int) -> Expr {
    match expr {
        Expr::Here { span } => Expr::Integer { raw: values_emitted.to_string(), span: *span },

        Expr::Unary { op, operand, span } => Expr::Unary {
            op: *op,
            operand: Box::new(substitute_here(operand, values_emitted)),
            span: *span,
        },

        Expr::Binary { left, op, right, span } => Expr::Binary {
            left: Box::new(substitute_here(left, values_emitted)),
            op: *op,
            right: Box::new(substitute_here(right, values_emitted)),
            span: *span,
        },

        _ => expr.clone(),
    }
}

fn int_only_scope(scope: &HashMap<String, Value>) -> HashMap<String, Int> {
    scope
        .iter()
        .filter_map(|(name, value)| match value {
            Value::Int(int) => Some((name.clone(), int.clone())),
            Value::Struct { .. } => None,
        })
        .collect()
}

fn into_value_error(error: EvalError, scope: &HashMap<String, Value>) -> ResolveError {
    match error {
        // If the name is in scope at all, it's a struct value used where an
        // Int was needed (it got filtered out of `int_only_scope`) rather
        // than genuinely unknown.
        EvalError::UnknownConstant { name, span } => {
            if scope.contains_key(&name) {
                ResolveError::ExpectedIntValue { span }
            } else {
                ResolveError::UnknownConstant { name, span }
            }
        }

        EvalError::NotConstant { span } => ResolveError::ExpectedValueExpression { span },
        EvalError::DivisionByZero { span } => ResolveError::DivisionByZero { span },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use std::path::Path;

    use crate::ast::{Expr, Invocation, MacroDeclaration, Program, Statement};
    use crate::eval::Int;
    use crate::lexer;
    use crate::parser;
    use crate::resolver::{collect_symbols, AliasResolver, ResolveError};

    use super::{BuiltinType, ResolvedGenericArg, ResolvedType, Value};

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
                Statement::Macro(decl) if decl.name == name => Some(decl),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected a macro named `{name}`"))
    }

    fn emit_expr(declaration: &MacroDeclaration) -> &Expr {
        let Statement::Meta(meta) = &declaration.body[0] else {
            panic!("expected the macro body's first statement to be a meta statement");
        };

        &meta.args[0]
    }

    fn find_invocations<'a>(program: &'a Program, name: &str) -> Vec<&'a Invocation> {
        program
            .statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Invocation(invocation) if invocation.name == name => Some(invocation),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn evaluates_plain_int_emit() {
        let program = parse_fixture("double.basm");
        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("x".to_string(), Value::Int(Int::from(21)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(value, Value::Int(Int::from(42)));
    }

    // Enum declarations are parsed and registered as symbols, but not
    // wired into `Value`/`ResolvedType` — see `ast::EnumDeclaration`'s doc.
    // A value-position use of a variant should fail with an ordinary
    // resolve error, not panic.
    #[test]
    fn enum_variant_in_value_position_is_a_resolve_error_not_a_panic() {
        let program = parse_fixture("enum_value_position.basm");

        let declaration = find_macro(&program, "use_variant");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let result = resolver.eval_value(emit_expr(declaration), &HashMap::new());

        assert!(result.is_err());
    }

    #[test]
    fn evaluates_struct_emit_with_named_args() {
        let program = parse_fixture("make_reg_named.basm");

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: reg_id,
                args: vec![],
                fields: vec![
                    ("id".to_string(), Value::Int(Int::from(0))),
                    ("width".to_string(), Value::Int(Int::from(2))),
                ],
                nominal: None,
            }
        );
    }

    #[test]
    fn evaluates_struct_emit_with_positional_args() {
        let program = parse_fixture("make_reg_positional.basm");

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: reg_id,
                args: vec![],
                fields: vec![
                    ("id".to_string(), Value::Int(Int::from(0))),
                    ("width".to_string(), Value::Int(Int::from(2))),
                ],
                nominal: None,
            }
        );
    }

    #[test]
    fn evaluates_member_access_on_struct_valued_scope_entry() {
        let program = parse_fixture("read_id.basm");

        let declaration = find_macro(&program, "read_id");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert(
            "dst".to_string(),
            Value::Struct {
                symbol: reg_id,
                args: vec![],
                fields: vec![
                    ("id".to_string(), Value::Int(Int::from(7))),
                    ("width".to_string(), Value::Int(Int::from(2))),
                ],
                nominal: None,
            },
        );

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(value, Value::Int(Int::from(7)));
    }

    #[test]
    fn rejects_unknown_field_name() {
        let program = parse_fixture("unknown_field.basm");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::UnknownField { .. })
        ));
    }

    #[test]
    fn rejects_wrong_arity() {
        let program = parse_fixture("wrong_arity.basm");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::InvalidArgumentCount { expected: 2, actual: 1, .. })
        ));
    }

    #[test]
    fn rejects_field_type_mismatch() {
        let program = parse_fixture("field_type_mismatch.basm");

        let declaration = find_macro(&program, "wrap");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(1)));

        // `A.id` is declared `int`; the body constructs it from `B(...)`, a
        // struct — same mismatch as a wrong-typed macro operand, just at
        // struct-construction time instead of invocation-binding time.
        match resolver.eval_value(emit_expr(declaration), &scope) {
            Err(ResolveError::TypeMismatch { name, expected, actual, .. }) => {
                assert_eq!(name, "id");
                assert_eq!(expected, "int");
                assert_eq!(actual, "B");
            }
            other => panic!("expected a type mismatch error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_struct_callee() {
        let program = parse_fixture("non_struct_callee.basm");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::ExpectedStructCallee { .. })
        ));
    }

    #[test]
    fn resolves_top_level_named_const_as_invocation_operand() {
        let program = parse_fixture("named_const_reference.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let invocations = find_invocations(&program, "read");
        assert_eq!(invocations.len(), 2);

        // `read r0` — a direct reference to a struct-valued top-level
        // const, used as an invocation operand rather than passed down
        // from an enclosing macro's own params.
        let via_r0 = resolver.expand_invocation(invocations[0], &HashMap::new()).unwrap();
        assert_eq!(via_r0.emitted, vec![Value::Int(Int::from(0))]);

        // `read zero` — `zero`'s own value (`r0`) is itself a bare
        // identifier referencing another struct-valued const, so this
        // exercises the same fallback recursively.
        let via_zero = resolver.expand_invocation(invocations[1], &HashMap::new()).unwrap();
        assert_eq!(via_zero.emitted, vec![Value::Int(Int::from(0))]);
    }

    #[test]
    fn brace_construction_with_flat_fields() {
        let program = parse_fixture("construct_flat_fields.basm");

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: reg_id,
                args: vec![],
                fields: vec![
                    ("id".to_string(), Value::Int(Int::from(0))),
                    ("width".to_string(), Value::Int(Int::from(2))),
                ],
                nominal: None,
            }
        );
    }

    #[test]
    fn paren_call_omitting_a_defaulted_field_uses_its_declared_default() {
        let program = parse_fixture("struct_field_default.basm");

        let declaration = find_macro(&program, "make_reg_call_with_default");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(3)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: reg_id,
                args: vec![],
                fields: vec![
                    ("id".to_string(), Value::Int(Int::from(3))),
                    ("width".to_string(), Value::Int(Int::from(8))),
                ],
                nominal: None,
            }
        );
    }

    #[test]
    fn paren_call_supplying_a_defaulted_field_overrides_the_default() {
        let program = parse_fixture("struct_field_default.basm");

        let declaration = find_macro(&program, "make_reg_call_override_default");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(3)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        let Value::Struct { fields, .. } = value else {
            panic!("expected a struct value");
        };

        assert_eq!(fields[1], ("width".to_string(), Value::Int(Int::from(16))));
    }

    #[test]
    fn brace_construction_omitting_a_defaulted_field_uses_its_declared_default() {
        let program = parse_fixture("struct_field_default.basm");

        let declaration = find_macro(&program, "make_reg_construct_with_default");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(5)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        let Value::Struct { fields, .. } = value else {
            panic!("expected a struct value");
        };

        assert_eq!(fields[1], ("width".to_string(), Value::Int(Int::from(8))));
    }

    #[test]
    fn brace_construction_with_generic_callee_and_for_generated_fields() {
        let program = parse_fixture("construct_generic_for.basm");

        let declaration = find_macro(&program, "make_array");
        let symbols = collect_symbols(&program).unwrap();
        let array_id = symbols.lookup("Array").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let value = resolver.eval_value(emit_expr(declaration), &HashMap::new()).unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: array_id,
                args: vec![
                    ResolvedGenericArg::Type(Box::new(ResolvedType::Builtin(BuiltinType::Int))),
                    ResolvedGenericArg::Const(Int::from(3)),
                ],
                fields: vec![
                    ("__el0".to_string(), Value::Int(Int::from(0))),
                    ("__el1".to_string(), Value::Int(Int::from(2))),
                    ("__el2".to_string(), Value::Int(Int::from(4))),
                ],
                nominal: None,
            }
        );
    }

    #[test]
    fn brace_construction_with_nested_for_if_else() {
        let program = parse_fixture("construct_generic_for_if.basm");

        let declaration = find_macro(&program, "make_array");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("index".to_string(), Value::Int(Int::from(1)));
        scope.insert("value".to_string(), Value::Int(Int::from(9)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        let Value::Struct { fields, .. } = value else {
            panic!("expected a struct value");
        };

        assert_eq!(
            fields,
            vec![
                ("__el0".to_string(), Value::Int(Int::from(0))),
                ("__el1".to_string(), Value::Int(Int::from(9))),
                ("__el2".to_string(), Value::Int(Int::from(0))),
            ]
        );
    }

    #[test]
    fn brace_construction_honors_a_passing_struct_invariant() {
        let program = parse_fixture("construct_with_invariant.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(3)));

        assert!(resolver.eval_value(emit_expr(declaration), &scope).is_ok());
    }

    #[test]
    fn brace_construction_rejects_a_failing_struct_invariant() {
        let program = parse_fixture("construct_with_invariant.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(300)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::InvariantViolated { .. })
        ));
    }

    #[test]
    fn paren_call_honors_a_passing_struct_invariant() {
        let program = parse_fixture("call_with_invariant.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(50)));

        assert!(resolver.eval_value(emit_expr(declaration), &scope).is_ok());
    }

    #[test]
    fn string_literal_evaluates_to_its_packed_bytes_as_an_int() {
        let program = parse_fixture("string_literal_value.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let value = resolver.eval_value(emit_expr(declaration), &HashMap::new()).unwrap();

        // "A" is one byte, 0x41 = 65.
        assert_eq!(value, Value::Int(Int::from(65)));
    }

    #[test]
    fn multi_byte_string_literal_packs_big_endian() {
        let program = parse_fixture("string_literal_value.basm");

        let declaration = find_macro(&program, "make_multi");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let value = resolver.eval_value(emit_expr(declaration), &HashMap::new()).unwrap();

        // "AB" is bytes [0x41, 0x42] big-endian == 0x4142 == 16706.
        assert_eq!(value, Value::Int(Int::from(16706)));
    }

    #[test]
    fn paren_call_rejects_a_failing_struct_invariant() {
        let program = parse_fixture("call_with_invariant.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(-1)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::InvariantViolated { .. })
        ));
    }

    #[test]
    fn as_conversion_wraps_and_tags_a_passing_value() {
        let program = parse_fixture("as_conversion.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let ubyte_id = symbols.lookup("UByte").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(50)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: bits_id,
                args: vec![ResolvedGenericArg::Const(Int::from(8))],
                fields: vec![("value".to_string(), Value::Int(Int::from(50)))],
                nominal: Some(ubyte_id),
            }
        );
    }

    #[test]
    fn as_conversion_rejects_a_value_failing_the_aliass_own_invariant() {
        let program = parse_fixture("as_conversion.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        // 150 fits inside `Bits<8>` (< 256) but fails `UByte`'s own,
        // stricter `value < 100` — must be caught at the alias layer, not
        // silently accepted because the underlying struct is happy.
        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(150)));

        match resolver.eval_value(emit_expr(declaration), &scope) {
            Err(ResolveError::InvariantViolated { type_name, .. }) => {
                assert_eq!(type_name, "UByte");
            }
            other => panic!("expected an InvariantViolated on UByte, got {other:?}"),
        }
    }

    #[test]
    fn as_conversion_rejects_a_value_failing_the_underlying_structs_invariant() {
        let program = parse_fixture("as_conversion.basm");

        let declaration = find_macro(&program, "make");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        // -5 satisfies `UByte`'s own `value < 100` (checked first, at the
        // alias layer) but not `Bits`'s `value >= 0` — caught after
        // auto-wrapping, by the terminal struct's own invariant.
        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(-5)));

        match resolver.eval_value(emit_expr(declaration), &scope) {
            Err(ResolveError::InvariantViolated { type_name, .. }) => {
                assert_eq!(type_name, "Bits");
            }
            other => panic!("expected an InvariantViolated on Bits, got {other:?}"),
        }
    }

    #[test]
    fn a_value_bypassing_as_does_not_satisfy_a_nominal_parameter() {
        let program = parse_fixture("as_conversion.basm");

        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        // A plain, untagged `Bits<8>` value that would satisfy `UByte`'s
        // own invariant structurally (50 < 100) still can't be handed to a
        // `UByte`-typed parameter directly — only `@as` (or a checked
        // `const`) may produce a value that type-checks as `UByte`.
        let untagged = Value::Struct {
            symbol: bits_id,
            args: vec![ResolvedGenericArg::Const(Int::from(8))],
            fields: vec![("value".to_string(), Value::Int(Int::from(50)))],
            nominal: None,
        };

        let take_symbol = symbols.lookup("take").unwrap();
        let take_declaration = find_macro(&program, "take");

        let mut stack = Vec::new();

        assert!(matches!(
            resolver.run_macro_body(take_symbol, take_declaration, vec![untagged], &mut stack),
            Err(ResolveError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn checked_const_wraps_and_tags_its_declared_value() {
        let program = parse_fixture("checked_const.basm");
        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let ubyte_id = symbols.lookup("UByte").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let value = resolver
            .resolve_const_value("FIVE", crate::token::Span::new(0, 0))
            .unwrap();

        assert_eq!(
            value,
            Value::Struct {
                symbol: bits_id,
                args: vec![ResolvedGenericArg::Const(Int::from(8))],
                fields: vec![("value".to_string(), Value::Int(Int::from(5)))],
                nominal: Some(ubyte_id),
            }
        );
    }

    #[test]
    fn checked_const_is_usable_as_an_invocation_operand_with_no_as_at_the_call_site() {
        let program = parse_fixture("checked_const.basm");
        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let ubyte_id = symbols.lookup("UByte").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let invocations = find_invocations(&program, "use_five");
        assert_eq!(invocations.len(), 1);

        let expansion = resolver.expand_invocation(invocations[0], &HashMap::new()).unwrap();

        assert_eq!(
            expansion.emitted,
            vec![Value::Struct {
                symbol: bits_id,
                args: vec![ResolvedGenericArg::Const(Int::from(8))],
                fields: vec![("value".to_string(), Value::Int(Int::from(5)))],
                nominal: Some(ubyte_id),
            }]
        );
    }

    #[test]
    fn checked_const_rejects_a_declared_value_failing_its_types_invariant() {
        let program = parse_fixture("checked_const_out_of_range.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        match resolver.resolve_const_value("TOO_BIG", crate::token::Span::new(0, 0)) {
            Err(ResolveError::InvariantViolated { type_name, .. }) => {
                assert_eq!(type_name, "UByte");
            }
            other => panic!("expected an InvariantViolated on UByte, got {other:?}"),
        }
    }
}
