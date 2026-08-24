use super::*;

impl Parser {
    // =============
    // expressions
    // =============

    pub(super) fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    pub fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        self.parse_expr_bp_until(min_bp, None)
    }

    // Bounded variant used by custom invocation-syntax capture matching:
    // `stop`, when set, ends the expression the moment the upcoming token
    // would otherwise be swallowed as this capture's own continuation (a
    // postfix `.`/`(`, or a binary operator) even though it's meant to
    // belong to the pattern's next literal segment instead — e.g. `<-`
    // lexes as `Less` then `Minus`, so capturing `$dst$` in
    // `mov $dst$ <- $value$` must stop at `Less` rather than parsing
    // `r1 <- 7` as `r1 < (-7)` in one shot. `None` is the ordinary,
    // unbounded case every other caller uses.
    pub(super) fn parse_expr_bp_until(
        &mut self,
        min_bp: u8,
        stop: Option<&TokenKind>,
    ) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix_expr(stop)?;

        loop {
            if let Some(stop) = stop {
                if same_variant(&self.current().kind, stop) {
                    break;
                }
            }

            // postfix: member access
            //
            //  foo.bar
            //  foo.bar.baz
            if self.check(&TokenKind::Dot) {
                let binding_power = 100;

                if binding_power < min_bp {
                    break;
                }

                self.advance();

                let member_token = self.current().clone();
                let member = self.expect_identifier()?;

                let span = Span::new(
                    left.span().start,
                    member_token.span.end,
                );

                left = Expr::Member {
                    object: Box::new(left),
                    member,
                    span,
                };

                continue;
            }

            // postfix: function call
            //
            //  foo()
            //  foo(bar, baz)
            //  Reg(id = 0)
            if self.check(&TokenKind::LParen) {
                let binding_power = 100;

                if binding_power < min_bp {
                    break;
                }

                left = self.finish_call(left)?;
                continue;
            }

            // postfix: brace-literal construction
            //
            //  U8String { chars: ... }
            //  Array<u8, N> { __el0: value, ... }
            //
            // Only reachable for a bare identifier callee (matching
            // `Expr::Call`'s existing constraint) and only when not
            // `restrict_brace_construction` — see that flag's doc for why:
            // an `@if`/`@for` header's condition/range bound also ends in a
            // bare identifier immediately followed by `{`, and that `{`
            // means something else entirely there.
            let callee_name = match &left {
                Expr::Identifier { name, .. } if !self.restrict_brace_construction => Some(name.clone()),
                _ => None,
            };

            if let Some(name) = callee_name {
                if self.check(&TokenKind::LBrace) {
                    let binding_power = 100;

                    if binding_power < min_bp {
                        break;
                    }

                    left = self.finish_construct(left, Vec::new())?;
                    continue;
                }

                // Gating on a known generic signature (rather than just
                // "next token is `<`") is what keeps this from misreading
                // an ordinary `x < y` comparison as the start of generic
                // arguments — mirrors how type position already knows a
                // name's generic signature before committing to parsing
                // `<...>` as arguments.
                if self.check(&TokenKind::Less) && self.generic_signatures.contains_key(&name) {
                    let binding_power = 100;

                    if binding_power < min_bp {
                        break;
                    }

                    let start = left.span().start;
                    let (generic_args, _) = self.parse_generic_argument_list(Some(&name), start)?;

                    if !self.check(&TokenKind::LBrace) {
                        return Err(ParseError::new(
                            "expected `{` after generic arguments in a brace-literal \
                             construction (a generic callee without a trailing `{ ... }` \
                             isn't supported here)",
                            self.current().span,
                        ));
                    }

                    left = self.finish_construct(left, generic_args)?;
                    continue;
                }
            }

            // postfix: `@as` — the only way to produce a value of a nominal
            // (invariant-bearing) `type` alias.
            //
            //  3 @as int16_t
            //  "Abc" @as String<3>
            if self.at_as_meta() {
                let binding_power = 100;

                if binding_power < min_bp {
                    break;
                }

                let start = left.span().start;

                self.advance(); // `@`
                self.advance(); // `as`

                let ty = self.parse_type_expr()?;
                let end = ty.span().end;

                left = Expr::As {
                    value: Box::new(left),
                    ty,
                    span: Span::new(start, end),
                };

                continue;
            }

            // ============
            // binary ops
            // ============

            let Some((left_bp, right_bp, op)) =
                self.current_binary_operator()
            else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.advance();

            let right = self.parse_expr_bp_until(right_bp, stop)?;

            let span = Span::new(
                left.span().start,
                right.span().end,
            );

            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_prefix_expr(&mut self, stop: Option<&TokenKind>) -> Result<Expr, ParseError> {
        let token = self.current().clone();

        match token.kind {
            // ============
            // literals
            // ============

            TokenKind::Identifier(name) => {
                self.advance();

                Ok(Expr::Identifier {
                    name,
                    span: token.span,
                })
            }

            TokenKind::Integer(raw) => {
                self.advance();

                Ok(Expr::Integer {
                    raw,
                    span: token.span,
                })
            }

            TokenKind::String(value) => {
                self.advance();

                Ok(Expr::String {
                    value,
                    span: token.span,
                })
            }

            // ============
            // paranthesized expressions
            // ============

            TokenKind::LParen => {
                let start = token.span.start;

                self.advance();

                // Parens have their own explicit close, so `>`-shaped
                // tokens inside them are never ambiguous with closing a
                // generic argument list — same as C++ allowing `(a > b)`
                // there but not a bare `a > b`. Brace construction is
                // similarly unambiguous once inside parens — same as Rust
                // allowing a struct literal inside parens in `if`/`while`
                // scrutinee position.
                let outer_closing_restriction = self.restrict_closing_ops;
                let outer_brace_restriction = self.restrict_brace_construction;
                self.restrict_closing_ops = false;
                self.restrict_brace_construction = false;

                let result = self.parse_expr();

                self.restrict_closing_ops = outer_closing_restriction;
                self.restrict_brace_construction = outer_brace_restriction;

                let mut expr = result?;

                let closing = self.current().clone();

                self.expect_simple(TokenKind::RParen)?;

                let span = Span::new(start, closing.span.end);

                set_expr_span(&mut expr, span);

                Ok(expr)
            }

            // ============
            // `expr` — evaluate and splice
            // ============

            TokenKind::Backtick => {
                let start = token.span.start;

                self.advance();

                // Same reasoning as the LParen case just below: a splice
                // has its own explicit close, so a `>`-shaped token inside
                // it can't be closing a generic argument list, and a `{`
                // inside it can't be an enclosing `@if`/`@for` header's body.
                let outer_closing_restriction = self.restrict_closing_ops;
                let outer_brace_restriction = self.restrict_brace_construction;
                self.restrict_closing_ops = false;
                self.restrict_brace_construction = false;

                let result = self.parse_expr();

                self.restrict_closing_ops = outer_closing_restriction;
                self.restrict_brace_construction = outer_brace_restriction;

                let inner = result?;

                let closing = self.current().clone();

                self.expect_simple(TokenKind::Backtick)?;

                let span = Span::new(start, closing.span.end);

                Ok(Expr::Splice {
                    inner: Box::new(inner),
                    span,
                })
            }

            // ============
            // @here
            // ============
            //
            // Every other `@`-prefixed directive (`@emit`, `@return`, ...)
            // only makes sense as its own body statement and is parsed by
            // `parse_meta_statement` instead — `@here` is the one that
            // needs to work inline (`target - @here`), so it's recognized
            // here as a primary expression. Deliberately narrow: only the
            // literal identifier `here` is accepted after `@` in
            // expression position; nothing else is a general "any `@foo`
            // is an expression" mechanism.

            TokenKind::At => {
                let start = token.span.start;

                self.advance();

                let name_token = self.current().clone();
                let name = self.expect_identifier()?;

                if name != "here" {
                    return Err(ParseError::new(
                        format!("expected `here` after `@` in an expression, found `{name}`"),
                        name_token.span,
                    ));
                }

                Ok(Expr::Here {
                    span: Span::new(start, name_token.span.end),
                })
            }

            // ============
            // unary -
            // ============

            TokenKind::Minus => {
                let start = token.span.start;

                self.advance();

                let operand = self.parse_expr_bp_until(90, stop)?;

                let span = Span::new(start, operand.span().end);

                Ok(Expr::Unary {
                    op: UnaryOp::Negate,
                    operand: Box::new(operand),
                    span,
                })
            }

            // ===========
            // unary !
            // ============

            TokenKind::Bang => {
                let start = token.span.start;

                self.advance();

                let operand = self.parse_expr_bp_until(90, stop)?;

                let span = Span::new(
                    start,
                    operand.span().end,
                );

                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }

            // ===========
            // unary ~
            // ============

            TokenKind::Tilde => {
                let start = token.span.start;

                self.advance();

                let operand = self.parse_expr_bp_until(90, stop)?;

                let span = Span::new(
                    start,
                    operand.span().end,
                );

                Ok(Expr::Unary {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                    span,
                })
            }

            other => Err(ParseError::new(
                format!("expected expression, found {other:?}"),
                token.span,
            )),
        }
    }

    // =============
    // binary ops
    // =============
    fn current_binary_operator(&self) -> Option<(u8, u8, BinaryOp)> {
        // Only `>`-shaped tokens can be mistaken for closing a generic
        // argument list; nothing else is ambiguous there, regardless of
        // precedence — see `Parser::restrict_closing_ops`.
        if self.restrict_closing_ops
            && matches!(
                self.current().kind,
                TokenKind::Greater | TokenKind::GreaterEqual | TokenKind::ShiftRight
            )
        {
            return None;
        }

        let result = match self.current().kind {
            // Highest binary precedence
            TokenKind::Star => (80, 81, BinaryOp::Multiply),
            TokenKind::Slash => (80, 81, BinaryOp::Divide),
            TokenKind::Percent => (80, 81, BinaryOp::Remainder),

            TokenKind::Plus => (70, 71, BinaryOp::Add),
            TokenKind::Minus => (70, 71, BinaryOp::Subtract),

            TokenKind::ShiftLeft => (60, 61, BinaryOp::ShiftLeft),
            TokenKind::ShiftRight => (60, 61, BinaryOp::ShiftRight),

            TokenKind::Less => (50, 51, BinaryOp::Less),
            TokenKind::LessEqual => (50, 51, BinaryOp::LessEqual),
            TokenKind::Greater => (50, 51, BinaryOp::Greater),
            TokenKind::GreaterEqual => (50, 51, BinaryOp::GreaterEqual),

            TokenKind::EqualEqual => (40, 41, BinaryOp::Equal),
            TokenKind::BangEqual => (40, 41, BinaryOp::NotEqual),

            TokenKind::Ampersand => (30, 31, BinaryOp::BitAnd),

            TokenKind::Caret => (20, 21, BinaryOp::BitXor),

            TokenKind::Pipe => (10, 11, BinaryOp::BitOr),

            TokenKind::AndAnd => (6, 7, BinaryOp::And),
            TokenKind::OrOr => (2, 3, BinaryOp::Or),

            _ => return None,
        };

        Some(result)
    }

    // =============
    // function calls
    // =============
    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let start = callee.span().start;

        self.expect_simple(TokenKind::LParen)?;
        self.skip_newlines();

        let mut arguments = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                arguments.push(self.parse_call_argument()?);
                self.skip_newlines();

                if self.check(&TokenKind::Comma) {
                    self.advance();
                    self.skip_newlines();

                    if self.check(&TokenKind::RParen) {
                        // Allow:
                        //     foo(1, 2,)
                        break;
                    }

                    continue;
                }

                break;
            }
        }

        let closing = self.current().clone();

        self.expect_simple(TokenKind::RParen)?;

        Ok(Expr::Call {
            callee: Box::new(callee),
            arguments,
            span: Span::new(start, closing.span.end),
        })
    }

    fn parse_call_argument(&mut self) -> Result<CallArgument, ParseError> {
        let start = self.current().span.start;

        // Named arguments like id = 0
        let name = if matches!(
            self.current().kind,
            TokenKind::Identifier(_)
        ) && self.check_next(&TokenKind::Equal)
        {
            let name = self.expect_identifier()?;

            self.expect_simple(TokenKind::Equal)?;

            Some(name)
        } else {
            None
        };

        let value = self.parse_expr()?;

        let end = value.span().end;

        Ok(CallArgument {
            name,
            value,
            span: Span::new(start, end),
        })
    }
}

// =============
// other helpers
// =============

fn set_expr_span(expr: &mut Expr, new_span: Span) {
    match expr {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::String { span, .. }
        | Expr::Member { span, .. }
        | Expr::Call { span, .. }
        | Expr::Construct { span, .. }
        | Expr::As { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Splice { span, .. }
        | Expr::Here { span, .. } => {
            *span = new_span;
        }
    }
}
