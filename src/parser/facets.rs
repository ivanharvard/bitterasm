//! Grammar for the `| name ...` modifier list that can follow a struct's
//! generic params or a macro's parameter list, e.g.
//! `macro foo(x: int) | before qux() | after baz() { ... }`. Shared by
//! [`super::declarations`] (structs) and [`super::macros`] (macros); which
//! names are valid and what shape each takes is metadata from
//! [`crate::facets`], not decided here.
//!
//! `pub` and `-> Type` are spelled with dedicated tokens rather than a
//! plain identifier, so they're recognized directly here rather than
//! falling through to the general `name (payload)` grammar every other
//! facet uses — but they still end up as ordinary [`Facet`] entries in the
//! returned list, validated the same way as any other facet. `-> Type` may
//! also appear bare (no leading `|`) directly after the parameter list,
//! which is the original, simpler form; both spellings produce the same
//! facet.

use crate::ast::{Facet, FacetPayload};
use crate::facets::{self, PayloadShape};

use super::*;

impl Parser {
    // `allow_multiline`: whether a facet list may continue onto a later
    // line looking for another `|` — safe for a struct/macro declaration,
    // since whatever follows the facet list is always its own `{ ... }`
    // body, which tolerates (and itself skips) leading newlines regardless.
    // A `type` alias has no such body — it's a single-line statement ending
    // in a plain newline — so eagerly skipping newlines here would consume
    // that terminator whenever the alias has zero or a single-line facet
    // list, mistaking the next statement's first token for a continuation.
    pub(super) fn parse_facet_list(
        &mut self,
        allow_return_type: bool,
        allow_multiline: bool,
    ) -> Result<Vec<Facet>, ParseError> {
        let mut facets = Vec::new();

        if self.check(&TokenKind::Arrow) {
            facets.push(self.parse_return_type_facet(allow_return_type)?);
        }

        loop {
            if allow_multiline {
                self.skip_newlines();
            }

            if !self.check(&TokenKind::Pipe) {
                break;
            }

            self.advance();

            facets.push(self.parse_facet(allow_return_type)?);
        }

        Ok(facets)
    }

    fn parse_facet(&mut self, allow_return_type: bool) -> Result<Facet, ParseError> {
        if self.check(&TokenKind::Arrow) {
            return self.parse_return_type_facet(allow_return_type);
        }

        if self.check(&TokenKind::Pub) {
            let start = self.current().span.start;
            self.advance();
            let end = self.previous().span.end;

            return Ok(Facet {
                name: "pub".to_string(),
                payload: FacetPayload::Bare,
                span: Span::new(start, end),
            });
        }

        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        let Some(shape) = facets::payload_shape(&name) else {
            return Err(ParseError::new(
                format!("unknown facet `{name}`"),
                self.previous().span,
            ));
        };

        let payload = match shape {
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
        };

        let end = self.previous().span.end;

        Ok(Facet {
            name,
            payload,
            span: Span::new(start, end),
        })
    }

    fn parse_return_type_facet(&mut self, allow_return_type: bool) -> Result<Facet, ParseError> {
        let start = self.current().span.start;

        if !allow_return_type {
            return Err(ParseError::new(
                "`->` is not valid here",
                self.current().span,
            ));
        }

        self.expect_simple(TokenKind::Arrow)?;

        let ty = self.parse_type_expr()?;
        let end = self.previous().span.end;

        Ok(Facet {
            name: "return".to_string(),
            payload: FacetPayload::Type(ty),
            span: Span::new(start, end),
        })
    }
}
