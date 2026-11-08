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
                    self.parse_from_import(false)?
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

            TokenKind::Section => {
                Ok(Statement::Section(
                    self.parse_section(false)?
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

                    TokenKind::From => Ok(Statement::Import(
                        self.parse_from_import(true)?
                    )),

                    TokenKind::Section => Ok(Statement::Section(
                        self.parse_section(true)?
                    )),

                    TokenKind::Identifier(_) if self.at_space_declaration() => Ok(Statement::Const(
                        self.parse_space_declaration(true)?
                    )),

                    TokenKind::Identifier(_) if self.check_next(&TokenKind::Colon) => {
                        Ok(Statement::Label(
                            self.parse_label(true)?
                        ))
                    }

                    TokenKind::Dot if self.at_dotted_label() => {
                        Ok(Statement::Label(
                            self.parse_label(true)?
                        ))
                    }

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
                } else if self.at_space_declaration() {
                    Ok(Statement::Const(self.parse_space_declaration(false)?))
                } else if self.check_next(&TokenKind::Colon) {
                    Ok(Statement::Label(
                        self.parse_label(false)?
                    ))
                } else {
                    self.parse_invocation_statement(&name)
                }
            }

            TokenKind::Dot if self.at_dotted_label() => {
                Ok(Statement::Label(
                    self.parse_label(false)?
                ))
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

    // `pub` (already consumed, when `is_pub`) starts the statement's span.
    fn parse_from_import(&mut self, is_pub: bool) -> Result<ImportStatement, ParseError> {
        let start = if is_pub { self.previous().span.start } else { self.current().span.start };

        self.expect_simple(TokenKind::From)?;

        let module = self.parse_module_path()?;

        self.expect_simple(TokenKind::Import)?;

        let items = if self.check(&TokenKind::Star) {
            self.advance();
            ImportItems::All
        } else {
            let mut names = Vec::new();

            // A section is imported by its dotted name: `import .text`.
            names.push(self.parse_imported_name()?);

            while self.check(&TokenKind::Comma) {
                self.advance();
                names.push(self.parse_imported_name()?);
            }

            ImportItems::Names(names)
        };

        let end = self.statement_end()?;

        let import = ImportStatement {
            is_pub,
            module,
            items,
            span: Span::new(start, end),
        };

        self.imports.push(import.clone());

        Ok(import)
    }

    fn parse_imported_name(&mut self) -> Result<String, ParseError> {
        if self.check(&TokenKind::Dot) {
            self.advance();
            let mut name = format!(".{}", self.expect_identifier()?);
            while self.check(&TokenKind::Dot) {
                self.advance();
                name.push('.');
                name.push_str(&self.expect_identifier()?);
            }
            Ok(name)
        } else {
            self.expect_identifier()
        }
    }

    fn parse_module_path(&mut self) -> Result<ModulePath, ParseError> {
        let start = self.current().span.start;

        let mut relative_level = 0;

        // Each leading `.` goes up a directory. The lexer reads runs of
        // dots as range and wildcard tokens, so count those as their dots.
        loop {
            relative_level += match self.current().kind {
                TokenKind::Dot => 1,
                TokenKind::DotDot => 2,
                TokenKind::Ellipsis => 3,
                _ => break,
            };
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

    // A label name may carry a leading `.` (`.loop:`), matching the
    // NASM/GAS local-label spelling; the dot is kept as part of the name,
    // and references spell it the same way (`jmp .loop`).
    fn parse_label(&mut self, is_pub: bool) -> Result<Label, ParseError> {
        let start = self.current().span.start;

        let name = if self.at_dot_identifier() {
            self.expect_dotted_name()
        } else {
            self.expect_identifier()?
        };

        self.expect_simple(TokenKind::Colon)?;

        let end = self.statement_end()?;

        Ok(Label {
            name,
            is_pub,
            doc: None,
            span: Span::new(start, end),
        })
    }

    // =============
    // sections
    // =============

    // `section <name>`, where `<name>` is a dotted identifier — an optional
    // leading `.` (matching NASM/ELF convention, e.g. `.text`, `.rodata`)
    // followed by one or more identifier segments joined by `.` (e.g.
    // `.rela.text`). Stored as a single literal string, dots included; no
    // further character restrictions beyond what an identifier already
    // allows.
    fn parse_section(&mut self, is_pub: bool) -> Result<Section, ParseError> {
        let start = if is_pub { self.previous().span.start } else { self.current().span.start };

        self.expect_simple(TokenKind::Section)?;

        let mut name = String::new();

        if self.check(&TokenKind::Dot) {
            self.advance();
            name.push('.');
        }

        name.push_str(&self.expect_identifier()?);

        while self.check(&TokenKind::Dot) {
            self.advance();
            name.push('.');
            name.push_str(&self.expect_identifier()?);
        }

        // `section .text in flash | progbits` declares the section: which
        // space it's in, and its facets. The space stops at `|`, so a
        // facet can follow it on the same line.
        let space = if self.check(&TokenKind::In) {
            self.advance();
            Some(self.parse_expr_bp_until(0, Some(&TokenKind::Pipe))?)
        } else {
            None
        };
        let facets = self.parse_applied_facets()?;

        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!("unexpected token after section name: {:?}", self.current().kind),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        Ok(Section {
            name,
            space,
            facets,
            is_pub,
            imported: false,
            span: Span::new(start, end),
            doc: None,
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
            kind: ConstKind::Const,
            doc: None,
            span: Span::new(start, end),
        })
    }

    // `space name = value`, then its facets: `| unit<bits<16>>`, each on
    // its own line or after the value. `space` is only a keyword here, so
    // it stays usable as an ordinary name everywhere else.
    fn parse_space_declaration(&mut self, is_pub: bool) -> Result<ConstDeclaration, ParseError> {
        let start = if is_pub { self.previous().span.start } else { self.current().span.start };

        self.advance(); // `space`

        let (name, _) = self.parse_spliced_name()?;

        self.expect_simple(TokenKind::Equal)?;

        // Stops at `|`, so `space flash = stream | unit<bits<16>>` is the
        // value followed by a facet, not a bitwise or.
        let value = self.parse_expr_bp_until(0, Some(&TokenKind::Pipe))?;
        let facets = self.parse_applied_facets()?;

        if !self.at_statement_end() {
            return Err(ParseError::new(
                format!("unexpected token after space declaration: {:?}", self.current().kind),
                self.current().span,
            ));
        }

        let end = self.statement_end()?;

        Ok(ConstDeclaration {
            name,
            is_pub,
            ty: None,
            value,
            kind: ConstKind::Space { facets },
            doc: None,
            span: Span::new(start, end),
        })
    }

    // `space name =` at the start of a statement.
    pub(super) fn at_space_declaration(&self) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(word) if word == "space")
            && matches!(self.tokens.get(self.pos + 1).map(|token| &token.kind), Some(TokenKind::Identifier(_)))
            && matches!(self.tokens.get(self.pos + 2).map(|token| &token.kind), Some(TokenKind::Equal))
    }

    // Facets applied to a space or section: `| name`, `| name<T>` or
    // `| name arg, ...`, each a call to the macro `name` with the subject
    // left out. A facet may continue on the next line.
    pub(super) fn parse_applied_facets(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut facets = Vec::new();

        loop {
            let before_newlines = self.pos;
            self.skip_newlines();
            if !self.check(&TokenKind::Pipe) {
                self.pos = before_newlines;
                break;
            }
            let pipe = self.advance().span;

            let name_token = self.current().clone();
            let name = self.expect_identifier()?;
            let callee = Expr::Identifier { name: name.clone(), span: name_token.span };

            let generic_args = if self.check(&TokenKind::Less) {
                self.parse_generic_argument_list(Some(&name), name_token.span.start)?.0
            } else {
                Vec::new()
            };

            // Like a built-in facet's expression, each argument stops at
            // the next `|`; parentheses allow a bitwise or inside one.
            let mut arguments = Vec::new();
            if !self.at_statement_end() && !self.check(&TokenKind::Pipe) {
                loop {
                    let value = self.parse_expr_bp_until(0, Some(&TokenKind::Pipe))?;
                    arguments.push(CallArgument { name: None, span: value.span(), value });
                    if !self.check(&TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
            }

            let end = self.previous().span.end;
            facets.push(Expr::Call {
                callee: Box::new(callee),
                generic_args,
                arguments,
                span: Span::new(pipe.start, end),
            });
        }

        Ok(facets)
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
        candidates.extend(
            self.unanchored_syntaxes
                .iter()
                .filter(|(_, pattern)| !crate::facets::syntax::is_operand_pattern(pattern))
                .cloned(),
        );

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

        Ok(SyntaxOverrideStatement { name, pattern, doc: None, span })
    }
}
