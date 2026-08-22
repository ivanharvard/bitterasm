//! A hand-written recursive-descent parser (Pratt parsing for expressions,
//! in [`expressions`]) over a single file's [`Token`] stream.
//!
//! Generic argument lists are the awkward part: `Reg<64>` and `Reg<T>` are
//! only distinguishable by knowing whether `Reg`'s declaration takes a
//! const or a type parameter, and that declaration may live later in this
//! file or in another file entirely. [`Parser`] handles the first case with
//! a throwaway prepass (see [`parse_seeded`]) that walks the file purely to
//! collect every declaration's generic signature before parsing it for
//! real; [`crate::loader`] handles the second by threading signatures
//! collected from one file's [`parse_seeded`] call into the next as a seed.
//!
//! A macro's `syntax` facet (see [`crate::facets::syntax`]) needs the exact
//! same treatment: matching a call site against a custom pattern requires
//! already knowing that pattern, possibly from later in this file or from
//! another file entirely, so [`ParserSeed`] carries `macro_syntaxes`
//! alongside `generic_signatures` through the same prepass and the same
//! cross-file threading in [`crate::loader`].

use std::collections::HashMap;
use std::fmt;

use crate::ast::{
    BinaryOp, CallArgument, ConstDeclaration, Expr, ImportItems, ImportStatement,
    Invocation, Label, MacroDeclaration, MacroParameter, MetaStatement, ModulePath,
    Program, Statement, UnaryOp, TypeAliasDeclaration,
};

use crate::facets::syntax::SyntaxPattern;
use crate::token::{Span, Token, TokenKind};
use crate::types::{GenericParameter, StructField, TypeExpr, TypeArgument};

mod expressions;
mod statements;
mod type_exprs;
mod declarations;
mod macros;
mod facets;
mod invocation_syntax;

#[cfg(test)]
mod tests;

// ================
// public entry point
// ================

/// Parses a single, self-contained token stream with no externally-known
/// generic signatures. Most callers should go through [`crate::loader`]
/// instead, which seeds signatures pulled in from imports.
///
/// ```
/// use bitterasm::lexer::lex;
/// use bitterasm::parser::parse;
///
/// let tokens = lex("const x = 1\n").unwrap();
/// let program = parse(tokens).unwrap();
/// assert_eq!(program.statements.len(), 1);
/// ```
pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    parse_seeded(tokens, &ParserSeed::default()).map(|(program, _)| program)
}

/// What the same-file prepass discovers, threaded across files the same
/// way: `generic_signatures` (struct/type-alias generic signatures) and
/// `macro_syntaxes` (custom call-site patterns from a `syntax` facet) are
/// unrelated key spaces produced by different declarations, but both are
/// needed *before* the rest of a file can be parsed for real, so both ride
/// through [`parse_seeded`]/[`crate::loader`] together as one seed value
/// rather than two parallel maps that always have to move in lockstep.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParserSeed {
    pub generic_signatures: HashMap<String, Vec<GenericParamKind>>,
    pub macro_syntaxes: HashMap<String, SyntaxPattern>,
}

/// Parses `tokens`, treating `seed` as generic signatures and macro syntax
/// patterns known in advance (e.g. pulled in from imported modules, whose
/// declarations aren't textually present in this token stream). Returns
/// everything visible by the end of the file (seed plus whatever this file
/// declared itself), so callers can pass it on to files that import *this*
/// one in turn.
pub(crate) fn parse_seeded(
    tokens: Vec<Token>,
    seed: &ParserSeed,
) -> Result<(Program, ParserSeed), ParseError> {
    // dummy pass to collect signatures
    let mut prepass = Parser::new(tokens.clone());
    prepass.generic_signatures.extend(seed.generic_signatures.clone());
    prepass.macro_syntaxes.extend(seed.macro_syntaxes.clone());
    let _ = prepass.parse_program();

    let mut parser = Parser::new(tokens);
    parser.generic_signatures = prepass.generic_signatures;
    parser.macro_syntaxes = prepass.macro_syntaxes;

    let program = parser.parse_program()?;

    Ok((
        program,
        ParserSeed {
            generic_signatures: parser.generic_signatures,
            macro_syntaxes: parser.macro_syntaxes,
        },
    ))
}

// Parses `tokens` leniently (ignoring errors past the first) purely to
// discover which modules it imports, without needing to know anything about
// those modules' contents first. Import statements don't depend on any
// generic signature, so this succeeds even when later statements in the
// file don't (e.g. because they use a type from one of these imports).
pub(crate) fn discover_imports(tokens: Vec<Token>) -> Vec<ImportStatement> {
    let mut parser = Parser::new(tokens);
    let _ = parser.parse_program();
    parser.imports
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
    macro_syntaxes: HashMap<String, SyntaxPattern>,
    imports: Vec<ImportStatement>,

    // While parsing a generic argument expression (e.g. the `width` in
    // `bits<width>`), `>`-shaped tokens (`Greater`, `GreaterEqual`,
    // `ShiftRight`) can't be treated as operators — any of them might
    // instead be closing the argument list. Every other operator, `<<`
    // included, is unambiguous (only `>` ever closes a list, never `<`),
    // so this is narrower than refusing every low-precedence operator.
    // Lifted while parsing inside parens, same as `Foo<(a > b)>` needing
    // parens in C++ for the same reason.
    pub(super) restrict_closing_ops: bool,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            generic_signatures: HashMap::new(),
            macro_syntaxes: HashMap::new(),
            imports: Vec::new(),
            restrict_closing_ops: false,
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

    fn register_macro_syntax(&mut self, name: &str, pattern: SyntaxPattern) {
        self.macro_syntaxes.insert(name.to_string(), pattern);
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
