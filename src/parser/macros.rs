use crate::ast::FacetPayload;

use super::*;

impl Parser {
    // =============
    // meta
    // =============

    pub(super) fn parse_meta_statement(
        &mut self
    ) -> Result<MetaStatement, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::At)?;

        let name = self.expect_identifier()?;

        let mut args = Vec::new();

        if !self.at_statement_end() {
            args.push(self.parse_expr()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }

        let end = self.statement_end()?;

        Ok(MetaStatement {
            name,
            args,
            body: None,
            span: Span::new(start, end),
        })
    }

    // =============
    // macros
    // =============

    pub(super) fn parse_macro_declaration(
        &mut self,
        is_pub: bool,
    ) -> Result<MacroDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Macro)?;

        let name = self.expect_identifier()?;

        self.expect_simple(TokenKind::LParen)?;

        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            params.push(self.parse_macro_parameter()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                params.push(self.parse_macro_parameter()?);
            }
        }

        self.expect_simple(TokenKind::RParen)?;

        let facets = self.parse_facet_list(true)?;
        let return_ty = crate::facets::extract_return_type(&facets);
        let is_pub = is_pub || crate::facets::is_pub(&facets);

        if let Some(facet) = facets.iter().find(|facet| facet.name == "syntax") {
            let FacetPayload::Expr(Expr::String { value, .. }) = &facet.payload else {
                return Err(ParseError::new(
                    "the `syntax` facet requires a string literal pattern",
                    facet.span,
                ));
            };

            let param_names: Vec<String> = params.iter().map(|param| param.name.clone()).collect();

            let pattern = crate::facets::syntax::parse_pattern(&name, value, &param_names)
                .map_err(|message| ParseError::new(message, facet.span))?;

            self.register_macro_syntax(&name, pattern);
        }

        self.skip_newlines();

        self.expect_simple(TokenKind::LBrace)?;

        self.skip_newlines();

        let mut body = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(
                    "unterminated macro body",
                    self.current().span,
                ));
            }

            body.push(self.parse_statement()?);

            self.skip_newlines();
        }

        let closing = self.current().clone();

        self.expect_simple(TokenKind::RBrace)?;

        if self.check(&TokenKind::Newline) {
            self.advance();
        }

        Ok(MacroDeclaration {
            name,
            is_pub,
            params,
            return_ty,
            facets,
            body,
            span: Span::new(start, closing.span.end),
        })
    }

    fn parse_macro_parameter(
        &mut self,
    ) -> Result<MacroParameter, ParseError> {
        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        self.expect_simple(TokenKind::Colon)?;

        let ty = self.parse_type_expr()?;

        let end = ty.span().end;

        Ok(MacroParameter {
            name,
            ty,
            span: Span::new(start, end),
        })
    }
}
