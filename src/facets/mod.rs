//! Facet-specific logic and metadata live in each facet's own file
//! ([`invariant`], [`before`], [`after`], [`syntax`])
//! — adding a new facet means adding a new file and one line in each match
//! below, not editing an existing facet's file. A facet's own
//! applicability/cardinality rules are logic that file expresses directly
//! via [`check`], not passive data a shared algorithm elsewhere
//! interprets — so a facet with unusual rules doesn't need the shared
//! algorithm to grow a special case for it.
//!
//! `pub` and `-> Type` belong directly to declaration signatures and are
//! deliberately not facets. Every facet's resolution-time
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
//! other facet modules.

mod after;
mod before;
mod invariant;
mod from;
mod to;
pub(crate) mod syntax;

use crate::ast::{Expr, Facet, FacetPayload};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Struct,
    Macro,
    TypeAlias,
}

/// What a facet's payload looks like after its name, e.g. `before qux()`
/// (an expression) vs. `invariant { ... }` (a block). Needed
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
        "from" => Some(from::PAYLOAD),
        "to" => Some(to::PAYLOAD),
        "before" => Some(before::PAYLOAD),
        "after" => Some(after::PAYLOAD),
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
        "from" => Some(from::check(decl_kind, count)),
        "to" => Some(to::check(decl_kind, count)),
        "before" => Some(before::check(decl_kind, count)),
        "after" => Some(after::check(decl_kind, count)),
        "syntax" => Some(syntax::check(decl_kind, count)),
        _ => None,
    }
}

/// Every `invariant` entry's condition in `facets`, in declaration order —
/// empty if there are none.
pub fn extract_invariants(facets: &[Facet]) -> Vec<Expr> {
    invariant::extract(facets)
}

pub fn extract_exprs(facets: &[Facet], name: &str) -> Vec<Expr> {
    facets
        .iter()
        .filter_map(|facet| match (&facet.name[..], &facet.payload) {
            (facet_name, FacetPayload::Expr(expr)) if facet_name == name => Some(expr.clone()),
            _ => None,
        })
        .collect()
}
