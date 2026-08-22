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
//! What's still out of scope here: operand type checking (an invocation's
//! operands aren't checked against the macro's declared param types yet),
//! and non-default invocation syntax (a macro always binds via the plain
//! `name arg, arg, ...` form for now).

use std::collections::HashMap;

use crate::ast::{Invocation, MacroDeclaration, Statement};
use crate::token::Span;

use super::aliases::AliasResolver;
use super::symbols::SymbolId;
use super::values::Value;
use super::ResolveError;

#[derive(Debug, Clone, PartialEq)]
pub struct MacroExpansion {
    pub emitted: Vec<Value>,
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
        scope: &HashMap<String, Value>,
        stack: &mut Vec<SymbolId>,
    ) -> Result<MacroExpansion, ResolveError> {
        let mut emitted = Vec::new();

        for statement in body {
            match statement {
                Statement::Meta(meta) => match meta.name.as_str() {
                    "emit" => match meta.args.as_slice() {
                        [expr] => emitted.push(self.eval_value(expr, scope)?),

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
                            [expr] => Some(self.eval_value(expr, scope)?),

                            other => {
                                return Err(ResolveError::InvalidArgumentCount {
                                    name: "@return".to_string(),
                                    expected: 1,
                                    actual: other.len(),
                                    span: meta.span,
                                })
                            }
                        };

                        return Ok(MacroExpansion { emitted, returned: value });
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
                    // a usable result elsewhere — so only its emitted
                    // values are spliced into ours, in order; `returned` is
                    // discarded.
                    let nested = self.expand_invocation_inner(invocation, scope, stack)?;
                    emitted.extend(nested.emitted);
                }

                other => {
                    let (kind, span) = describe_statement(other);

                    return Err(ResolveError::UnsupportedMacroStatement {
                        kind: kind.to_string(),
                        span,
                    });
                }
            }
        }

        Ok(MacroExpansion { emitted, returned: None })
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

fn describe_statement(statement: &Statement) -> (&'static str, Span) {
    match statement {
        Statement::Import(s) => ("import", s.span),
        Statement::Struct(s) => ("struct", s.span),
        Statement::TypeAlias(s) => ("type alias", s.span),
        Statement::Const(s) => ("const", s.span),
        Statement::Label(s) => ("label", s.span),
        Statement::Macro(s) => ("nested macro", s.span),
        Statement::Invocation(_) | Statement::Meta(_) => {
            unreachable!("Invocation and Meta statements are handled before this is called")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use std::path::Path;

    use crate::ast::{Invocation, MacroDeclaration, Program, Statement};
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
                returned: None,
            }
        );
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
}
