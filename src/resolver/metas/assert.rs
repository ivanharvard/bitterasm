//! `@assert <expr>[, "message"]` — evaluates `expr` exactly the way
//! `@emit` does ([`AliasResolver::eval_value`]), then checks the result is
//! truthy under the language's `0`/`1` `Int` convention for booleans (see
//! the [`crate::eval`] module doc) — the same check `invariant`
//! ([`crate::facets::invariant`]) is meant to eventually perform at
//! struct-construction time, just spelled as an explicit statement rather
//! than an implicit per-construction hook. Nothing is pushed onto the
//! expansion's `emitted` stream either way: a passing assertion is
//! silent, a failing one aborts resolution with
//! [`ResolveError::AssertionFailed`], carrying the optional message
//! along verbatim so the caller decides how to display it.

use std::collections::HashMap;

use crate::ast::Expr;
use crate::token::Span;

use crate::resolver::{AliasResolver, ResolveError, Value};

pub fn check(
    resolver: &mut AliasResolver<'_>,
    args: &[Expr],
    scope: &HashMap<String, Value>,
    span: Span,
) -> Result<(), ResolveError> {
    let (condition, message) = match args {
        [condition] => (condition, None),

        [condition, Expr::String { value, .. }] => (condition, Some(value.clone())),

        // Present, but not a string literal — e.g. `@assert x, 1`.
        [_, other] => return Err(ResolveError::InvalidAssertMessage { span: other.span() }),

        // `@return`'s 0-or-1-argument case reports its own upper bound the
        // same way when the count is wrong either way; there's no
        // "expected 1 or 2" shape to report instead.
        other => {
            return Err(ResolveError::InvalidArgumentCount {
                name: "@assert".to_string(),
                expected: 2,
                actual: other.len(),
                span,
            })
        }
    };

    if resolver.eval_truthy(condition, scope)? {
        Ok(())
    } else {
        Err(ResolveError::AssertionFailed { message, span })
    }
}
