//! `@error "message"` — stops compilation with `message`. The same as
//! `@assert 0, "message"`, for the branch of an `@if` that's always wrong.

use crate::ast::Expr;
use crate::token::Span;

use crate::resolver::ResolveError;

pub fn raise(args: &[Expr], span: Span) -> ResolveError {
    match args {
        [Expr::String { value, .. }] => ResolveError::AssertionFailed { message: Some(value.clone()), span },
        [other] => ResolveError::InvalidAssertMessage { span: other.span() },
        other => ResolveError::InvalidArgumentCount {
            name: "@error".to_string(),
            expected: 1,
            actual: other.len(),
            span,
        },
    }
}
