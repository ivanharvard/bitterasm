//! `emits` — one type an `@emit` inside this macro's body may produce.
//! Repeatable, same as `to`/`from`: several `| emits <T>` facets on one
//! macro declare a set, not a sequence — nothing requires every member to
//! actually appear on a given call, only that no `@emit`ed value's type
//! ever falls outside the declared set. See
//! `AliasResolver::check_emitted_type` (`crate::resolver::macro_body`) for
//! the check this enables; a macro with no `emits` facet at all stays
//! entirely unconstrained, exactly as before this facet existed.

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Type;

pub fn check(decl_kind: DeclKind, _count: usize) -> Result<(), Violation> {
    if matches!(decl_kind, DeclKind::Macro) {
        Ok(())
    } else {
        Err(Violation::NotApplicable)
    }
}
