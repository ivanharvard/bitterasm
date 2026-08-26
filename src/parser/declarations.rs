use crate::ast::{EnumDeclaration, StructDeclaration};

use super::*;

impl Parser {
    // =============
    // structs
    // =============

    pub(super) fn parse_enum_declaration(
        &mut self,
        is_pub: bool,
    ) -> Result<EnumDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Enum)?;

        let name = self.expect_identifier()?;

        self.skip_newlines();
        self.expect_simple(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut variants = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(
                    "unterminated enum declaration",
                    self.current().span,
                ));
            }

            variants.push(self.expect_identifier()?);

            self.skip_newlines();

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else if !self.check(&TokenKind::RBrace) {
                return Err(ParseError::new(
                    "expected ',' or '}' after enum variant",
                    self.current().span,
                ));
            }
        }

        let closing = self.current().clone();
        self.expect_simple(TokenKind::RBrace)?;
        self.consume_trailing_newline();

        Ok(EnumDeclaration {
            name,
            is_pub,
            variants,
            span: Span::new(start, closing.span.end),
        })
    }

    pub(super) fn parse_struct_declaration(
        &mut self,
        is_pub: bool,
    ) -> Result<StructDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Struct)?;

        let name = self.expect_identifier()?;

        let generic_params = self.parse_generic_params()?;

        self.register_generic_signature(&name, &generic_params);

        let facets = self.parse_facet_list(true)?;

        self.skip_newlines();

        let (fields, body_end) = self.parse_struct_body_items()?;

        self.consume_trailing_newline();

        Ok(StructDeclaration {
            name,
            is_pub,
            generic_params,
            facets,
            fields,
            span: Span::new(start, body_end),
        })
    }

    // Parses `{ item, item, ... }`, given the opening `{` hasn't been
    // consumed yet — the body of a struct declaration, or of a struct
    // body's own `@for`/`@if`. A generative item (`@for`/`@if`) is
    // self-delimited by its own closing brace and doesn't need a trailing
    // comma the way a plain field does; one is still allowed if present,
    // for consistency.
    fn parse_struct_body_items(&mut self) -> Result<(Vec<StructBodyItem>, usize), ParseError> {
        self.expect_simple(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut items = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(
                    "unterminated struct declaration",
                    self.current().span,
                ));
            }

            let item = self.parse_struct_body_item()?;
            let is_generative = matches!(item, StructBodyItem::For { .. } | StructBodyItem::If { .. });
            items.push(item);

            self.skip_newlines();

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else if !self.check(&TokenKind::RBrace) && !is_generative {
                return Err(ParseError::new(
                    "expected ',' or '}' after struct field",
                    self.current().span,
                ));
            }
        }

        let closing = self.current().clone();
        self.expect_simple(TokenKind::RBrace)?;

        Ok((items, closing.span.end))
    }

    fn parse_struct_body_item(&mut self) -> Result<StructBodyItem, ParseError> {
        if self.check(&TokenKind::At) {
            self.parse_struct_generative_item()
        } else {
            Ok(StructBodyItem::Field(self.parse_struct_field()?))
        }
    }

    fn parse_struct_generative_item(&mut self) -> Result<StructBodyItem, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::At)?;

        let name_token = self.current().clone();
        let name = self.expect_identifier()?;

        match name.as_str() {
            "for" => self.parse_struct_for_item(start),
            "if" => self.parse_struct_if_item(start),

            other => Err(ParseError::new(
                format!("`@{other}` isn't valid inside a struct body — only `@for`/`@if` are"),
                name_token.span,
            )),
        }
    }

    fn parse_struct_for_item(&mut self, start: usize) -> Result<StructBodyItem, ParseError> {
        let var = self.expect_identifier()?;

        self.expect_keyword("in")?;

        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let result = self.parse_expr();

        self.restrict_brace_construction = outer_restriction;

        let source = result?;

        self.skip_newlines();

        let (body, body_end) = self.parse_struct_body_items()?;

        Ok(StructBodyItem::For {
            var,
            source,
            body,
            span: Span::new(start, body_end),
        })
    }

    fn parse_struct_if_item(&mut self, start: usize) -> Result<StructBodyItem, ParseError> {
        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let condition = self.parse_expr();

        self.restrict_brace_construction = outer_restriction;

        let condition = condition?;

        self.skip_newlines();

        let (body, mut end) = self.parse_struct_body_items()?;

        let else_body = if self.at_else_meta() {
            self.advance(); // `@`
            self.advance(); // `else`
            self.skip_newlines();

            let (else_body, else_end) = self.parse_struct_body_items()?;
            end = else_end;

            Some(else_body)
        } else {
            None
        };

        Ok(StructBodyItem::If {
            condition,
            body,
            else_body,
            span: Span::new(start, end),
        })
    }

    fn parse_struct_field(
        &mut self,
    ) -> Result<StructField, ParseError> {
        let start = self.current().span.start;

        let is_pub = if self.check(&TokenKind::Pub) {
            self.advance();
            true
        } else {
            false
        };

        let is_const = if self.check(&TokenKind::Const) {
            self.advance();
            true
        } else {
            false
        };

        let (name, _) = self.parse_spliced_name()?;

        self.expect_simple(TokenKind::Colon)?;

        let ty = self.parse_type_expr()?;

        let mut end = ty.span().end;

        let default = if self.check(&TokenKind::Equal) {
            self.advance();

            let default = self.parse_expr()?;
            end = default.span().end;

            Some(default)
        } else {
            None
        };

        Ok(StructField {
            name,
            ty,
            is_pub,
            is_const,
            default,
            span: Span::new(start, end),
        })
    }

    // =============
    // type aliases
    // =============

    pub(super) fn parse_type_alias(
        &mut self,
        is_pub: bool,
    ) -> Result<TypeAliasDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Type)?;

        let name = self.expect_identifier()?;

        let generic_params =
            self.parse_generic_params()?;

        self.register_generic_signature(&name, &generic_params);

        self.expect_simple(TokenKind::Equal)?;

        let target = self.parse_type_expr()?;

        let facets = self.parse_facet_list(true)?;

        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!(
                    "unexpected token after type alias: {:?}",
                    self.current().kind
                ),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        Ok(TypeAliasDeclaration {
            name,
            is_pub,
            generic_params,
            facets,
            ty: target,
            span: Span::new(start, end),
        })
    }
}
