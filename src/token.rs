//! Token and source-span types shared by the lexer, parser, and diagnostics.

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
    In,
    Pub,
    Skip,

    Macro,
    Type,
    Struct,
    Enum,
    Const,
    Section,

    // Punctuation
    Dot,
    DotDot,
    DotDotEq,
    Ellipsis,
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
    AndAnd,
    Pipe,
    OrOr,
    Caret,
    Tilde,

    ShiftLeft,
    ShiftRight,

    Arrow,
    FatArrow,

    Dollar,
    Backtick,

    /// A backslash-escaped literal character outside a string literal —
    /// `` \` `` or `\$`, spelling out a literal backtick/dollar sign in a
    /// position where the bare character would otherwise be meaningful
    /// (a splice delimiter, a capture delimiter). Carries the escaped
    /// character itself, same shape as `String` carrying its decoded value.
    Escaped(char),

    // Metaprogramming
    At,

    // Structure
    Newline,
    Eof,
}

impl TokenKind {
    /// The source text of a word-shaped token — an identifier or a keyword
    /// — or `None` for anything else. Used where a keyword can't mean
    /// anything special, e.g. after the `.` of a dotted label name
    /// (`.skip:`, `jmp .type`).
    pub fn word_text(&self) -> Option<&str> {
        Some(match self {
            TokenKind::Identifier(name) => name,
            TokenKind::From => "from",
            TokenKind::Import => "import",
            TokenKind::As => "as",
            TokenKind::In => "in",
            TokenKind::Pub => "pub",
            TokenKind::Skip => "skip",
            TokenKind::Macro => "macro",
            TokenKind::Type => "type",
            TokenKind::Struct => "struct",
            TokenKind::Enum => "enum",
            TokenKind::Const => "const",
            TokenKind::Section => "section",
            _ => return None,
        })
    }
}

/// A half-open `[start, end)` byte range into the original source text,
/// attached to every token and AST node for diagnostics.
///
/// ```
/// use bitterasm::token::Span;
///
/// let span = Span::new(3, 7);
/// assert_eq!(span.len(), 4);
/// assert!(!span.is_empty());
/// assert!(Span::new(5, 5).is_empty());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
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
