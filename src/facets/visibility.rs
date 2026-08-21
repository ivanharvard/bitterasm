//! `pub` — shared by structs and macros; sugar for the same visibility
//! mechanism as the `pub` prefix keyword. At most one per declaration
//! (writing it twice has no additional effect, but rejecting it keeps the
//! error honest rather than silently ignoring a mistake).

use crate::ast::Facet;

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Bare;

pub fn check(_decl_kind: DeclKind, count: usize) -> Result<(), Violation> {
    if count > 1 {
        return Err(Violation::TooMany);
    }

    Ok(())
}

pub fn applies(facets: &[Facet]) -> bool {
    facets.iter().any(|facet| facet.name == "pub")
}
