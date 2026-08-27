//! Turns `.basm` source text into a flat [`Token`] stream. Newlines are
//! preserved as their own tokens (statements are newline-terminated, not
//! semicolon-terminated), and `>>` always lexes as one [`TokenKind::ShiftRight`]
//! token even where it closes two nested generic lists — the parser splits
//! it back into two `>` tokens where that's what the grammar needs; see
//! [`crate::parser`].
//!
//! A bare backslash is only meaningful outside string literals as the start
//! of one of two escapes, `` \` `` or `\$` — [`TokenKind::Escaped`] carrying
//! the literal character it spells out, as opposed to the structural
//! [`TokenKind::Backtick`]/[`TokenKind::Dollar`] the un-escaped characters
//! produce. Any other character after `\` (including none, at EOF) is a
//! lex error; there's no general-purpose escape mechanism outside strings,
//! just these two.

use std::fmt;
use crate::token::{Token, TokenKind, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl LexError {
    pub fn new(message: impl Into<String>, start: usize, end: usize) -> Self {
        Self { message: message.into(), span: Span::new(start, end) }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at byte {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LexError {}

// Public API

/// Lexes an entire source string, returning every token including a
/// trailing `Eof`. Fails on the first invalid character or malformed
/// literal — there is no error-recovery pass.
///
/// ```
/// use bitterasm::lexer::lex;
/// use bitterasm::token::TokenKind;
///
/// let tokens = lex("const x = 1\n").unwrap();
/// let kinds: Vec<&TokenKind> = tokens.iter().map(|t| &t.kind).collect();
///
/// assert_eq!(
///     kinds,
///     vec![
///         &TokenKind::Const,
///         &TokenKind::Identifier("x".to_string()),
///         &TokenKind::Equal,
///         &TokenKind::Integer("1".to_string()),
///         &TokenKind::Newline,
///         &TokenKind::Eof,
///     ],
/// );
/// ```
pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex_all()
}

// Lexer

struct Lexer<'src> {
    source: &'src str,

    // current byte offset
    pos: usize,

    tokens: Vec<Token>,
}

impl<'src> Lexer<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            source,
            pos: 0,
            tokens: Vec::new(),
        }
    }

    fn lex_all(mut self) -> Result<Vec<Token>, LexError> {
        while !self.is_eof() {
            self.lex_one()?;
        }

        self.tokens.push(
            Token::new(
                TokenKind::Eof,
                self.pos,
                self.pos
            )
        );

        Ok(self.tokens)
    } 

    fn lex_one(&mut self) -> Result<(), LexError> {
        let ch = match self.peek() {
            Some(ch) => ch,
            None => return Ok(()),
        };
        
        match ch {
            // ignored whitespace
            ' ' | '\t' | '\r' => {
                self.skip_horizontal_whitespace();
            }

            // newline
            '\n' => {
                let start = self.pos;
                self.advance();

                self.push(TokenKind::Newline, start);
            }

            // comments
            '#' => {
                self.skip_line_comment();
            }

            // identifier/keywords
            ch if is_identifier_start(ch) => {
                self.lex_identifier();
            }

            // numbers
            '0'..='9' => {
                self.lex_number()?;
            }

            // strings
            '"' => {
                self.lex_string()?;
            }

            // chars — sugar for their codepoint as a plain integer literal,
            // not a distinct kind of token; see `lex_char`.
            '\'' => {
                self.lex_char()?;
            }

            // punctuation
            '.' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('.') {
                    if self.consume_if('.') {
                        self.push(TokenKind::Ellipsis, start);
                    } else {
                        self.push(TokenKind::DotDot, start);
                    }
                } else {
                    self.push(TokenKind::Dot, start);
                }
            }

            ',' => self.single(TokenKind::Comma),
            ':' => self.single(TokenKind::Colon),
            ';' => self.single(TokenKind::Semicolon),
            '(' => self.single(TokenKind::LParen),
            ')' => self.single(TokenKind::RParen),
            '[' => self.single(TokenKind::LBracket),
            ']' => self.single(TokenKind::RBracket),
            '{' => self.single(TokenKind::LBrace),
            '}' => self.single(TokenKind::RBrace),
            '*' => self.single(TokenKind::Star),

            '+' => self.single(TokenKind::Plus),
            '-' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('>') {
                    self.push(TokenKind::Arrow, start);
                } else {
                    self.push(TokenKind::Minus, start);
                }
            }

            '/' => self.single(TokenKind::Slash),
            '%' => self.single(TokenKind::Percent),

            '=' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('=') {
                    self.push(TokenKind::EqualEqual, start);
                } else if self.consume_if('>') {
                    self.push(TokenKind::FatArrow, start);
                } else {
                    self.push(TokenKind::Equal, start);
                }
            }

            '!' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('=') {
                    self.push(TokenKind::BangEqual, start);
                } else {
                    self.push(TokenKind::Bang, start);
                }
            }

            '<' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('=') {
                    self.push(TokenKind::LessEqual, start);
                } else if self.consume_if('<') {
                    self.push(TokenKind::ShiftLeft, start);
                } else {
                    self.push(TokenKind::Less, start);
                }
            }

            '>' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('=') {
                    self.push(TokenKind::GreaterEqual, start);
                } else if self.consume_if('>') {
                    self.push(TokenKind::ShiftRight, start);
                } else {
                    self.push(TokenKind::Greater, start);
                }
            }

            '&' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('&') {
                    self.push(TokenKind::AndAnd, start);
                } else {
                    self.push(TokenKind::Ampersand, start);
                }
            }

            '|' => {
                let start = self.pos;
                self.advance();

                if self.consume_if('|') {
                    self.push(TokenKind::OrOr, start);
                } else {
                    self.push(TokenKind::Pipe, start);
                }
            }

            '^' => self.single(TokenKind::Caret),
            '~' => self.single(TokenKind::Tilde),

            '@' => self.single(TokenKind::At),

            '$' => self.single(TokenKind::Dollar),
            '`' => self.single(TokenKind::Backtick),

            '\\' => self.lex_escape()?,

            // unknown

            other => {
                let start = self.pos;
                self.advance();

                return Err(LexError::new(
                    format!("unexpected character '{}'", other),
                    start,
                    self.pos,
                ));
            }
        }

        Ok(())
    }

    fn lex_identifier(&mut self) {
        let start = self.pos;

        self.advance(); 

        while let Some(ch) = self.peek() {
            if is_identifier_continue(ch) {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];

        let kind = match text {
            "from" => TokenKind::From,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "pub" => TokenKind::Pub,

            "macro" => TokenKind::Macro,
            "type" => TokenKind::Type,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "const" => TokenKind::Const,

            _ => TokenKind::Identifier(text.to_owned()),
        };

        self.push(kind, start);
    }

    // numbers
    fn lex_number(&mut self) -> Result<(), LexError> {
        let start = self.pos;

        if self.peek() == Some('0') {
            self.advance();

            match self.peek() {
                Some('x') | Some('X') => {
                    self.advance();

                    let digit_start = self.pos;
                    self.consume_digits(|ch| ch.is_ascii_hexdigit());

                    if self.pos == digit_start {
                        return Err(LexError::new(
                            "expected hexadecimal digits after 0x",
                            start,
                            self.pos,
                        ));
                    }
                }

                Some('b') | Some('B') => {
                    self.advance();

                    let digit_start = self.pos;
                    self.consume_digits(|ch| matches!(ch, '0' | '1'));

                    if self.pos == digit_start {
                        return Err(LexError::new(
                            "expected binary digits after 0b",
                            start,
                            self.pos,
                        ));
                    }
                }

                Some('o') | Some('O') => {
                    self.advance();

                    let digit_start = self.pos;
                    self.consume_digits(|ch| matches!(ch, '0'..='7'));

                    if self.pos == digit_start {
                        return Err(LexError::new(
                            "expected octal digits after 0o",
                            start,
                            self.pos,
                        ));
                    }
                }

                _ => {
                    self.consume_digits(|ch| ch.is_ascii_digit());
                }
            }
        } else {
            self.consume_digits(|ch| ch.is_ascii_digit());
        }

        let text = &self.source[start..self.pos];

        self.push(TokenKind::Integer(text.to_owned()), start);

        Ok(())
    }

    fn consume_digits<F>(&mut self, is_valid_digit: F)
    where
        F: Fn(char) -> bool,
    {
        let mut previous_was_underscore = false;

        while let Some(ch) = self.peek() {
            if is_valid_digit(ch) {
                previous_was_underscore = false;
                self.advance();
            } else if ch == '_' && !previous_was_underscore {
                previous_was_underscore = true;
                self.advance();
            } else {
                break;
            }
        }
    }

    // strings
    fn lex_string(&mut self) -> Result<(), LexError> {
        let start = self.pos;

        // Opening quote.
        self.advance();

        let mut value = String::new();

        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();

                    self.push(TokenKind::String(value), start);

                    return Ok(());
                }

                '\\' => {
                    self.advance();

                    let escape_start = self.pos;

                    let escaped = match self.advance() {
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('0') => '\0',
                        Some('\\') => '\\',
                        Some('"') => '"',

                        Some(other) => {
                            return Err(LexError::new(
                                format!("unknown escape sequence \\{other}"),
                                escape_start.saturating_sub(1),
                                self.pos,
                            ));
                        }

                        None => {
                            return Err(LexError::new(
                                "unterminated escape sequence",
                                start,
                                self.pos,
                            ));
                        }
                    };

                    value.push(escaped);
                }

                '\n' => {
                    return Err(LexError::new(
                        "unterminated string literal",
                        start,
                        self.pos,
                    ));
                }

                other => {
                    self.advance();
                    value.push(other);
                }
            }
        }

        Err(LexError::new(
            "unterminated string literal",
            start,
            self.pos,
        ))
    }

    // chars — `'a'` desugars here, at lex time, straight into a plain
    // `TokenKind::Integer` holding the one character's codepoint in
    // decimal — the same relationship `0x61` has to `97`, not a distinct
    // value kind downstream (see `crate::ast::Expr::Integer`). Exactly one
    // (possibly escaped) character is required between the quotes; zero or
    // more than one is a `LexError` — unlike a string, there's no length
    // this could otherwise ambiguously mean.
    fn lex_char(&mut self) -> Result<(), LexError> {
        let start = self.pos;

        // Opening quote.
        self.advance();

        let ch = match self.peek() {
            Some('\'') => {
                return Err(LexError::new(
                    "a char literal can't be empty",
                    start,
                    self.pos,
                ));
            }

            Some('\\') => {
                self.advance();

                let escape_start = self.pos;

                match self.advance() {
                    Some('n') => '\n',
                    Some('r') => '\r',
                    Some('t') => '\t',
                    Some('0') => '\0',
                    Some('\\') => '\\',
                    Some('\'') => '\'',

                    Some(other) => {
                        return Err(LexError::new(
                            format!("unknown escape sequence \\{other}"),
                            escape_start.saturating_sub(1),
                            self.pos,
                        ));
                    }

                    None => {
                        return Err(LexError::new(
                            "unterminated escape sequence",
                            start,
                            self.pos,
                        ));
                    }
                }
            }

            Some('\n') | None => {
                return Err(LexError::new(
                    "unterminated char literal",
                    start,
                    self.pos,
                ));
            }

            Some(other) => {
                self.advance();
                other
            }
        };

        match self.peek() {
            Some('\'') => {
                self.advance();
            }

            _ => {
                return Err(LexError::new(
                    "a char literal must contain exactly one character",
                    start,
                    self.pos,
                ));
            }
        }

        self.push(TokenKind::Integer((ch as u32).to_string()), start);

        Ok(())
    }

    // escapes outside strings — `` \` `` / `\$`, spelling out a literal
    // backtick/dollar sign in source text rather than the delimiter
    // meaning those characters carry unescaped (a splice, a capture).
    fn lex_escape(&mut self) -> Result<(), LexError> {
        let start = self.pos;

        // Leading backslash.
        self.advance();

        let escaped = match self.advance() {
            Some('`') => '`',
            Some('$') => '$',

            Some(other) => {
                return Err(LexError::new(
                    format!("unknown escape sequence \\{other}"),
                    start,
                    self.pos,
                ));
            }

            None => {
                return Err(LexError::new(
                    "unterminated escape sequence",
                    start,
                    self.pos,
                ));
            }
        };

        self.push(TokenKind::Escaped(escaped), start);

        Ok(())
    }

    // comments
    fn skip_horizontal_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\r')) {
            self.advance();
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }

            self.advance();
        }
    }

    // character navigation
    fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

     /// Consume and return the next character.
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;

        self.pos += ch.len_utf8();

        Some(ch)
    }

    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    // helpers

    fn single(&mut self, kind: TokenKind) {
        let start = self.pos;

        self.advance();

        self.push(kind, start);
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token::new(kind, start, self.pos));
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn lexes_invocation() {
        assert_eq!(
            kinds("mov r1, 42\n"),
            vec![
                TokenKind::Identifier("mov".into()),
                TokenKind::Identifier("r1".into()),
                TokenKind::Comma,
                TokenKind::Integer("42".into()),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_import() {
        assert_eq!(
            kinds("from tinycpu.native import *\n"),
            vec![
                TokenKind::From,
                TokenKind::Identifier("tinycpu".into()),
                TokenKind::Dot,
                TokenKind::Identifier("native".into()),
                TokenKind::Import,
                TokenKind::Star,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_label() {
        assert_eq!(
            kinds("start:\n"),
            vec![
                TokenKind::Identifier("start".into()),
                TokenKind::Colon,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_number_bases() {
        assert_eq!(
            kinds("42 0xff 0b1010 0o755\n"),
            vec![
                TokenKind::Integer("42".into()),
                TokenKind::Integer("0xff".into()),
                TokenKind::Integer("0b1010".into()),
                TokenKind::Integer("0o755".into()),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_number_separators() {
        assert_eq!(
            kinds("1_000 0xff_ff 0b1010_0101\n"),
            vec![
                TokenKind::Integer("1_000".into()),
                TokenKind::Integer("0xff_ff".into()),
                TokenKind::Integer("0b1010_0101".into()),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn ignores_comments_but_preserves_newline() {
        assert_eq!(
            kinds("mov r1, 42 # hello\nnop\n"),
            vec![
                TokenKind::Identifier("mov".into()),
                TokenKind::Identifier("r1".into()),
                TokenKind::Comma,
                TokenKind::Integer("42".into()),
                TokenKind::Newline,
                TokenKind::Identifier("nop".into()),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_metaprogramming() {
        assert_eq!(
            kinds("@if foo == 42 {\n@emit 0xff\n}\n"),
            vec![
                TokenKind::At,
                TokenKind::Identifier("if".into()),
                TokenKind::Identifier("foo".into()),
                TokenKind::EqualEqual,
                TokenKind::Integer("42".into()),
                TokenKind::LBrace,
                TokenKind::Newline,
                TokenKind::At,
                TokenKind::Identifier("emit".into()),
                TokenKind::Integer("0xff".into()),
                TokenKind::Newline,
                TokenKind::RBrace,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_match_fat_arrow() {
        assert_eq!(
            kinds("@match x { 0 => {} }\n"),
            vec![
                TokenKind::At,
                TokenKind::Identifier("match".into()),
                TokenKind::Identifier("x".into()),
                TokenKind::LBrace,
                TokenKind::Integer("0".into()),
                TokenKind::FatArrow,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::RBrace,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_range() {
        assert_eq!(
            kinds("0..N\n"),
            vec![
                TokenKind::Integer("0".into()),
                TokenKind::DotDot,
                TokenKind::Identifier("N".into()),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_member_access_distinct_from_range() {
        assert_eq!(
            kinds("arr.len\n"),
            vec![
                TokenKind::Identifier("arr".into()),
                TokenKind::Dot,
                TokenKind::Identifier("len".into()),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_string_escapes() {
        assert_eq!(
            kinds("\"hello\\nworld\""),
            vec![
                TokenKind::String("hello\nworld".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_char_literal_as_its_codepoint() {
        assert_eq!(
            kinds("'a'"),
            vec![
                TokenKind::Integer("97".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_escaped_char_literal() {
        assert_eq!(
            kinds("'\\n'"),
            vec![
                TokenKind::Integer("10".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn rejects_empty_char_literal() {
        assert!(lex("''").is_err());
    }

    #[test]
    fn rejects_multi_character_char_literal() {
        assert!(lex("'ab'").is_err());
    }

    #[test]
    fn spans_are_byte_offsets() {
        let tokens = lex("mov r1").unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 3));
        assert_eq!(tokens[1].span, Span::new(4, 6));
        assert_eq!(tokens[2].span, Span::new(6, 6));
    }

    #[test]
    fn supports_unicode_identifiers() {
        let tokens = lex("λ foo\n").unwrap();

        assert_eq!(
            tokens[0],
            Token {
                kind: TokenKind::Identifier("λ".into()),
                span: Span::new(0, 2),
            }
        );
    }

    #[test]
    fn rejects_unknown_characters() {
        let error = lex("mov r1, ?").unwrap_err();

        assert!(error.message.contains("unexpected character"));
    }

    #[test]
    fn lexes_backtick() {
        assert_eq!(
            kinds("`foo`\n"),
            vec![
                TokenKind::Backtick,
                TokenKind::Identifier("foo".into()),
                TokenKind::Backtick,
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_escapes_outside_strings() {
        assert_eq!(
            kinds("\\`\\$\n"),
            vec![
                TokenKind::Escaped('`'),
                TokenKind::Escaped('$'),
                TokenKind::Newline,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn rejects_unknown_escape_outside_strings() {
        let error = lex("\\n").unwrap_err();

        assert!(error.message.contains("unknown escape sequence"));
    }

    #[test]
    fn rejects_unterminated_escape_outside_strings() {
        let error = lex("\\").unwrap_err();

        assert_eq!(error.message, "unterminated escape sequence");
    }

    #[test]
    fn rejects_unterminated_strings() {
        let error = lex("\"hello").unwrap_err();

        assert_eq!(error.message, "unterminated string literal");
    }
}
