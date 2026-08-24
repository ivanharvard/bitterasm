//! `invariant` — valid on `struct` and `type` alias declarations. May appear
//! more than once; when it does, every occurrence must hold (an implicit AND
//! across separate conditions) rather than composing in sequence the way
//! `before`/`after` do.
//!
//! On a **struct**, each expression (often a call to a separately-declared
//! bool-returning macro, e.g. `invariant fits_inside_width(width, value)`,
//! so a check can be shared across declarations instead of duplicated
//! inline) is checked against the struct's own fields and generic const
//! params — enforced at every construction site
//! (`resolver::structs::AliasResolver::check_struct_invariants`, called from
//! both `resolver::values::eval_call_value` and `eval_construct_value`).
//!
//! On a **type alias**, there are no fields/params of its own to check
//! against — the expression instead names its own binder for "the value
//! being converted into this type," the author's choice
//! (`type Age = int | invariant years >= 0`, `years` here), not a fixed
//! reserved word. Since an alias invariant's scope is otherwise completely
//! empty, that binder is just whichever free identifier (one that doesn't
//! already resolve to a real symbol) appears in the expression — validated
//! to be exactly one consistent name across all of an alias's `invariant`
//! occurrences by `resolver::facets::validate_alias_invariant_binder`.
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

// Every `invariant` occurrence's condition, in declaration order — unlike
// `return_type::extract`'s `find_map` (at most one `-> Type` makes sense),
// `invariant` is explicitly allowed to appear more than once, with every
// occurrence required to hold (see this file's module doc).
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
