//! `from` — an inbound conversion template on a struct or type alias.

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

pub fn check(decl_kind: DeclKind, _count: usize) -> Result<(), Violation> {
    if matches!(decl_kind, DeclKind::Struct | DeclKind::TypeAlias) {
        Ok(())
    } else {
        Err(Violation::NotApplicable)
    }
}
