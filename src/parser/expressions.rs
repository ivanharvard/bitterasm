use super::*;

impl Parser {
    // =============
    // expressions
    // =============

    pub(super) fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    pub fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix_expr()?;

        loop {
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

            let right = self.parse_expr_bp(right_bp)?;

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

    fn parse_prefix_expr(&mut self) -> Result<Expr, ParseError> {
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

                let mut expr = self.parse_expr()?;

                let closing = self.current().clone();

                self.expect_simple(TokenKind::RParen)?;

                let span = Span::new(start, closing.span.end);

                set_expr_span(&mut expr, span);

                Ok(expr)
            }

            // ============
            // unary -
            // ============

            TokenKind::Minus => {
                let start = token.span.start;

                self.advance();

                let operand = self.parse_expr_bp(90)?;

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

                let operand = self.parse_expr_bp(90)?;

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

                let operand = self.parse_expr_bp(90)?;

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

        let mut arguments = Vec::new();

        if !self.check(&TokenKind::RParen) {
            loop {
                arguments.push(self.parse_call_argument()?);

                if self.check(&TokenKind::Comma) {
                    self.advance();

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
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. } => {
            *span = new_span;
        }
    }
}
