//! Grammar for a brace-literal construction's field list — `{ name: value,
//! @for ..., @if ... }` — the value-expression counterpart of
//! `declarations::parse_struct_body_items`. Deliberately mirrors that
//! grammar item-for-item (same comma/trailing-comma rules, same
//! `@for`/`@if` self-delimiting), just built on [`ConstructItem`]/value
//! expressions instead of [`StructBodyItem`]/declared types.

use crate::ast::ConstructItem;

use super::*;

impl Parser {
    // Finishes parsing a brace-literal construction, given `callee` and any
    // already-parsed `generic_args` (empty for a non-generic callee), and
    // the parser sitting on the opening `{` (not yet consumed).
    pub(super) fn finish_construct(
        &mut self,
        callee: Expr,
        generic_args: Vec<TypeArgument>,
    ) -> Result<Expr, ParseError> {
        let start = callee.span().start;

        let (fields, body_end) = self.parse_construct_body_items()?;

        Ok(Expr::Construct {
            callee: Box::new(callee),
            generic_args,
            fields,
            span: Span::new(start, body_end),
        })
    }

    // Parses `{ item, item, ... }`, given the opening `{` hasn't been
    // consumed yet — the body of a brace-literal construction, or of a
    // construction body's own `@for`/`@if`. A generative item (`@for`/`@if`)
    // is self-delimited by its own closing brace and doesn't need a
    // trailing comma the way a plain field does; one is still allowed if
    // present, for consistency — same rule as `declarations::parse_struct_body_items`.
    fn parse_construct_body_items(&mut self) -> Result<(Vec<ConstructItem>, usize), ParseError> {
        self.expect_simple(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut items = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(
                    "unterminated brace-literal construction",
                    self.current().span,
                ));
            }

            let item = self.parse_construct_item()?;
            let is_generative = matches!(item, ConstructItem::For { .. } | ConstructItem::If { .. });
            items.push(item);

            self.skip_newlines();

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else if !self.check(&TokenKind::RBrace) && !is_generative {
                return Err(ParseError::new(
                    "expected ',' or '}' after a construction field",
                    self.current().span,
                ));
            }
        }

        let closing = self.current().clone();
        self.expect_simple(TokenKind::RBrace)?;

        Ok((items, closing.span.end))
    }

    fn parse_construct_item(&mut self) -> Result<ConstructItem, ParseError> {
        if self.check(&TokenKind::At) {
            self.parse_construct_generative_item()
        } else {
            self.parse_construct_field()
        }
    }

    fn parse_construct_generative_item(&mut self) -> Result<ConstructItem, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::At)?;

        let name_token = self.current().clone();
        let name = self.expect_identifier()?;

        match name.as_str() {
            "for" => self.parse_construct_for_item(start),
            "if" => self.parse_construct_if_item(start),

            other => Err(ParseError::new(
                format!("`@{other}` isn't valid inside a brace-literal construction — only `@for`/`@if` are"),
                name_token.span,
            )),
        }
    }

    fn parse_construct_for_item(&mut self, start: usize) -> Result<ConstructItem, ParseError> {
        let var = self.expect_identifier()?;

        self.expect_keyword("in")?;

        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let result = self.parse_range_bounds();

        self.restrict_brace_construction = outer_restriction;

        let (range_start, range_end) = result?;

        self.skip_newlines();

        let (body, body_end) = self.parse_construct_body_items()?;

        Ok(ConstructItem::For {
            var,
            start: range_start,
            end: range_end,
            body,
            span: Span::new(start, body_end),
        })
    }

    fn parse_construct_if_item(&mut self, start: usize) -> Result<ConstructItem, ParseError> {
        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let condition = self.parse_expr();

        self.restrict_brace_construction = outer_restriction;

        let condition = condition?;

        self.skip_newlines();

        let (body, mut end) = self.parse_construct_body_items()?;

        let else_body = if self.at_else_meta() {
            self.advance(); // `@`
            self.advance(); // `else`
            self.skip_newlines();

            let (else_body, else_end) = self.parse_construct_body_items()?;
            end = else_end;

            Some(else_body)
        } else {
            None
        };

        Ok(ConstructItem::If {
            condition,
            body,
            else_body,
            span: Span::new(start, end),
        })
    }

    fn parse_construct_field(&mut self) -> Result<ConstructItem, ParseError> {
        let start = self.current().span.start;

        let (name, _) = self.parse_spliced_name()?;

        self.expect_simple(TokenKind::Colon)?;

        let value = self.parse_expr()?;

        let end = value.span().end;

        Ok(ConstructItem::Field {
            name,
            value,
            span: Span::new(start, end),
        })
    }
}
