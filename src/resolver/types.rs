use crate::ast::Expr;

use super::symbols::SymbolId;

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    Builtin(BuiltinType),
    Struct {
        symbol: SymbolId,
        args: Vec<ResolvedGenericArg>,
    },
    TypeParameter {
        name: String
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinType {
    Int,
    Uint,
    Bits {
        width: BitsWidth,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BitsWidth {
    Literal(u64),
    Parameter(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedGenericArg {
    Type(Box<ResolvedType>),
    Const(Expr),
}