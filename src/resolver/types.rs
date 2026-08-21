//! The resolved counterpart of [`crate::types::TypeExpr`]: a type expression
//! after its name has been looked up in the [`super::SymbolTable`] and
//! confirmed to actually refer to a struct, a builtin, or a generic
//! parameter in scope, rather than just a path of identifiers.

use crate::eval::Int;

use super::symbols::SymbolId;

/// A type after name resolution. Generic const arguments are folded to a
/// concrete [`Int`] wherever a concrete value is available, so `bits<4+4>`
/// and `bits<8>` now resolve to the same [`ResolvedType::Struct`] value —
/// unlike a const arg that's still an unbound parameter (e.g. resolving
/// `Reg`'s own field types, where `bits<width>`'s `width` has no concrete
/// value yet), which stays symbolic via [`ResolvedGenericArg::ConstParam`],
/// mirroring how [`ResolvedType::TypeParameter`] handles the same problem
/// for type arguments.
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
    Const(Int),
    ConstParam(String),
}