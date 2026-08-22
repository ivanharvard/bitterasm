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

                if self.check_next(&TokenKind::Colon) {
                    Ok(Statement::Label(
                        self.parse_label()?
                    ))
                } else if let Some(pattern) = self.macro_syntaxes.get(&name).cloned() {
                    Ok(Statement::Invocation(
                        self.parse_invocation_via_syntax(&name, &pattern)?
                    ))
                } else {
                    Ok(Statement::Invocation(
                        self.parse_invocation()?
                    ))
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

        let name = self.expect_identifier()?;

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
}
