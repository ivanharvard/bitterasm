use std::collections::HashMap;
use std::fmt;

use crate::ast::{
    BinaryOp, CallArgument, ConstDeclaration, Expr, ImportItems, ImportStatement,
    Invocation, Label, ModulePath, Program, Statement, UnaryOp, TypeAliasDeclaration,
};
use crate::token::{Span, Token, TokenKind};
use crate::types::{GenericParameter, StructField, TypeExpr, TypeArgument};


mod expressions;
mod statements;

#[cfg(test)]
mod tests;

// ================
// public entry point
// ================

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    // dummy pass to collect signatures
    let mut prepass = Parser::new(tokens.clone());
    let _ = prepass.parse_program();

    let mut parser = Parser::new(tokens);
    parser.generic_signatures = prepass.generic_signatures;

    parser.parse_program()
}

// ================
// errors
// ================

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at byte {}..{}",
            self.message,
            self.span.start,
            self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

// ================
// parser
// ================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenericParamKind {
    Type,
    Const,
}

impl From<&GenericParameter> for GenericParamKind {
    fn from(param: &GenericParameter) -> Self {
        match param {
            GenericParameter::Type { .. } => GenericParamKind::Type,
            GenericParameter::Const { .. } => GenericParamKind::Const,
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,

    generic_signatures: HashMap<String, Vec<GenericParamKind>>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        // `bits` has no struct declaration to register its signature from.
        let mut generic_signatures = HashMap::new();
        generic_signatures.insert(
            "bits".to_string(),
            vec![GenericParamKind::Const],
        );

        Self {
            tokens,
            pos: 0,
            generic_signatures,
        }
    }

    fn register_generic_signature(
        &mut self,
        name: &str,
        params: &[GenericParameter],
    ) {
        if params.is_empty() {
            return;
        }

        self.generic_signatures.insert(
            name.to_string(),
            params.iter().map(GenericParamKind::from).collect(),
        );
    }

    // ===============
    // program
    // ===============

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start = self.current().span.start;

        let mut statements = Vec::new();

        self.skip_newlines();

        while !self.at_eof() {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        let end = self.current().span.end;

        Ok(Program {
            statements,
            span: Span::new(start, end),
        })
    }

    // =============
    // statement end
    // =============

    fn at_statement_end(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Newline | TokenKind::Eof
        )
    }

    // consume a new line if present and return the ending byte offset
    // eof is also a valid statement terminator
    fn statement_end(&mut self) -> Result<usize, ParseError> {
        match self.current().kind {
            TokenKind::Newline => {
                let end = self.current().span.end;
                self.advance();
                Ok(end)
            }

            TokenKind::Eof => Ok(self.current().span.end),

            _ => Err(ParseError::new(
                "expected end of line",
                self.current().span,
            )),
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    // =============
    // expectations
    // =============

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        let token = self.current().clone();

        match token.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }

            other => Err(ParseError::new(
                format!("expected identifier, found {other:?}"),
                token.span,
            )),
        }
    }

    fn expect_simple(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                format!(
                    "expected {:?}, found {:?}",
                    expected,
                    self.current().kind
                ),
                self.current().span,
            ))
        }
    }

    // `>>` lexes as one `ShiftRight` token, so closing two nested generic
    // lists at once (e.g. `Param<Reg<64>>`) needs it split back into two
    // `Greater` tokens.
    fn expect_generic_close(&mut self) -> Result<Token, ParseError> {
        if self.check(&TokenKind::ShiftRight) {
            let token = self.current().clone();
            let mid = token.span.start + 1;

            self.tokens[self.pos] = Token::new(TokenKind::Greater, mid, token.span.end);
            self.tokens.insert(self.pos, Token::new(TokenKind::Greater, token.span.start, mid));

            return Ok(self.advance().clone());
        }

        let token = self.current().clone();
        self.expect_simple(TokenKind::Greater)?;
        Ok(token)
    }

    // =============
    // token navigation
    // =============

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.at_eof() {
            self.pos += 1;
        }
        self.previous()
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        same_variant(&self.current().kind, kind)
    }

    fn check_next(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|token| same_variant(&token.kind, kind))
            .unwrap_or(false)
    }
}

// =============
// token comparison
// =============

fn same_variant(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}
