//! Reifies a resolved [`Value`] into [`EmittedValue`] — a self-contained,
//! serializable shape with no dependency on any in-process state, unlike
//! `Value` itself. This is what `bitterasm compile` writes to a `.em` file
//! and what `bitter` is meant to read back.
//!
//! The one thing that actually needs rewriting is `Value::Struct`'s
//! [`SymbolId`] — it only means anything against the [`SymbolTable`] that
//! produced it, so [`reify_value`] resolves it to the struct's own name
//! once, here, rather than asking every later reader to carry a
//! `SymbolTable` around just to make sense of an id.

use serde::{Deserialize, Serialize};

use crate::resolver::{BuiltinType, ResolvedGenericArg, ResolvedType, SymbolTable, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EmittedValue {
    // An arbitrary-precision `Int`'s decimal string, not a JSON number —
    // JSON numbers are typically read back as f64, which can't represent
    // every value an Int can.
    Int { value: String },

    Struct {
        name: String,
        args: Vec<EmittedGenericArg>,
        fields: Vec<(String, EmittedValue)>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EmittedGenericArg {
    Const { value: String },
    Type(EmittedType),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EmittedType {
    Builtin { name: String },
    Struct { name: String, args: Vec<EmittedGenericArg> },
}

pub fn reify_value(symbols: &SymbolTable, value: &Value) -> EmittedValue {
    match value {
        Value::Int(int) => EmittedValue::Int { value: int.to_string() },

        Value::Struct { symbol, args, fields } => EmittedValue::Struct {
            name: symbols.get(*symbol).name.clone(),
            args: args.iter().map(|arg| reify_generic_arg(symbols, arg)).collect(),
            fields: fields
                .iter()
                .map(|(name, value)| (name.clone(), reify_value(symbols, value)))
                .collect(),
        },
    }
}

fn reify_generic_arg(symbols: &SymbolTable, arg: &ResolvedGenericArg) -> EmittedGenericArg {
    match arg {
        ResolvedGenericArg::Const(int) => EmittedGenericArg::Const { value: int.to_string() },

        ResolvedGenericArg::Type(ty) => EmittedGenericArg::Type(reify_type(symbols, ty)),

        // A concrete Value only ever comes from naming a fully-resolved
        // type at a call site (`eval_call_value` in resolver/values.rs) —
        // there's no path that leaves one of its own generic args pointing
        // back at an outer, not-yet-instantiated const param.
        ResolvedGenericArg::ConstParam(name) => unreachable!(
            "a fully-evaluated value's generic args can't carry an unresolved \
             const param (`{name}`)"
        ),
    }
}

fn reify_type(symbols: &SymbolTable, ty: &ResolvedType) -> EmittedType {
    match ty {
        ResolvedType::Builtin(BuiltinType::Int) => EmittedType::Builtin { name: "int".to_string() },

        ResolvedType::Struct { symbol, args } => EmittedType::Struct {
            name: symbols.get(*symbol).name.clone(),
            args: args.iter().map(|arg| reify_generic_arg(symbols, arg)).collect(),
        },

        // Same reasoning as `ConstParam` above, one level up: a concrete
        // value's resolved type is never left in terms of an
        // uninstantiated type parameter.
        ResolvedType::TypeParameter { name } => unreachable!(
            "a fully-evaluated value's generic args can't carry an unresolved \
             type parameter (`{name}`)"
        ),
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
    use crate::resolver::{collect_symbols, AliasResolver};
    use crate::token::Span;

    use super::*;

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
    fn reifies_an_int() {
        let symbols =
            collect_symbols(&Program { statements: vec![], span: Span::new(0, 0) }).unwrap();

        assert_eq!(
            reify_value(&symbols, &Value::Int(Int::from(42))),
            EmittedValue::Int { value: "42".to_string() },
        );
    }

    #[test]
    fn reifies_a_struct_with_resolved_generic_args_by_name() {
        let program = parse_fixture("generic_alias.basm");

        let declaration = find_macro(&program, "make_byte");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let expansion = resolver
            .run_macro_body(
                symbols.lookup("make_byte").unwrap(),
                declaration,
                vec![Value::Int(Int::from(3))],
                &mut stack,
            )
            .unwrap();

        let emitted = reify_value(&symbols, &expansion.emitted[0]);

        assert_eq!(
            emitted,
            EmittedValue::Struct {
                name: "bits".to_string(),
                args: vec![EmittedGenericArg::Const { value: "8".to_string() }],
                fields: vec![("value".to_string(), EmittedValue::Int { value: "3".to_string() })],
            }
        );
    }
}
