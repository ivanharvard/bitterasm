//! `warn` — enables one lint (or lint group) for this declaration.
use super::{DeclKind, PayloadShape, Violation};
pub const PAYLOAD: PayloadShape = PayloadShape::Expr;
pub fn check(_decl_kind: DeclKind, _count: usize) -> Result<(), Violation> { Ok(()) }
