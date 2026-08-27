use super::invocation_syntax::SyntaxMatch;
use super::*;

impl Parser {
    // ===============
    // statement
    // ===============

    pub(super) fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.current().kind {
            TokenKind::From => {
                Ok(Statement::Import(
                    self.parse_from_import()?
                ))
            }

            TokenKind::Struct => {
                Ok(Statement::Struct(
                    self.parse_struct_declaration(false)?
                ))
            }

            TokenKind::Enum => {
                Ok(Statement::Enum(
                    self.parse_enum_declaration(false)?
                ))
            }

            TokenKind::Type => {
                Ok(Statement::TypeAlias(
                    self.parse_type_alias(false)?
                ))
            }

            TokenKind::Const => {
                Ok(Statement::Const(
                    self.parse_const_declaration(false)?
                ))
            }

            TokenKind::Macro => {
                Ok(Statement::Macro(
                    self.parse_macro_declaration(false)?
                ))
            }

            TokenKind::Pub => {
                self.advance();

                match &self.current().kind {
                    TokenKind::Struct => Ok(Statement::Struct(
                        self.parse_struct_declaration(true)?
                    )),

                    TokenKind::Enum => Ok(Statement::Enum(
                        self.parse_enum_declaration(true)?
                    )),

                    TokenKind::Type => Ok(Statement::TypeAlias(
                        self.parse_type_alias(true)?
                    )),

                    TokenKind::Const => Ok(Statement::Const(
                        self.parse_const_declaration(true)?
                    )),

                    TokenKind::Macro => Ok(Statement::Macro(
                        self.parse_macro_declaration(true)?
                    )),

                    other => Err(ParseError::new(
                        format!("expected declaration after `pub`, found {other:?}"),
                        self.current().span,
                    )),
                }
            }

            TokenKind::Identifier(name) => {
                let name = name.clone();

                if name == "syntax" && self.at_syntax_override_start() {
                    Ok(Statement::SyntaxOverride(self.parse_syntax_override()?))
                } else if self.check_next(&TokenKind::Colon) {
                    Ok(Statement::Label(
                        self.parse_label()?
                    ))
                } else {
                    self.parse_invocation_statement(&name)
                }
            }

            TokenKind::At => {
                Ok(Statement::Meta(
                    self.parse_meta_statement()?
                ))
            }

            other => Err(ParseError::new(
                format!("expected statement, found {other:?}"),
                self.current().span,
            )),
        }
    }

    // ===============
    // imports
    // ===============

    fn parse_from_import(&mut self) -> Result<ImportStatement, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::From)?;

        let module = self.parse_module_path()?;

        self.expect_simple(TokenKind::Import)?;

        let items = if self.check(&TokenKind::Star) {
            self.advance();
            ImportItems::All
        } else {
            let mut names = Vec::new();

            names.push(self.expect_identifier()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                names.push(self.expect_identifier()?);
            }

            ImportItems::Names(names)
        };

        let end = self.statement_end()?;

        let import = ImportStatement {
            module,
            items,
            span: Span::new(start, end),
        };

        self.imports.push(import.clone());

        Ok(import)
    }

    fn parse_module_path(&mut self) -> Result<ModulePath, ParseError> {
        let start = self.current().span.start;

        let mut relative_level = 0;

        // unresolved atm
        while self.check(&TokenKind::Dot) {
            relative_level += 1;
            self.advance();
        }

        let mut segments = Vec::new();

        segments.push(self.expect_identifier()?);

        while self.check(&TokenKind::Dot) {
            self.advance();
            segments.push(self.expect_identifier()?);
        }

        let end = self.previous().span.end;

        Ok(ModulePath {
            segments,
            relative_level,
            span: Span::new(start, end),
        })
    }

    // =============
    // labels
    // =============

    fn parse_label(&mut self) -> Result<Label, ParseError> {
        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        self.expect_simple(TokenKind::Colon)?;

        let end = self.statement_end()?;

        Ok(Label {
            name,
            span: Span::new(start, end),
        })
    }

    // =============
    // invocations
    // =============

    fn parse_invocation(&mut self) -> Result<Invocation, ParseError> {
        let start = self.current().span.start;

        let name = self.expect_identifier()?;

        let mut operands = Vec::new();

        // if we haven't reached the end of the statement there is at least one operand
        if !self.at_statement_end() {
            operands.push(self.parse_expr()?);

            while self.check(&TokenKind::Comma) {
                self.advance();

                if self.at_statement_end() {
                    return Err(ParseError::new(
                        "expected operand after comma",
                        self.current().span,
                    ));
                }

                operands.push(self.parse_expr()?);
            }
        }

        // if theres something other than a newline or EOF here then we failed
        // to consume the whole invocation
        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!(
                    "unexpected token in invocation: {:?}",
                    self.current().kind
                ),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        Ok(Invocation {
            name,
            operands,
            span: Span::new(start, end),
        })
    }

    // =============
    // const
    // =============

    fn parse_const_declaration(
        &mut self,
        is_pub: bool,
    ) -> Result<ConstDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Const)?;

        let (name, _) = self.parse_spliced_name()?;

        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect_simple(TokenKind::Equal)?;

        let value = self.parse_expr()?;

        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!(
                    "unexpected token after constant value: {:?}",
                    self.current().kind
                ),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        Ok(ConstDeclaration {
            name,
            is_pub,
            ty,
            value,
            span: Span::new(start, end),
        })
    }

    // `name` (already known to not be a label) either invokes some macro
    // via custom syntax or falls back to default `name arg, arg, ...`
    // syntax. Candidates come from two places: patterns anchored to `name`
    // itself (cheap, keyed lookup) and every unanchored pattern in the
    // program (tried unconditionally, since nothing about `name` — the
    // whole point of an unanchored pattern — says whether one of those is
    // for it). If `name` itself is anchored by *something*, this identifier
    // is "claimed": failing to match any candidate is a hard error, same
    // as today, not a silent fall-through to default syntax (matches how
    // an anchored macro's custom syntax already fully replaces its default
    // form rather than sitting alongside it). An unclaimed name has no
    // such exclusivity — no candidate matching just means none of the
    // active dialects have anything to say about this statement, so
    // default syntax applies exactly as if no custom syntax existed at
    // all.
    fn parse_invocation_statement(&mut self, name: &str) -> Result<Statement, ParseError> {
        let claimed = self.macro_syntaxes.contains_key(name);

        let mut candidates: Vec<(String, SyntaxPattern)> = self
            .macro_syntaxes
            .get(name)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|pattern| (name.to_string(), pattern))
            .collect();
        candidates.extend(self.unanchored_syntaxes.iter().cloned());

        if candidates.is_empty() {
            return Ok(Statement::Invocation(self.parse_invocation()?));
        }

        match self.parse_invocation_via_syntax_candidates(&candidates)? {
            SyntaxMatch::Matched(invocation) => Ok(Statement::Invocation(invocation)),

            SyntaxMatch::NoMatch { best_error } if claimed => Err(best_error.unwrap_or_else(|| {
                ParseError::new(format!("no syntax pattern registered for `{name}`"), self.current().span)
            })),

            SyntaxMatch::NoMatch { .. } => Ok(Statement::Invocation(self.parse_invocation()?)),
        }
    }

    // ===============
    // syntax overrides
    // ===============

    // Pure lookahead, no tokens consumed: is the current `syntax` identifier
    // actually the start of `syntax name(...) = { ... }`, as opposed to an
    // ordinary invocation of some macro that happens to be named `syntax`
    // (`syntax foo, bar`) or a custom-syntax call site of one? Scans past a
    // balanced `(...)` to find the `=` and `{` that only this construct
    // ever has right there — a plain invocation's operands, even a call
    // expression like `syntax foo(a, b)`, are never followed by `= {`.
    fn at_syntax_override_start(&self) -> bool {
        if self.block_depth > 0 {
            return false;
        }

        let mut i = self.pos + 1;

        if !matches!(self.tokens.get(i).map(|token| &token.kind), Some(TokenKind::Identifier(_))) {
            return false;
        }
        i += 1;

        if !matches!(self.tokens.get(i).map(|token| &token.kind), Some(TokenKind::LParen)) {
            return false;
        }
        i += 1;

        let mut depth: usize = 1;
        loop {
            match self.tokens.get(i).map(|token| &token.kind) {
                Some(TokenKind::LParen) => {
                    depth += 1;
                    i += 1;
                }

                Some(TokenKind::RParen) => {
                    depth -= 1;
                    i += 1;
                    if depth == 0 {
                        break;
                    }
                }

                Some(TokenKind::Eof) | None => return false,

                _ => i += 1,
            }
        }

        matches!(self.tokens.get(i).map(|token| &token.kind), Some(TokenKind::Equal))
            && matches!(self.tokens.get(i + 1).map(|token| &token.kind), Some(TokenKind::LBrace))
    }

    fn parse_syntax_override(&mut self) -> Result<SyntaxOverrideStatement, ParseError> {
        let start = self.current().span.start;

        self.advance(); // `syntax`

        let name = self.expect_identifier()?;

        self.expect_simple(TokenKind::LParen)?;
        self.skip_newlines();

        let mut params = Vec::new();

        if !self.check(&TokenKind::RParen) {
            params.push(self.expect_identifier()?);
            self.skip_newlines();

            while self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                params.push(self.expect_identifier()?);
                self.skip_newlines();
            }
        }

        self.expect_simple(TokenKind::RParen)?;
        self.skip_newlines();
        self.expect_simple(TokenKind::Equal)?;
        self.skip_newlines();

        let tokens = self.parse_pattern_block("syntax override pattern")?;
        let end = self.previous().span.end;
        let span = Span::new(start, end);

        let pattern = crate::facets::syntax::parse_pattern(tokens, &params)
            .map_err(|message| ParseError::new(message, span))?;

        self.register_syntax_override(&name, pattern.clone(), span)?;

        Ok(SyntaxOverrideStatement { name, pattern, span })
    }
}
