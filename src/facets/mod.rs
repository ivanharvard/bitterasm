//! Facet-specific logic and metadata live in each facet's own file
//! ([`invariant`], [`before`], [`after`], [`visibility`], [`return_type`])
//! — adding a new facet means adding a new file and one line in each match
//! below, not editing an existing facet's file. A facet's own
//! applicability/cardinality rules are logic that file expresses directly
//! via [`check`], not passive data a shared algorithm elsewhere
//! interprets — so a facet with unusual rules doesn't need the shared
//! algorithm to grow a special case for it.
//!
//! `pub` and `-> Type` are ordinary facets here too, even though they're
//! spelled with dedicated tokens rather than a plain identifier
//! ([`crate::parser`] recognizes those tokens directly, but still produces
//! the same [`crate::ast::Facet`] shape as any other facet) — their actual
//! effect on `is_pub`/`return_ty` is computed by [`is_pub`] and
//! [`extract_return_type`], which just call into [`visibility`] and
//! [`return_type`] respectively. Every other facet's resolution-time
//! behavior (checking `invariant` at construction time, firing
//! `before`/`after` hooks) is expected to be added the same way: as
//! functions in that facet's own file, called from [`crate::resolver`].
//! This module doesn't depend on `crate::resolver` itself, so those
//! functions return the facet-owned [`Violation`] here and the resolver
//! translates it into its own error type — not the reverse.
//!
//! [`syntax`] is the first facet whose *parsed data* — not just its payload
//! shape and cardinality rule — is consumed outside its own file: a macro's
//! declared call-site pattern has to be known while parsing everything
//! after it, so `crate::parser` (matching call sites) and `crate::loader`
//! (threading patterns across file imports) both reach into it directly.
//! That's why it's `pub(crate)` rather than a bare private `mod` like the
//! other five.

mod after;
mod before;
mod invariant;
mod return_type;
pub(crate) mod syntax;
mod visibility;

use crate::ast::{Expr, Facet};
use crate::types::TypeExpr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Struct,
    Macro,
    TypeAlias,
}

/// What a facet's payload looks like after its name, e.g. `before qux()`
/// (an expression) vs. `invariant { ... }` (a block) vs. bare `pub`. Needed
/// by the parser, before resolution — grammar shape, not resolution
/// behavior, so it stays plain data rather than a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadShape {
    Bare,
    Expr,
    Block,
    Type,
}

/// Why a facet's occurrence on a declaration is invalid. Kept small and
/// facet-agnostic so the resolver can turn it into its own error type;
/// anything facet-specific about *why* belongs in that facet's own file
/// (as a doc comment, or a richer type it defines itself), not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    NotApplicable,
    TooMany,
}

/// The parser needs a payload shape before any declaration this facet is
/// attached to is resolvable — `None` means the name isn't registered.
pub fn payload_shape(name: &str) -> Option<PayloadShape> {
    match name {
        "invariant" => Some(invariant::PAYLOAD),
        "before" => Some(before::PAYLOAD),
        "after" => Some(after::PAYLOAD),
        "pub" => Some(visibility::PAYLOAD),
        "return" => Some(return_type::PAYLOAD),
        "syntax" => Some(syntax::PAYLOAD),
        _ => None,
    }
}

/// Checks one occurrence of a named facet against its own rules. `count` is
/// this occurrence's 1-based position among same-named facets on the same
/// declaration (2 means "this is the second `invariant` on this struct").
/// `None` means the name isn't registered — the parser already rejects
/// those, so a resolver caller should only ever see `Some`.
pub fn check(name: &str, decl_kind: DeclKind, count: usize) -> Option<Result<(), Violation>> {
    match name {
        "invariant" => Some(invariant::check(decl_kind, count)),
        "before" => Some(before::check(decl_kind, count)),
        "after" => Some(after::check(decl_kind, count)),
        "pub" => Some(visibility::check(decl_kind, count)),
        "return" => Some(return_type::check(decl_kind, count)),
        "syntax" => Some(syntax::check(decl_kind, count)),
        _ => None,
    }
}

/// Whether `facets` contains a `pub` entry.
pub fn is_pub(facets: &[Facet]) -> bool {
    visibility::applies(facets)
}

/// The type from a `return` (`-> Type`) entry in `facets`, if any.
pub fn extract_return_type(facets: &[Facet]) -> Option<TypeExpr> {
    return_type::extract(facets)
}

/// Every `invariant` entry's condition in `facets`, in declaration order —
/// empty if there are none.
pub fn extract_invariants(facets: &[Facet]) -> Vec<Expr> {
    invariant::extract(facets)
}
