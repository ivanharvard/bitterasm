//! Grammar for the `| name ...` modifier list that can follow a struct's
//! generic params or a macro's parameter list, e.g.
//! `macro foo(x: int) | before qux() | after baz() { ... }`. Shared by
//! [`super::declarations`] (structs) and [`super::macros`] (macros); which
//! names are valid and what shape each takes is metadata from
//! [`crate::facets`], not decided here.
//!
//! `pub` and `-> Type` are declaration syntax, not facets, and therefore do
//! not pass through this module.

use crate::ast::{Facet, FacetPayload};
use crate::facets::{self, PayloadShape};

use super::*;

impl Parser {
    // `allow_multiline`: whether a facet list may continue onto a later
    // line looking for another `|`. Newlines are consumed only when a pipe
    // actually follows; otherwise the declaration's statement terminator is
    // left for its caller. That makes multiline facets safe for type aliases
    // as well as declarations with brace-delimited bodies.
    pub(super) fn parse_facet_list(
        &mut self,
        allow_multiline: bool,
    ) -> Result<Vec<Facet>, ParseError> {
        let mut facets = Vec::new();

        loop {
            if allow_multiline {
                let before_newlines = self.pos;
                self.skip_newlines();
                if !self.check(&TokenKind::Pipe) {
                    self.pos = before_newlines;
                }
            }

            if !self.check(&TokenKind::Pipe) {
                break;
            }

            self.advance();

            facets.push(self.parse_facet()?);
        }

        Ok(facets)
    }

    fn parse_facet(&mut self) -> Result<Facet, ParseError> {
        let start = self.current().span.start;

        let name = if self.check(&TokenKind::From) {
            self.advance();
            "from".to_string()
        } else {
            self.expect_identifier()?
        };

        let Some(shape) = facets::payload_shape(&name) else {
            return Err(ParseError::new(
                format!("unknown facet `{name}`"),
                self.previous().span,
            ));
        };

        let is_lint_facet = matches!(name.as_str(), "allow" | "expect" | "warn" | "deny" | "forbid");
        let payload = if is_lint_facet && self.check(&TokenKind::LParen) {
            let open = self.current().span;
            self.advance();
            let mut arguments = Vec::new();
            if !self.check(&TokenKind::RParen) {
                loop {
                    let value = self.parse_expr()?;
                    arguments.push(CallArgument { name: None, span: value.span(), value });
                    if !self.check(&TokenKind::Comma) { break; }
                    self.advance();
                }
            }
            let close = self.current().span;
            self.expect_simple(TokenKind::RParen)?;
            FacetPayload::Expr(Expr::Call {
                callee: Box::new(Expr::Identifier {
                    name: "lints".to_string(),
                    span: open,
                }),
                arguments,
                span: Span::new(open.start, close.end),
            })
        } else if is_lint_facet {
            let token = self.current().clone();
            let selector = self.expect_identifier()?;
            FacetPayload::Expr(Expr::Identifier { name: selector, span: token.span })
        } else { match shape {
            // Bounded the same way custom invocation-syntax capture
            // matching bounds a capture (`parse_expr_bp_until`'s own doc) —
            // `|` is also `BinaryOp::BitOr`, so an unbounded parse of
            // `invariant a > 0 | invariant b < 10` would swallow the next
            // facet's leading `|` as its own continuation instead of
            // stopping there. Parens still allow a genuine bitwise-or
            // inside a condition (`invariant (a | b) > 0`) — entering them
            // always re-parses with no `stop` at all, the same way they
            // already suspend `restrict_closing_ops`/`restrict_brace_construction`.
            PayloadShape::Expr => {
                FacetPayload::Expr(self.parse_expr_bp_until(0, Some(&TokenKind::Pipe))?)
            }

            PayloadShape::Block => {
                self.expect_simple(TokenKind::LBrace)?;
                self.skip_newlines();

                let mut statements = Vec::new();

                while !self.check(&TokenKind::RBrace) {
                    if self.at_eof() {
                        return Err(ParseError::new(
                            format!("unterminated `{name}` facet body"),
                            self.current().span,
                        ));
                    }

                    statements.push(self.parse_statement()?);
                    self.skip_newlines();
                }

                self.advance();

                FacetPayload::Block(statements)
            }

            // `pub` and `return` are intercepted above, before reaching
            // this identifier-based path, since they're spelled with
            // dedicated tokens rather than a plain identifier — their
            // shapes never actually reach this match.
            PayloadShape::Bare | PayloadShape::Type => unreachable!(
                "facet `{name}` has a dedicated-token payload shape but was parsed via the identifier path"
            ),
        }};

        if is_lint_facet {
            let selectors: Vec<&str> = match &payload {
                FacetPayload::Expr(Expr::Identifier { name, .. }) => vec![name],
                FacetPayload::Expr(Expr::Call { arguments, .. }) => arguments
                    .iter()
                    .filter_map(|argument| match &argument.value {
                        Expr::Identifier { name, .. } => Some(name.as_str()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            if selectors.is_empty() {
                return Err(ParseError::new(
                    format!("the `{name}` facet requires one or more lint names"),
                    Span::new(start, self.previous().span.end),
                ));
            }
            for selector in selectors {
                if crate::diagnostics::LintName::named(selector).is_none()
                    && crate::diagnostics::LintName::group(selector).is_none()
                {
                    return Err(ParseError::new(
                        format!("unknown lint or lint group `{selector}`"),
                        Span::new(start, self.previous().span.end),
                    ));
                }
            }
        }

        let end = self.previous().span.end;

        Ok(Facet {
            name,
            payload,
            span: Span::new(start, end),
        })
    }

}
