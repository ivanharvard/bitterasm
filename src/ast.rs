//! The syntax tree produced by [`crate::parser`], before import resolution
//! ([`crate::loader`]) or symbol resolution ([`crate::resolver`]). A
//! [`Program`] is one file's worth of statements; multiple files are
//! flattened into one by the loader before anything downstream sees them.
//!
//! `mov r1, 7`-shaped lines parse as [`Statement::Invocation`] — BitterASM
//! has no built-in instruction syntax, so every mnemonic is just an
//! identifier followed by operand expressions, resolved to a macro
//! definition later. [`Statement::Meta`] is reserved for the `@`-prefixed
//! directives (e.g. a macro body's `@return`) that make up macro bodies.

use crate::token::Span;
use crate::types::{
    GenericParameter,
    StructField,
    TypeExpr,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Import(ImportStatement),

    Struct(StructDeclaration),
    TypeAlias(TypeAliasDeclaration),
    Const(ConstDeclaration),

    Label(Label),
    Invocation(Invocation),

    Macro(MacroDeclaration),
    
    Meta(MetaStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportStatement {
    pub module: ModulePath,
    pub items: ImportItems,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModulePath {
    pub segments: Vec<String>,
    pub relative_level: usize,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportItems {
    All,
    Names(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    pub name: String,
    pub operands: Vec<Expr>,
    pub span: Span, 
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier {
        name: String,
        span: Span,
    },

    Integer {
        raw: String,
        span: Span,
    },

    String {
        value: String,
        span: Span,
    },

    Member {
        object: Box<Expr>,
        member: String,
        span: Span,
    },

    Call {
        callee: Box<Expr>,
        arguments: Vec<CallArgument>,
        span: Span,
    },

    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },

    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },

    /// `` `expr` `` — "evaluate this expression now and splice the result
    /// in here", as opposed to a bare `expr` which, in a context that
    /// otherwise treats its surrounding tokens as literal (unevaluated)
    /// source, stays literal. Everywhere `expr` is already evaluated
    /// unconditionally (e.g. an `@emit` argument), a splice is a no-op:
    /// `` `1 + 1` `` and `1 + 1` evaluate identically.
    Splice {
        inner: Box<Expr>,
        span: Span,
    },

    /// `@here` — how many values have been `@emit`'d so far in this
    /// expansion, as a plain count (not bits/bytes; see
    /// `resolver::aliases::AliasResolver::values_emitted`). Unlike
    /// `@emit`/`@return`, which only make sense as their own body
    /// statement, `@here` is meant to be used inline (`target - @here`),
    /// so it's a primary expression rather than a `MetaStatement`.
    Here {
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Identifier { span, .. }
            | Expr::Integer { span, .. }
            | Expr::String { span, .. }
            | Expr::Member { span, .. }
            | Expr::Call { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Splice { span, .. }
            | Expr::Here { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArgument {
    pub name: Option<String>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
    BitNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,

    ShiftLeft,
    ShiftRight,

    BitAnd,
    BitXor,
    BitOr,

    And,
    Or,

    Equal,
    NotEqual,

    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaStatement {
    pub name: String,
    pub args: Vec<Expr>,
    pub body: Option<Vec<Statement>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDeclaration {
    pub name: String,
    pub is_pub: bool,
    /// Optional explicit annotation
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {
    pub name: String,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub facets: Vec<Facet>,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDeclaration {
    pub name: String,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDeclaration {
    pub name: String,
    pub is_pub: bool,
    pub params: Vec<MacroParameter>,
    pub return_ty: Option<TypeExpr>,
    pub facets: Vec<Facet>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroParameter {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

// ===============
// facets
// ===============
//
// `| name ...` modifiers on struct/macro declarations. `pub` and `-> Type`
// are spelled with dedicated tokens rather than a plain identifier, but
// they still parse into ordinary `Facet` entries here, same as any other
// facet — see `crate::facets` for what each name means and how its actual
// effect (e.g. `pub`'s on `is_pub`) gets applied. What names are valid,
// what they attach to, and how many times they may appear is metadata
// owned by `crate::facets`, not this type; this is just the parsed shape.

#[derive(Debug, Clone, PartialEq)]
pub struct Facet {
    pub name: String,
    pub payload: FacetPayload,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FacetPayload {
    Bare,
    Expr(Expr),
    Block(Vec<Statement>),
    Type(TypeExpr),
}