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
    pub(super) fn parse_facet_list(
        &mut self,
        allow_return_type: bool,
    ) -> Result<Vec<Facet>, ParseError> {
        let mut facets = Vec::new();

        if self.check(&TokenKind::Arrow) {
            facets.push(self.parse_return_type_facet(allow_return_type)?);
        }

        loop {
            self.skip_newlines();

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
            PayloadShape::Expr => FacetPayload::Expr(self.parse_expr()?),

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
