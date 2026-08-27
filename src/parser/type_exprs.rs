//! Grammar for type expressions (`Reg`, `foo.Reg`, `bits<8>`, `Reg<T>`) and
//! generic parameter lists, shared by struct/alias declarations
//! ([`super::declarations`]), macro parameters ([`super::macros`]), and
//! const type annotations ([`super::statements`]).

use super::*;

impl Parser {
    pub(super) fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
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
            let (args, span) = self.parse_generic_argument_list(ty.name(), start)?;

            ty = TypeExpr::Apply {
                base: Box::new(ty),
                args,
                span,
            };
        }

        Ok(ty)
    }

    // Parses a `<Arg, Arg, ...>` generic argument list, given the parser is
    // sitting on the opening `<` (not yet consumed). `name` is the callee's
    // own name (ignoring any module-path qualification), used to look up
    // its generic signature the same way type position always has; shared
    // with expression-position brace construction
    // (`expressions::parse_expr_bp_until`), which needs the identical
    // const-vs-type disambiguation for `Array<u8, N> { ... }`.
    pub(super) fn parse_generic_argument_list(
        &mut self,
        name: Option<&str>,
        start: usize,
    ) -> Result<(Vec<TypeArgument>, Span), ParseError> {
        self.advance();
        self.skip_newlines();

        let mut args = Vec::new();

        if self.check(&TokenKind::Greater) {
            return Err(ParseError::new(
                "generic argument list cannot be empty",
                self.current().span,
            ));
        }

        let signature = name.and_then(|name| self.generic_signatures.get(name)).cloned();

        let mut index = 0;

        loop {
            let expected_kind = signature
                .as_ref()
                .and_then(|kinds| kinds.get(index))
                .copied();

            args.push(self.parse_type_argument(expected_kind)?);
            index += 1;
            self.skip_newlines();

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                continue;
            }

            break;
        }

        let closing = self.expect_generic_close()?;

        Ok((args, Span::new(start, closing.span.end)))
    }

    fn parse_type_argument(
        &mut self,
        expected_kind: Option<GenericParamKind>,
    ) -> Result<TypeArgument, ParseError> {
        if self.check(&TokenKind::Ellipsis) {
            return Ok(TypeArgument::Wildcard(self.advance().span));
        }
        match expected_kind {
            Some(GenericParamKind::Const) => {
                Ok(TypeArgument::Const(self.parse_const_arg_expr()?))
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
                    Ok(TypeArgument::Const(self.parse_const_arg_expr()?))
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

    // Parses a const generic argument's expression with `>`-shaped tokens
    // (`Greater`, `GreaterEqual`, `ShiftRight`) suppressed as operators, so
    // a closing `>` (or a split-off `>>` closing two nested lists at once)
    // isn't consumed as a comparison or shift instead. See
    // `Parser::restrict_closing_ops`.
    fn parse_const_arg_expr(&mut self) -> Result<Expr, ParseError> {
        let outer_restriction = self.restrict_closing_ops;
        self.restrict_closing_ops = true;

        let result = self.parse_expr();

        self.restrict_closing_ops = outer_restriction;

        result
    }

    pub(super) fn parse_generic_params(
        &mut self,
    ) -> Result<Vec<GenericParameter>, ParseError> {
        let mut parameters = Vec::new();

        if !self.check(&TokenKind::Less) {
            return Ok(parameters);
        }

        self.advance();
        self.skip_newlines();

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

            self.skip_newlines();

            if self.check(&TokenKind::Comma) {
                self.advance();
                self.skip_newlines();
                continue;
            }

            break;
        }

        self.expect_generic_close()?;

        Ok(parameters)
    }
}
