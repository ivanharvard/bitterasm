//! `invariant` — struct-only. May appear more than once; when it does,
//! every occurrence must hold (an implicit AND across separate conditions)
//! rather than composing in sequence the way `before`/`after` do. Each
//! expression (often a call to a separately-declared bool-returning macro,
//! e.g. `invariant fits_inside_width(width, value)`, so a check can be
//! shared across declarations instead of duplicated inline) is checked
//! against the struct's own fields and generic const params whenever a
//! value of that struct is constructed; compilation fails if any of them
//! doesn't fold to a truthy result. Not yet enforced — needs a way to bind
//! a construction call's arguments to the struct's fields, which doesn't
//! exist yet (see [`crate::resolver`]); const evaluation itself
//! ([`crate::eval`]) is already there.

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

pub fn check(decl_kind: DeclKind, _count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Struct {
        return Err(Violation::NotApplicable);
    }

    Ok(())
}
