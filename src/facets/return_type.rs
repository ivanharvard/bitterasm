//! `return` (spelled `-> Type` in source, e.g. `-> bool` or `| -> bool`) —
//! macro-only. At most one per macro. The piped `| -> Type` form is sugar
//! for the same thing as writing `-> Type` directly after the parameter
//! list; both parse into the same facet, so writing both on one
//! declaration is a duplicate, not two different things merging.

use crate::ast::{Facet, FacetPayload};
use crate::types::TypeExpr;

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Type;

pub fn check(decl_kind: DeclKind, count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Macro {
        return Err(Violation::NotApplicable);
    }

    if count > 1 {
        return Err(Violation::TooMany);
    }

    Ok(())
}

pub fn extract(facets: &[Facet]) -> Option<TypeExpr> {
    facets.iter().find_map(|facet| {
        if facet.name != "return" {
            return None;
        }

        let FacetPayload::Type(ty) = &facet.payload else {
            return None;
        };

        Some(ty.clone())
    })
}
