//! `after` — macro-only. May appear multiple times; each one fires once,
//! in declaration order, after the macro's own body (the reverse-ordered
//! counterpart to [`super::before`]). Macro parameters remain directly
//! available; `self.returned` exposes the completed return value.

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

pub fn check(decl_kind: DeclKind, _count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Macro {
        return Err(Violation::NotApplicable);
    }

    Ok(())
}
