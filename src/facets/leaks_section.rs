//! `leaks_section` — macro-only, bare marker, at most one. Opts this macro
//! out of the automatic section push/pop restore that every macro call gets
//! by default (`crate::resolver::macro_body::run_macro_body_inner`), for
//! the deliberate case of a macro meant to behave like a bare `section`
//! statement itself — e.g. a convenience wrapper whose whole point is
//! changing what section the *caller's* subsequent code lands in. Without
//! this facet, a `section` statement inside a macro body is scoped to that
//! call and restored on return; see
//! `docs/sections-and-linking/PROGRESS.md`, "Decided scope", for why that's
//! the safe default and this facet exists as the explicit, visible
//! exception to it — the same shape `before`/`after`/`syntax` already
//! establish for "safe default, opt-in exception, declared right on the
//! macro."

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Bare;

pub fn check(decl_kind: DeclKind, count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Macro {
        return Err(Violation::NotApplicable);
    }

    if count > 1 {
        return Err(Violation::TooMany);
    }

    Ok(())
}
