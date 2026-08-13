use crate::ast::StructDeclaration;

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
                    self.parse_struct_declaration()?
                ))
            }

            TokenKind::Type => {
                Ok(Statement::TypeAlias(
                    self.parse_type_alias()?
                ))
            }

            TokenKind::Const => {
                Ok(Statement::Const(
                    self.parse_const_declaration()?
                ))
            }

            TokenKind::Identifier(_) => {
                if self.check_next(&TokenKind::Colon) {
                    Ok(Statement::Label(
                        self.parse_label()?
                    ))
                } else {
                    Ok(Statement::Invocation(
                        self.parse_invocation()?
                    ))
                }
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

        Ok(ImportStatement {
            module,
            items,
            span: Span::new(start, end),
        })
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
            ty,
            value,
            span: Span::new(start, end),
        })
    }

    // =============
    // types
    // =============

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        let start = self.current().span.start;

        let mut path = Vec::new();

        path.push(self.expect_identifier()?);

        while self.check(&TokenKind::Dot) {
            self.advance();
            path.push(self.expect_identifier()?);
        }

        let named_end = self.previous().span.end;

        let mut ty = TypeExpr::Named {
            path,
            span: Span::new(start, named_end),
        };

        if self.check(&TokenKind::Less) {
            self.advance();

            let mut args = Vec::new();

            if self.check(&TokenKind::Greater) {
                return Err(ParseError::new(
                    "generic argument list cannot be empty",
                    self.current().span,
                ));
            }

            let signature = ty.name().and_then(|name| {
                self.generic_signatures.get(name)
            }).cloned();

            let mut index = 0;

            loop {
                let expected_kind = signature
                    .as_ref()
                    .and_then(|kinds| kinds.get(index))
                    .copied();

                args.push(self.parse_type_argument(expected_kind)?);
                index += 1;

                if self.check(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }

                break;
            }

            let closing = self.current().clone();

            self.expect_simple(TokenKind::Greater)?;

            ty = TypeExpr::Apply {
                base: Box::new(ty),
                args,
                span: Span::new(start, closing.span.end),
            };
        }

        Ok(ty)
    }

    fn parse_type_argument(
        &mut self,
        expected_kind: Option<GenericParamKind>,
    ) -> Result<TypeArgument, ParseError> {
        // min_bp above Less/Greater's binding power so the closing '>' of
        // the generic argument list isn't consumed as a comparison operator
        match expected_kind {
            Some(GenericParamKind::Const) => {
                let expr = self.parse_expr_bp(51)?;
                Ok(TypeArgument::Const(expr))
            }

            Some(GenericParamKind::Type) => {
                let ty = self.parse_type_expr()?;
                Ok(TypeArgument::Type(ty))
            }

            // The callee's signature isn't known (builtin type, or a
            // forward reference to a declaration later in the source), so
            // try guessing from the leading token.
            None => match &self.current().kind {
                TokenKind::Integer(_)
                | TokenKind::Minus
                | TokenKind::LParen
                | TokenKind::Bang
                | TokenKind::Tilde => {
                    let expr = self.parse_expr_bp(51)?;
                    Ok(TypeArgument::Const(expr))
                }

                TokenKind::Identifier(_) => {
                    let ty = self.parse_type_expr()?;
                    Ok(TypeArgument::Type(ty))
                }

                other => Err(ParseError::new(
                    format!("expected type argument, found {other:?}"),
                    self.current().span,
                )),
            },
        }
    }

    fn parse_generic_params(
        &mut self,
    ) -> Result<Vec<GenericParameter>, ParseError> {
        let mut parameters = Vec::new();

        if !self.check(&TokenKind::Less) {
            return Ok(parameters);
        }

        self.advance();

        if self.check(&TokenKind::Greater) {
            return Err(ParseError::new(
                "generic parameter list cannot be empty",
                self.current().span,
            ));
        }

        loop {
            let start = self.current().span.start;

            match &self.current().kind {
                TokenKind::Const => {
                    self.advance();

                    let name = self.expect_identifier()?;

                    self.expect_simple(TokenKind::Colon)?;

                    let ty = self.parse_type_expr()?;

                    let end = ty.span().end;

                    parameters.push(GenericParameter::Const {
                        name,
                        ty,
                        span: Span::new(start, end),
                    });
                }

                TokenKind::Identifier(_) => {
                    let name = self.expect_identifier()?;

                    let end = self.previous().span.end;

                    parameters.push(GenericParameter::Type {
                        name,
                        span: Span::new(start, end),
                    });
                }

                other => {
                    return Err(ParseError::new(
                        format!(
                            "expected generic parameter, found {other:?}"
                        ),
                        self.current().span,
                    ));
                }
            }

            if self.check(&TokenKind::Comma) {
                self.advance();
                continue;
            }

            break;
        }

        self.expect_simple(TokenKind::Greater)?;

        Ok(parameters)
    }

    // =============
    // structs
    // =============
    
    fn parse_struct_declaration(
        &mut self,
    ) -> Result<StructDeclaration, ParseError> {
        let start = self.current().span.start;

        self.expect_simple(TokenKind::Struct)?;

        let name = self.expect_identifier()?;

        let generic_params = self.parse_generic_params()?;

        self.register_generic_signature(&name, &generic_params);

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
        }

        let closing = self.current().clone();

        self.expect_simple(TokenKind::RBrace)?;

        if self.check(&TokenKind::Newline) {
            self.advance();
        }

        Ok(StructDeclaration {
            name,
            generic_params,
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

        if self.check(&TokenKind::Newline) {
            self.advance();
        } else if !self.check(&TokenKind::RBrace) {
            return Err(ParseError::new(
                "expected newline or '}' after struct field",
                self.current().span,
            ));
        }

        Ok(StructField {
            name,
            ty,
            span: Span::new(start, end),
        })
    }

    // =============
    // type aliases
    // =============

    fn parse_type_alias(
        &mut self,
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
            generic_params: generic_params,
            ty: target,
            span: Span::new(start, end),
        })
    }
}
