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

use crate::ast::{CallArgument, Expr};
use crate::eval::{self, EvalError, Int};
use crate::token::Span;

use super::aliases::AliasResolver;
use super::symbols::SymbolId;
use super::types::{ResolvedGenericArg, ResolvedType};
use super::ResolveError;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Int),
    Struct {
        symbol: SymbolId,
        args: Vec<ResolvedGenericArg>,
        fields: Vec<(String, Value)>,
    },
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
            Expr::Integer { .. } | Expr::Unary { .. } | Expr::Binary { .. } => {
                let int_scope = int_only_scope(scope);

                eval::eval(expr, &int_scope)
                    .map(Value::Int)
                    .map_err(|error| into_value_error(error, scope))
            }

            Expr::Identifier { name, span } => scope.get(name).cloned().ok_or_else(|| {
                ResolveError::UnknownConstant {
                    name: name.clone(),
                    span: *span,
                }
            }),

            Expr::Member { object, member, span } => match self.eval_value(object, scope)? {
                Value::Struct { symbol, fields, .. } => fields
                    .into_iter()
                    .find(|(name, _)| name == member)
                    .map(|(_, value)| value)
                    .ok_or_else(|| ResolveError::UnknownField {
                        type_name: self.symbols.get(symbol).name.clone(),
                        field: member.clone(),
                        span: *span,
                    }),

                Value::Int(_) => Err(ResolveError::ExpectedStructValue { span: *span }),
            },

            Expr::Call { callee, arguments, span } => {
                self.eval_call_value(callee, arguments, *span, scope)
            }

            Expr::String { span, .. } => Err(ResolveError::ExpectedValueExpression { span: *span }),
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

        let ResolvedType::Struct { symbol, args } = resolved else {
            return Err(ResolveError::ExpectedStructCallee {
                name: name.clone(),
                span: *callee_span,
            });
        };

        let field_names: Vec<String> = self
            .find_struct_declaration(symbol)?
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect();

        if arguments.len() != field_names.len() {
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

        let mut fields = Vec::with_capacity(field_names.len());

        for field_name in field_names {
            let value = by_name.remove(&field_name).ok_or_else(|| ResolveError::Internal {
                message: format!(
                    "struct call to `{name}` had the right argument count but field \
                     `{field_name}` was never targeted — likely two arguments named the same field",
                ),
                span,
            })?;

            fields.push((field_name, value));
        }

        Ok(Value::Struct { symbol, args, fields })
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

    use crate::ast::{Expr, MacroDeclaration, Program, Statement};
    use crate::eval::Int;
    use crate::lexer;
    use crate::parser;
    use crate::resolver::{collect_symbols, AliasResolver, ResolveError};

    use super::Value;

    fn parse_program(source: &str) -> Program {
        let mut full = source.to_string();
        full.push('\n');

        let tokens = lexer::lex(&full).expect("source should lex");
        parser::parse(tokens).expect("source should parse")
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

    #[test]
    fn evaluates_plain_int_emit() {
        let program = parse_program("macro double(x: int) {\n    @emit x * 2\n}");
        let declaration = find_macro(&program, "double");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("x".to_string(), Value::Int(Int::from(21)));

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(value, Value::Int(Int::from(42)));
    }

    #[test]
    fn evaluates_struct_emit_with_named_args() {
        let program = parse_program(
            "struct Reg {\n    id: int,\n    width: int\n}\n\n\
             macro make_reg(v: int) {\n    @emit Reg(id=0, width=v)\n}",
        );

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
            }
        );
    }

    #[test]
    fn evaluates_struct_emit_with_positional_args() {
        let program = parse_program(
            "struct Reg {\n    id: int,\n    width: int\n}\n\n\
             macro make_reg(v: int) {\n    @emit Reg(0, v)\n}",
        );

        let declaration = find_macro(&program, "make_reg");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
            }
        );
    }

    #[test]
    fn evaluates_member_access_on_struct_valued_scope_entry() {
        let program = parse_program(
            "struct Reg {\n    id: int,\n    width: int\n}\n\n\
             macro read_id(dst: Reg) {\n    @emit dst.id\n}",
        );

        let declaration = find_macro(&program, "read_id");
        let symbols = collect_symbols(&program).unwrap();
        let reg_id = symbols.lookup("Reg").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

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
            },
        );

        let value = resolver.eval_value(emit_expr(declaration), &scope).unwrap();

        assert_eq!(value, Value::Int(Int::from(7)));
    }

    #[test]
    fn rejects_unknown_field_name() {
        let program = parse_program(
            "struct Reg {\n    id: int,\n    width: int\n}\n\n\
             macro bad(v: int) {\n    @emit Reg(idx=0, width=v)\n}",
        );

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::UnknownField { .. })
        ));
    }

    #[test]
    fn rejects_wrong_arity() {
        let program = parse_program(
            "struct Reg {\n    id: int,\n    width: int\n}\n\n\
             macro bad(v: int) {\n    @emit Reg(v)\n}",
        );

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::InvalidArgumentCount { expected: 2, actual: 1, .. })
        ));
    }

    #[test]
    fn rejects_non_struct_callee() {
        let program = parse_program("macro bad(v: int) {\n    @emit int(v)\n}");

        let declaration = find_macro(&program, "bad");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut scope = HashMap::new();
        scope.insert("v".to_string(), Value::Int(Int::from(2)));

        assert!(matches!(
            resolver.eval_value(emit_expr(declaration), &scope),
            Err(ResolveError::ExpectedStructCallee { .. })
        ));
    }
}
