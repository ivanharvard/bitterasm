//! `expect` — suppresses a lint here and warns when it does not occur.
use super::{DeclKind, PayloadShape, Violation};
pub const PAYLOAD: PayloadShape = PayloadShape::Expr;
pub fn check(_decl_kind: DeclKind, _count: usize) -> Result<(), Violation> { Ok(()) }
