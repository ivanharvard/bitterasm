#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Values
    Identifier(String),
    Integer(String),
    String(String),

    // Keywords
    From,
    Import,
    As,
    Pub,

    Macro,
    Type,
    Struct,
    Enum,
    Const,

    // Punctuation
    Dot,
    Comma,
    Colon,
    Semicolon,

    LParen,
    RParen,

    LBracket,
    RBracket,

    LBrace,
    RBrace,

    Star,

    // Operators
    Plus,
    Minus,
    Slash,
    Percent,

    Equal,
    EqualEqual,
    Bang,
    BangEqual,

    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    Ampersand,
    Pipe,
    Caret,
    Tilde,

    ShiftLeft,
    ShiftRight,

    Arrow,

    Dollar,

    // Metaprogramming
    At,

    // Structure
    Newline,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Self { kind, span: Span::new(start, end) }
    }
}