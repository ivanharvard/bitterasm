use crate::ast::StructDeclaration;

use super::*;

impl Parser {
    // =============
    // structs
    // =============

    pub(super) fn parse_struct_declaration(
        &mut self,
        is_pub: bool,
    ) -> Result<StructDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Struct)?;

        let name = self.expect_identifier()?;

        let generic_params = self.parse_generic_params()?;

        self.register_generic_signature(&name, &generic_params);

        let facets = self.parse_facet_list(false)?;
        let is_pub = is_pub || crate::facets::is_pub(&facets);

        self.skip_newlines();

        self.expect_simple(TokenKind::LBrace)?;

        self.skip_newlines();

        let mut fields = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(
                    "unterminated struct declaration",
                    self.current().span,
                ));
            }

            fields.push(self.parse_struct_field()?);

            self.skip_newlines();

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else if !self.check(&TokenKind::RBrace) {
                return Err(ParseError::new(
                    "expected ',' or '}' after struct field",
                    self.current().span,
                ));
            }
        }

        let closing = self.current().clone();

        self.expect_simple(TokenKind::RBrace)?;

        if self.check(&TokenKind::Newline) {
            self.advance();
        }

        Ok(StructDeclaration {
            name,
            is_pub,
            generic_params,
            facets,
            fields,
            span: Span::new(start, closing.span.end),
        })
    }

    fn parse_struct_field(
        &mut self,
    ) -> Result<StructField, ParseError> {
        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        self.expect_simple(TokenKind::Colon)?;

        let ty = self.parse_type_expr()?;

        let end = ty.span().end;

        Ok(StructField {
            name,
            ty,
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
            generic_params: generic_params,
            ty: target,
            span: Span::new(start, end),
        })
    }
}
