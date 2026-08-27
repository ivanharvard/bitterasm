//! The resolved counterpart of [`crate::types::TypeExpr`]: a type expression
//! after its name has been looked up in the [`super::SymbolTable`] and
//! confirmed to actually refer to a struct, a builtin, or a generic
//! parameter in scope, rather than just a path of identifiers.

use crate::ast::Expr;
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
    Enum {
        symbol: SymbolId,
        args: Vec<ResolvedGenericArg>,
    },
    TypeParameter {
        name: String
    },

    /// A `type` alias that declares its own `invariant` facet(s), or whose
    /// target itself resolved to `Alias` (chained — see
    /// `crate::facets::invariant`'s module doc, "holds all the way down").
    /// A *plain* alias with no invariant anywhere in its chain stays fully
    /// transparent, exactly as before this variant existed — resolving it
    /// returns `underlying` directly, not `Alias` — so this only exists (and
    /// only changes type-equality behavior) where an invariant genuinely
    /// needs enforcing. `binder` is the alias's own chosen name for "the
    /// value being converted" (`None` if `invariants` is empty — a
    /// pointless-but-harmless invariant-free alias never reaches this
    /// variant in practice, since it'd have been left transparent, but the
    /// shape stays sound either way).
    Alias {
        symbol: SymbolId,
        binder: Option<String>,
        invariants: Vec<Expr>,
        underlying: Box<ResolvedType>,
    },
}

impl ResolvedType {
    /// Whether this type, when used as a macro signature, accepts `actual`.
    /// Naming a generic struct/enum without arguments is a wildcard over its
    /// specializations; explicit arguments continue to require exact equality.
    pub fn accepts(&self, actual: &ResolvedType) -> bool {
        if self == actual {
            return true;
        }
        match (self.strip_alias(), actual.strip_alias()) {
            (
                ResolvedType::Struct { symbol: expected, args: expected_args },
                ResolvedType::Struct { symbol: found, args: actual_args },
            )
            | (
                ResolvedType::Enum { symbol: expected, args: expected_args },
                ResolvedType::Enum { symbol: found, args: actual_args },
            ) => expected == found && generic_args_accept(expected_args, actual_args),
            _ => false,
        }
    }

    /// Unwraps through every `Alias` layer down to the underlying
    /// `Builtin`/`Struct`/`TypeParameter` — for the (common) call sites that
    /// only care about a type's *structure* (field lookup, diagnostics'
    /// "which struct is this", generic-argument application) and
    /// deliberately don't care about nominal identity, unlike the handful
    /// of `==`/`!=` type-equality checks that do.
    pub fn strip_alias(&self) -> &ResolvedType {
        match self {
            ResolvedType::Alias { underlying, .. } => underlying.strip_alias(),
            other => other,
        }
    }
}

fn generic_args_accept(expected: &[ResolvedGenericArg], actual: &[ResolvedGenericArg]) -> bool {
    expected.is_empty()
        || (expected.len() == actual.len()
            && expected.iter().zip(actual).all(|(expected, actual)| {
                matches!(expected, ResolvedGenericArg::Wildcard) || expected == actual
            }))
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
    Wildcard,
}
