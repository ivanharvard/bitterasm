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

        match name.as_str() {
            "for" => self.parse_for_meta(start),
            "if" => self.parse_if_meta(start),

            _ => {
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
                    else_body: None,
                    span: Span::new(start, end),
                })
            }
        }
    }

    // `@for name in start..end { body }` — the loop variable and range
    // bounds are packed positionally into `args` (`[Identifier, start,
    // end]`) rather than given their own `MetaStatement` fields, the same
    // way `@assert`'s `[condition, message]` already overloads `args`.
    fn parse_for_meta(&mut self, start: usize) -> Result<MetaStatement, ParseError> {
        let var_token = self.current().clone();
        let var_name = self.expect_identifier()?;
        let var = Expr::Identifier { name: var_name, span: var_token.span };

        self.expect_keyword("in")?;

        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let result = self.parse_range_bounds();

        self.restrict_brace_construction = outer_restriction;

        let (range_start, range_end) = result?;

        self.skip_newlines();

        let (body, body_end) =
            self.parse_statement_block("unterminated `@for` body")?;

        self.consume_trailing_newline();

        Ok(MetaStatement {
            name: "for".to_string(),
            args: vec![var, range_start, range_end],
            body: Some(body),
            else_body: None,
            span: Span::new(start, body_end),
        })
    }

    // `@if cond { body } [@else { body }]`.
    fn parse_if_meta(&mut self, start: usize) -> Result<MetaStatement, ParseError> {
        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let condition = self.parse_expr();

        self.restrict_brace_construction = outer_restriction;

        let condition = condition?;

        self.skip_newlines();

        let (body, mut end) = self.parse_statement_block("unterminated `@if` body")?;

        let else_body = if self.at_else_meta() {
            self.advance(); // `@`
            self.advance(); // `else`
            self.skip_newlines();

            let (else_body, else_end) =
                self.parse_statement_block("unterminated `@else` body")?;

            end = else_end;

            Some(else_body)
        } else {
            None
        };

        self.consume_trailing_newline();

        Ok(MetaStatement {
            name: "if".to_string(),
            args: vec![condition],
            body: Some(body),
            else_body,
            span: Span::new(start, end),
        })
    }

    // Parses `{ stmt* }`, given the opening `{` hasn't been consumed yet.
    // Returns the body and the closing `}`'s span end; callers decide what,
    // if anything, follows (a trailing newline for a macro/struct body, an
    // `@else` for `@if`).
    fn parse_statement_block(
        &mut self,
        unterminated_message: &str,
    ) -> Result<(Vec<Statement>, usize), ParseError> {
        self.expect_simple(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut body = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(unterminated_message, self.current().span));
            }

            body.push(self.parse_statement()?);
            self.skip_newlines();
        }

        let closing = self.current().clone();
        self.expect_simple(TokenKind::RBrace)?;

        Ok((body, closing.span.end))
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
        self.skip_newlines();

        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            params.push(self.parse_macro_parameter()?);
            self.skip_newlines();

            while self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                params.push(self.parse_macro_parameter()?);
                self.skip_newlines();
            }
        }

        self.expect_simple(TokenKind::RParen)?;

        let facets = self.parse_facet_list(true, true)?;
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

        let (body, body_end) = self.parse_statement_block("unterminated macro body")?;

        self.consume_trailing_newline();

        Ok(MacroDeclaration {
            name,
            is_pub,
            params,
            return_ty,
            facets,
            body,
            span: Span::new(start, body_end),
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
