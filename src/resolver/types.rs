//! The resolved counterpart of [`crate::types::TypeExpr`]: a type expression
//! after its name has been looked up in the [`super::SymbolTable`] and
//! confirmed to actually refer to a struct, a builtin, or a generic
//! parameter in scope, rather than just a path of identifiers.

use crate::ast::Expr;

use super::symbols::SymbolId;

/// A type after name resolution. Unlike [`crate::types::TypeExpr::Apply`],
/// generic const arguments here are still unevaluated [`Expr`] trees rather
/// than folded values — so, for instance, `bits<4 + 4>` and `bits<8>`
/// resolve to distinct [`ResolvedType::Struct`] values even though they
/// denote the same type. Const folding isn't implemented yet.
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedGenericArg {
    Type(Box<ResolvedType>),
    Const(Expr),
}