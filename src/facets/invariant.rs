//! `invariant` — valid on `struct` and `type` alias declarations. May appear
//! more than once; when it does, every occurrence must hold (an implicit AND
//! across separate conditions) rather than composing in sequence the way
//! `before`/`after` do.
//!
//! On a **struct**, `source` is the completed value, so fields are reached as
//! `source.field`; generic const parameters remain directly available. Each
//! expression (often a call to a separately-declared bool-returning macro),
//! so a check can be shared across declarations instead of duplicated
//! inline) is checked against the struct's own fields and generic const
//! params — enforced at every construction site
//! (`resolver::structs::AliasResolver::check_struct_invariants`, called from
//! both `resolver::values::eval_call_value` and `eval_construct_value`).
//!
//! On a **type alias**, `source` is the value being converted into the alias.
//! Compilation fails if a checked expression doesn't fold to a truthy
//! result.

use crate::ast::{Facet, FacetPayload, Expr};

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

pub fn check(decl_kind: DeclKind, _count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Struct && decl_kind != DeclKind::TypeAlias {
        return Err(Violation::NotApplicable);
    }

    Ok(())
}

// Every `invariant` occurrence's condition, in declaration order. An
// invariant may appear more than once, and every occurrence must hold.
pub fn extract(facets: &[Facet]) -> Vec<Expr> {
    facets
        .iter()
        .filter_map(|facet| {
            if facet.name != "invariant" {
                return None;
            }

            let FacetPayload::Expr(expr) = &facet.payload else {
                return None;
            };

            Some(expr.clone())
        })
        .collect()
}
