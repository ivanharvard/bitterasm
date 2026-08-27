//! `forbid` — promotes a lint to an error and prevents lowering it below this scope.
use super::{DeclKind, PayloadShape, Violation};
pub const PAYLOAD: PayloadShape = PayloadShape::Expr;
pub fn check(_decl_kind: DeclKind, _count: usize) -> Result<(), Violation> { Ok(()) }
