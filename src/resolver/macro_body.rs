//! Walks a single [`MacroDeclaration`]'s body, given already-bound
//! [`Value`] arguments for its declared params, collecting `@emit`'s values
//! in program order and stopping early on `@return` — mirroring how
//! `@return` is meant to splice a value in and exit, except `@emit` never
//! splices anything, it only appends to the output stream.
//!
//! This does NOT expand a real invocation (`mov r1, 7`) end to end — that
//! needs macro-to-invocation binding and operand type checking, neither of
//! which exist yet. This is the piece underneath that: given a macro
//! declaration and concrete argument values, what running its body
//! produces.

use std::collections::HashMap;

use crate::ast::{MacroDeclaration, Statement};
use crate::token::Span;

use super::aliases::AliasResolver;
use super::values::Value;
use super::ResolveError;

#[derive(Debug, Clone, PartialEq)]
pub struct MacroExpansion {
    pub emitted: Vec<Value>,
    pub returned: Option<Value>,
}

impl<'a> AliasResolver<'a> {
    pub fn run_macro_body(
        &mut self,
        declaration: &MacroDeclaration,
        arguments: Vec<Value>,
    ) -> Result<MacroExpansion, ResolveError> {
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

        let mut emitted = Vec::new();

        for statement in &declaration.body {
            // Expansion of anything other than a meta statement (a nested
            // invocation, a pasted plain statement) needs macro-to-invocation
            // binding this crate doesn't have yet — error instead of
            // silently dropping it, since a silently incomplete expansion
            // would be worse than a clear "not supported yet".
            let Statement::Meta(meta) = statement else {
                let (kind, span) = describe_statement(statement);

                return Err(ResolveError::UnsupportedMacroStatement {
                    kind: kind.to_string(),
                    span,
                });
            };

            match meta.name.as_str() {
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

                    return Ok(MacroExpansion { emitted, returned: value });
                }

                other => {
                    return Err(ResolveError::UnsupportedMacroStatement {
                        kind: format!("@{other}"),
                        span: meta.span,
                    })
                }
            }
        }

        Ok(MacroExpansion { emitted, returned: None })
    }
}

fn describe_statement(statement: &Statement) -> (&'static str, Span) {
    match statement {
        Statement::Import(s) => ("import", s.span),
        Statement::Struct(s) => ("struct", s.span),
        Statement::TypeAlias(s) => ("type alias", s.span),
        Statement::Const(s) => ("const", s.span),
        Statement::Label(s) => ("label", s.span),
        Statement::Invocation(s) => ("invocation", s.span),
        Statement::Macro(s) => ("nested macro", s.span),
        Statement::Meta(_) => unreachable!("Meta statements are handled before this is called"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use std::path::Path;

    use crate::ast::{MacroDeclaration, Program, Statement};
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

    #[test]
    fn emit_and_return_combine_and_return_stops_the_body() {
        let program = parse_fixture("combo.basm");

        let declaration = find_macro(&program, "combo");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let result = resolver
            .run_macro_body(declaration, vec![Value::Int(Int::from(5))])
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
        let bits_id = symbols.lookup("bits").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let result = resolver
            .run_macro_body(declaration, vec![Value::Int(Int::from(3))])
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
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        assert!(matches!(
            resolver.run_macro_body(declaration, vec![Value::Int(Int::from(1))]),
            Err(ResolveError::UnsupportedMacroStatement { .. })
        ));
    }
}
