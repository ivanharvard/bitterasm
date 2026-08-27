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
//! scope the generated declaration ends up in. A struct/macro/type alias's
//! own *name* can't yet contain a splice (`` `name` `` as a declaration name
//! doesn't parse — declaration names are still a plain `String` in the AST,
//! not an `Expr`), so those three kinds are captured verbatim with no
//! rewriting at all; only `Const`'s single `value: Expr` gets this
//! treatment today.
//!
//! `Statement::Import` is a hard, permanent error rather than an
//! unimplemented one: imports are resolved by `crate::loader` before the
//! resolver ever runs, against the importing *file's* path — a macro body
//! has no file of its own for a relative import to resolve against, and by
//! the time macro expansion happens the whole program is already flattened.

use std::collections::HashMap;

use crate::ast::{
    CallArgument, ConstDeclaration, ConstructItem, Expr, Invocation, MacroDeclaration, NamePart,
    Statement,
};
use crate::eval::Int;
use crate::token::Span;
use crate::types::TypeArgument;

use super::aliases::AliasResolver;
use super::structs::describe_type;
use super::symbols::SymbolId;
use super::values::Value;
use super::ResolveError;

/// Safety cap on `@for`'s iteration count — `Int` is arbitrary-precision,
/// so an unbounded or accidentally-huge range (`@for i in 0..N` with a
/// mis-set `N`) would otherwise hang rather than fail fast. Shared by
/// every `@for` unroller (a struct body's, in `structs.rs`; a top-level
/// one, in `toplevel.rs`), not just this one.
pub(super) const MAX_FOR_ITERATIONS: u64 = 1_000_000;

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
    /// own fresh recursion-guard stack; nested invocations found while
    /// walking a macro body go through [`Self::expand_invocation_inner`]
    /// instead, sharing the enclosing expansion's stack.
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
        let symbol = self.find_macro_symbol(&invocation.name, invocation.span)?;

        // find_macro_declaration borrows from self; clone immediately so
        // the borrow ends before the &mut self calls below (operand eval,
        // nested expansion) — same idiom as resolve_struct_fields in
        // structs.rs cloning declaration.generic_params before recursing.
        let declaration = self.find_macro_declaration(symbol)?.clone();

        let arguments = invocation
            .operands
            .iter()
            .map(|operand| self.eval_value(operand, scope))
            .collect::<Result<Vec<_>, _>>()?;

        self.run_macro_body(symbol, &declaration, arguments, stack)
    }

    /// Runs `declaration`'s body given already-bound `arguments`. `symbol`
    /// is `declaration`'s own id and `stack` is the shared recursion guard —
    /// every expansion, top-level or nested, funnels through here, so the
    /// cycle check lives in exactly one place. Self-recursion is caught at
    /// depth 1 (pushed before the body runs); mutual recursion (`A` calls
    /// `B` calls `A`) at depth 2 — both bounded, no stack-overflow risk.
    pub fn run_macro_body(
        &mut self,
        symbol: SymbolId,
        declaration: &MacroDeclaration,
        arguments: Vec<Value>,
        stack: &mut Vec<SymbolId>,
    ) -> Result<MacroExpansion, ResolveError> {
        if stack.contains(&symbol) {
            return Err(self.make_macro_cycle_error(stack, symbol));
        }

        if arguments.len() != declaration.params.len() {
            return Err(ResolveError::InvalidArgumentCount {
                name: declaration.name.clone(),
                expected: declaration.params.len(),
                actual: arguments.len(),
                span: declaration.span,
            });
        }

        let mut scope: HashMap<String, Value> = HashMap::new();

        for (param, value) in declaration.params.iter().zip(arguments) {
            let expected = self.resolve_type_expr(&param.ty)?;
            let actual = self.value_type(&value)?;

            if actual != expected {
                return Err(ResolveError::TypeMismatch {
                    name: param.name.clone(),
                    expected: describe_type(&expected, self.symbols),
                    actual: describe_type(&actual, self.symbols),
                    span: param.span,
                });
            }

            scope.insert(param.name.clone(), value);
        }

        stack.push(symbol);
        let result = self.walk_macro_body(&declaration.body, &scope, stack);
        stack.pop();

        result
    }

    fn walk_macro_body(
        &mut self,
        body: &[Statement],
        initial_scope: &HashMap<String, Value>,
        stack: &mut Vec<SymbolId>,
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
                            let nested = self.walk_macro_body(chosen_body, &scope, stack)?;
                            emitted.extend(nested.emitted);
                            generated.extend(nested.generated);

                            if nested.returned.is_some() {
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
                            let nested = self.walk_macro_body(chosen_body, &arm_scope, stack)?;
                            emitted.extend(nested.emitted);
                            generated.extend(nested.generated);
                            if nested.returned.is_some() {
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

                            let nested = self.walk_macro_body(for_body, &iter_scope, stack)?;
                            emitted.extend(nested.emitted);
                            generated.extend(nested.generated);

                            if nested.returned.is_some() {
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
                    let nested = self.expand_invocation_inner(invocation, &scope, stack)?;
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

                Statement::Struct(_)
                | Statement::Enum(_)
                | Statement::TypeAlias(_)
                | Statement::Macro(_)
                | Statement::Label(_) => {
                    // Their own name/body can't contain a splice yet (see
                    // the module doc), so there's nothing to rewrite —
                    // captured verbatim, to be spliced into the program
                    // wherever this expansion's call site was.
                    self.register_generated(statement)?;
                    generated.push(statement.clone());
                }

                Statement::Import(import) => {
                    return Err(ResolveError::UnsupportedMacroStatement {
                        kind: "import".to_string(),
                        span: import.span,
                    });
                }
            }
        }

        Ok(MacroExpansion { emitted, generated, returned: None })
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

    // Mirrors AliasResolver::make_cycle_error / ConstEvaluator::make_cycle_error's
    // stack-slicing shape, but takes `stack` as a parameter rather than
    // reading a `self` field — macro recursion tracking is scoped to one
    // top-level expansion, not memoized on the resolver like alias/const
    // cycle state (the same macro is legitimately invoked many times with
    // different arguments in one program, so nothing here is cached).
    fn make_macro_cycle_error(&self, stack: &[SymbolId], repeated: SymbolId) -> ResolveError {
        let start = stack.iter().position(|id| *id == repeated).unwrap_or(0);

        let mut cycle: Vec<String> = stack[start..]
            .iter()
            .map(|id| self.get_symbol(*id).name.clone())
            .collect();

        cycle.push(self.get_symbol(repeated).name.clone());

        ResolveError::CyclicMacroExpansion {
            cycle,
            span: self.get_symbol(repeated).span,
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
                Statement::Macro(decl) if decl.name == name => Some(decl),
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
    fn rejects_direct_self_recursion() {
        let program = parse_fixture("self_recursive.basm");

        let declaration = find_macro(&program, "loopy");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("loopy").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack);

        match result {
            Err(ResolveError::CyclicMacroExpansion { cycle, .. }) => {
                assert_eq!(cycle, vec!["loopy".to_string(), "loopy".to_string()]);
            }
            other => panic!("expected a cyclic macro expansion error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_mutual_recursion() {
        let program = parse_fixture("mutual_recursion.basm");

        let declaration = find_macro(&program, "ping");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("ping").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver.run_macro_body(symbol, declaration, vec![Value::Int(Int::from(1))], &mut stack);

        match result {
            Err(ResolveError::CyclicMacroExpansion { cycle, .. }) => {
                assert_eq!(
                    cycle,
                    vec!["ping".to_string(), "pong".to_string(), "ping".to_string()]
                );
            }
            other => panic!("expected a cyclic macro expansion error, got {other:?}"),
        }
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

        assert!(matches!(&result.generated[0], Statement::Struct(decl) if decl.name == "Foo"));
        assert!(matches!(&result.generated[1], Statement::TypeAlias(decl) if decl.name == "Bar"));
        assert!(matches!(&result.generated[2], Statement::Macro(decl) if decl.name == "helper"));
        assert!(matches!(&result.generated[3], Statement::Label(label) if label.name == "start"));

        // Top-level-only scoping: `collect_symbols` never descends into a
        // macro body, so this nested `start:` — captured verbatim into
        // `generated` above, still unresolved — was never registered as a
        // `SymbolKind::Label` and stays completely uninvolved in label
        // resolution.
        assert_eq!(symbols.lookup("start"), None);
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
