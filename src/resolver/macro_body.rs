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

use crate::ast::{CallArgument, ConstDeclaration, Expr, Invocation, MacroDeclaration, Statement};
use crate::token::Span;

use super::aliases::AliasResolver;
use super::structs::describe_type;
use super::symbols::SymbolId;
use super::values::{value_type, Value};
use super::ResolveError;

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
            let actual = value_type(&value);

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
                        [expr] => emitted.push(self.eval_value(expr, &scope)?),

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
                    generated.push(Statement::Const(self.splice_const(decl, &scope)?));
                }

                Statement::Const(decl) => {
                    let value = self.eval_value(&decl.value, &scope)?;
                    scope.insert(decl.name.clone(), value);
                }

                Statement::Struct(_)
                | Statement::TypeAlias(_)
                | Statement::Macro(_)
                | Statement::Label(_) => {
                    // Their own name/body can't contain a splice yet (see
                    // the module doc), so there's nothing to rewrite —
                    // captured verbatim, to be spliced into the program
                    // wherever this expansion's call site was.
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

    fn splice_const(
        &mut self,
        decl: &ConstDeclaration,
        scope: &HashMap<String, Value>,
    ) -> Result<ConstDeclaration, ResolveError> {
        Ok(ConstDeclaration {
            name: decl.name.clone(),
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

            Expr::Identifier { .. } | Expr::Integer { .. } | Expr::String { .. } => {
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
        }
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
            .map(|id| self.symbols.get(*id).name.clone())
            .collect();

        cycle.push(self.symbols.get(repeated).name.clone());

        ResolveError::CyclicMacroExpansion {
            cycle,
            span: self.symbols.get(repeated).span,
        }
    }
}

// A splice's evaluated `Value` reified back into source-shaped `Expr`, for
// a generated declaration's rewritten `value` — see
// `AliasResolver::splice_expr`.
fn reify_value(value: &Value, span: Span) -> Result<Expr, ResolveError> {
    match value {
        Value::Int(int) => Ok(Expr::Integer { raw: int.to_string(), span }),
        Value::Struct { .. } => Err(ResolveError::UnsupportedSpliceValue { span }),
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
    use crate::resolver::{collect_symbols, AliasResolver, ResolveError, ResolvedGenericArg};

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
    fn struct_via_generic_alias_carries_resolved_generic_args() {
        let program = parse_fixture("generic_alias.basm");

        let declaration = find_macro(&program, "make_byte");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_byte").unwrap();
        let bits_id = symbols.lookup("bits").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
    fn pub_const_is_generated_with_splices_evaluated() {
        let program = parse_fixture("generated_pub_const.basm");

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(7))], &mut stack)
            .unwrap();

        assert_eq!(result.generated.len(), 1);

        let Statement::Const(decl) = &result.generated[0] else {
            panic!("expected a generated const declaration");
        };

        assert_eq!(decl.name, "the_reg");
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
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
    }

    #[test]
    fn generated_declarations_bubble_up_through_nested_invocations() {
        let program = parse_fixture("nested_invocation_generates.basm");

        let declaration = find_macro(&program, "outer");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("outer").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let result = resolver
            .run_macro_body(symbol, declaration, vec![Value::Int(Int::from(9))], &mut stack)
            .unwrap();

        assert_eq!(result.generated.len(), 1);

        let Statement::Const(decl) = &result.generated[0] else {
            panic!("expected a generated const declaration");
        };

        assert_eq!(decl.name, "the_const");
        assert!(matches!(&decl.value, Expr::Integer { raw, .. } if raw == "9"));
    }

    #[test]
    fn generated_const_leaves_non_spliced_identifiers_alone() {
        let program = parse_fixture("generated_const_leaves_bare_identifiers.basm");

        let declaration = find_macro(&program, "make_ref");
        let symbols = collect_symbols(&program).unwrap();
        let symbol = symbols.lookup("make_ref").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
}
