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
            "match" => self.parse_match_meta(start),

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
                    match_arms: Vec::new(),
                    span: Span::new(start, end),
                })
            }
        }
    }

    // `@for name in source { body }` — the loop variable and iteration
    // source are packed positionally into `args` (`[Identifier, source]`)
    // rather than given their own `MetaStatement` fields, the same way
    // `@assert`'s `[condition, message]` already overloads `args`. `source`
    // is evaluated to a `Value::Struct` and its pub fields walked in order
    // (see `resolver::generated::eval_for_source`) — `start..end` is just
    // the common case, `Expr::Range` sugar for a synthesized struct.
    fn parse_for_meta(&mut self, start: usize) -> Result<MetaStatement, ParseError> {
        let var_token = self.current().clone();
        let var_name = self.expect_identifier()?;
        let var = Expr::Identifier { name: var_name, span: var_token.span };

        self.expect_keyword("in")?;

        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let result = self.parse_expr();

        self.restrict_brace_construction = outer_restriction;

        let source = result?;

        self.skip_newlines();

        let (body, body_end) =
            self.parse_statement_block("unterminated `@for` body")?;

        self.consume_trailing_newline();

        Ok(MetaStatement {
            name: "for".to_string(),
            args: vec![var, source],
            body: Some(body),
            else_body: None,
            match_arms: Vec::new(),
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

        // Both `} @else {` and a newline-separated `}\n@else {` are valid.
        self.skip_newlines();

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
            match_arms: Vec::new(),
            span: Span::new(start, end),
        })
    }

    // `@match value { pattern => { body }, _ => { fallback } }`.
    fn parse_match_meta(&mut self, start: usize) -> Result<MetaStatement, ParseError> {
        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;
        let scrutinee = self.parse_expr();
        self.restrict_brace_construction = outer_restriction;
        let scrutinee = scrutinee?;

        self.skip_newlines();
        self.expect_simple(TokenKind::LBrace)?;
        self.skip_newlines();

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(ParseError::new(
                    "unterminated `@match` body",
                    self.current().span,
                ));
            }

            let arm_start = self.current().span.start;
            let pattern =
                if matches!(&self.current().kind, TokenKind::Identifier(name) if name == "_") {
                    self.advance();
                    None
                } else {
                    Some(self.parse_expr()?)
                };
            self.expect_simple(TokenKind::FatArrow)?;
            self.skip_newlines();
            let (body, arm_end) = self.parse_statement_block("unterminated `@match` arm")?;
            arms.push(crate::ast::MatchArm {
                pattern,
                body,
                span: Span::new(arm_start, arm_end),
            });

            if self.check(&TokenKind::Comma) {
                self.advance();
            }
            self.skip_newlines();
        }

        let end = self.current().span.end;
        self.expect_simple(TokenKind::RBrace)?;
        self.consume_trailing_newline();

        Ok(MetaStatement {
            name: "match".to_string(),
            args: vec![scrutinee],
            body: None,
            else_body: None,
            match_arms: arms,
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

        if let Some(param) = params
            .windows(2)
            .find(|pair| pair[0].default.is_some() && pair[1].default.is_none())
            .map(|pair| &pair[1])
        {
            return Err(ParseError::new(
                "required macro parameters cannot follow parameters with defaults",
                param.span,
            ));
        }

        let return_ty = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let facets = self.parse_facet_list(true)?;

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

        let default = if self.check(&TokenKind::Equal) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end = default.as_ref().map_or_else(|| ty.span().end, |value| value.span().end);

        Ok(MacroParameter {
            name,
            ty,
            default,
            span: Span::new(start, end),
        })
    }
}
