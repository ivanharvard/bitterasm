//! `after` — macro-only. May appear multiple times; each one fires once,
//! in declaration order, after the macro's own body (the reverse-ordered
//! counterpart to [`super::before`]). Not yet enforced — needs
//! invocation-to-macro binding, which doesn't exist yet (see
//! [`crate::resolver`]).

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

pub fn check(decl_kind: DeclKind, _count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Macro {
        return Err(Violation::NotApplicable);
    }

    Ok(())
}
