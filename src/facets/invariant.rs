//! `invariant` — struct-only. At most one per struct. Its expression (often
//! a call to a separately-declared bool-returning macro, e.g.
//! `invariant fits_inside_width(width, value)`, so the check can be shared
//! across declarations instead of duplicated inline) is checked against the
//! struct's own fields and generic const params whenever a value of that
//! struct is constructed; compilation fails if it doesn't fold to a truthy
//! result. Not yet enforced — needs const evaluation, which doesn't exist
//! yet (see [`crate::resolver`]).

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

pub fn check(decl_kind: DeclKind, count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Struct {
        return Err(Violation::NotApplicable);
    }

    if count > 1 {
        return Err(Violation::TooMany);
    }

    Ok(())
}
