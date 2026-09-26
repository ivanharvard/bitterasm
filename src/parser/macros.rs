use crate::ast::{literal_name, FacetPayload, FoldBinding};

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
            "fold" => {
                let fold = self.parse_fold_meta(start)?;
                self.consume_trailing_newline();
                Ok(fold)
            }
            "next" => {
                let (value, updates) = self.parse_next_values()?;
                let end = self.statement_end()?;

                Ok(MetaStatement {
                    name: "next".to_string(),
                    args: value.into_iter().collect(),
                    body: None,
                    else_body: None,
                    match_arms: Vec::new(),
                    bindings: updates,
                    span: Span::new(start, end),
                })
            }
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
                    bindings: Vec::new(),
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
        let meta = self.parse_for_header_and_body(start)?;
        self.consume_trailing_newline();
        Ok(meta)
    }

    // `var in source { body }`, with `@for` already consumed.
    fn parse_for_header_and_body(&mut self, start: usize) -> Result<MetaStatement, ParseError> {
        let var_token = self.current().clone();
        let var_name = self.expect_identifier()?;
        let var = Expr::Identifier { name: var_name, span: var_token.span };

        self.expect_simple(TokenKind::In)?;

        let outer_restriction = self.restrict_brace_construction;
        self.restrict_brace_construction = true;

        let result = self.parse_expr();

        self.restrict_brace_construction = outer_restriction;

        let source = result?;

        self.skip_newlines();

        let (body, body_end) =
            self.parse_statement_block("unterminated `@for` body")?;

        Ok(MetaStatement {
            name: "for".to_string(),
            args: vec![var, source],
            body: Some(body),
            else_body: None,
            match_arms: Vec::new(),
            bindings: Vec::new(),
            span: Span::new(start, body_end),
        })
    }

    // `@fold acc = init, ... @for var in source { body }`, with the leading
    // `@fold` already consumed. Parsed to the same `[var, source]` + `body`
    // shape as `@for`, plus the accumulators in `bindings`. Leaves any
    // trailing newline alone: in expression position (`const x = @fold
    // ...`) the enclosing statement owns it.
    pub(super) fn parse_fold_meta(&mut self, start: usize) -> Result<MetaStatement, ParseError> {
        let accumulators = self.parse_fold_accumulators()?;

        let for_start = self.current().span.start;
        self.expect_simple(TokenKind::At)?;
        let for_token = self.current().clone();
        if self.expect_identifier()? != "for" {
            return Err(ParseError::new("expected `@for` after `@fold`'s accumulators", for_token.span));
        }

        let for_meta = self.parse_for_header_and_body(for_start)?;

        Ok(MetaStatement {
            name: "fold".to_string(),
            span: Span::new(start, for_meta.span.end),
            bindings: accumulators,
            ..for_meta
        })
    }

    // `acc = init, acc = init` up to the `@for` that follows. At least one.
    pub(super) fn parse_fold_accumulators(&mut self) -> Result<Vec<FoldBinding>, ParseError> {
        let mut accumulators = Vec::new();

        loop {
            accumulators.push(self.parse_fold_binding()?);

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }

        self.skip_newlines();
        Ok(accumulators)
    }

    fn parse_fold_binding(&mut self) -> Result<FoldBinding, ParseError> {
        let name_token = self.current().clone();
        let name = self.expect_identifier()?;
        self.expect_simple(TokenKind::Equal)?;
        let value = self.parse_expr()?;

        Ok(FoldBinding { name, span: Span::new(name_token.span.start, value.span().end), value })
    }

    // What follows `@next`: nothing (every accumulator unchanged), one
    // positional value, or `acc = value, ...`. Stops before the statement
    // or item terminator, which the caller consumes.
    pub(super) fn parse_next_values(&mut self) -> Result<(Option<Expr>, Vec<FoldBinding>), ParseError> {
        if self.at_statement_end() || self.check(&TokenKind::Comma) {
            return Ok((None, Vec::new()));
        }

        let named = matches!(self.current().kind, TokenKind::Identifier(_))
            && self.tokens.get(self.pos + 1).is_some_and(|next| next.kind == TokenKind::Equal);

        if !named {
            let value = self.parse_expr()?;
            if self.check(&TokenKind::Comma) && self.tokens.get(self.pos + 1).is_some_and(|next| {
                !matches!(next.kind, TokenKind::Newline | TokenKind::RBrace)
            }) {
                return Err(ParseError::new(
                    "a positional `@next` takes one value — name each accumulator \
                     (`@next a = ..., b = ...`) to update several",
                    self.current().span,
                ));
            }
            return Ok((Some(value), Vec::new()));
        }

        // Another `acc = value` may follow a comma on the next line. In a
        // construction, a comma followed by anything else ends the item.
        let mut updates = vec![self.parse_fold_binding()?];
        while self.check(&TokenKind::Comma) {
            let mut after = self.pos + 1;
            while self.tokens.get(after).is_some_and(|token| token.kind == TokenKind::Newline) {
                after += 1;
            }
            let continues = matches!(self.tokens.get(after).map(|t| &t.kind), Some(TokenKind::Identifier(_)))
                && self.tokens.get(after + 1).is_some_and(|next| next.kind == TokenKind::Equal);
            if !continues {
                break;
            }
            self.advance();
            self.skip_newlines();
            updates.push(self.parse_fold_binding()?);
        }

        Ok((None, updates))
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
            bindings: Vec::new(),
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
            bindings: Vec::new(),
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

        // A standalone `syntax name(...) = { ... }` override fires
        // unconditionally at parse time (see `parse_syntax_override`'s
        // doc) — it has no sound meaning nested inside any block that
        // might not even run (a macro body, a hook, `@for`/`@if`), so
        // `block_depth` makes `at_syntax_override_start` refuse to
        // recognize it there at all, falling back to an ordinary
        // (and almost certainly invalid) invocation parse instead.
        self.block_depth += 1;

        let mut body = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.at_eof() {
                self.block_depth -= 1;
                return Err(ParseError::new(unterminated_message, self.current().span));
            }

            body.push(match self.parse_statement() {
                Ok(statement) => statement,
                Err(error) => {
                    self.block_depth -= 1;
                    return Err(error);
                }
            });
            self.skip_newlines();
        }

        self.block_depth -= 1;

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

        let (name, _) = self.parse_spliced_name()?;

        let generic_params = self.parse_generic_params()?;

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
            let FacetPayload::Pattern(tokens) = &facet.payload else {
                unreachable!("`syntax`'s payload shape is `Pattern`, enforced by `parse_facet`")
            };

            // A computed (spliced) macro name only ever occurs generated
            // from inside another macro's body, where a static `syntax`
            // pattern registration (a parser-only, pre-evaluation concern)
            // doesn't apply — skip it rather than erroring.
            if let Some(literal) = literal_name(&name) {
                let param_names: Vec<String> = params.iter().map(|param| param.name.clone()).collect();

                let pattern = crate::facets::syntax::parse_pattern(tokens.clone(), &param_names)
                    .map_err(|message| ParseError::new(message, facet.span))?;

                self.register_macro_syntax(&literal, pattern);
            }
        }

        self.skip_newlines();

        let (body, body_end) = self.parse_statement_block("unterminated macro body")?;

        self.consume_trailing_newline();

        Ok(MacroDeclaration {
            name,
            is_pub,
            generic_params,
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
