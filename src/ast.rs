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
    StructBodyItem,
    TypeArgument,
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
    Enum(EnumDeclaration),
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
        member: SplicedName,
        span: Span,
    },

    Call {
        callee: Box<Expr>,
        arguments: Vec<CallArgument>,
        span: Span,
    },

    /// A qualified enum variant such as `Option<int>.Some(42)` or
    /// `Option<int>.None`.
    EnumVariant {
        enum_name: String,
        generic_args: Vec<TypeArgument>,
        variant: String,
        payload: Option<Box<Expr>>,
        span: Span,
    },

    /// `Array<u8, N> { field: value, ... }` — brace-literal struct
    /// construction. `generic_args` is empty for a non-generic callee (e.g.
    /// `U8String { chars: ... }`); when present, they're parsed only when
    /// `callee`'s name is already known (via `Parser::generic_signatures`)
    /// to take generics — see `parser::expressions`. `fields` mirrors
    /// `types::StructBodyItem`'s `@for`/`@if`-generative shape, but built
    /// from value expressions ([`ConstructItem`]) rather than declared
    /// field types, since a construction supplies values, not a schema.
    Construct {
        callee: Box<Expr>,
        generic_args: Vec<TypeArgument>,
        fields: Vec<ConstructItem>,
        span: Span,
    },

    /// `expr as Type` — the only way to produce a value of a nominal
    /// (invariant-bearing) `type` alias: checks every invariant along
    /// `Type`'s alias chain, auto-wrapping `expr`'s value into a
    /// single-field struct's field where needed (recursing — "holds all
    /// the way down"). `Type`'s own generic arguments (if any) are always
    /// spelled out explicitly here, never inferred — see
    /// `resolver::values::AliasResolver::convert_to`.
    As {
        value: Box<Expr>,
        ty: TypeExpr,
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

    /// `start..end` — exclusive-upper-bound range sugar. Not a value on its
    /// own (evaluating one directly is `EvalError::NotConstant`); the only
    /// place it evaluates to something is `resolver::values::eval_value`,
    /// which turns it into a synthesized `Value::Struct` with one pub field
    /// per element (see `resolver::generated::eval_range_value`) — `@for`'s
    /// four call sites all consume it that way, uniformly with any other
    /// struct-valued `in`-expression.
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
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
            | Expr::EnumVariant { span, .. }
            | Expr::Construct { span, .. }
            | Expr::As { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Splice { span, .. }
            | Expr::Here { span, .. }
            | Expr::Range { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArgument {
    pub name: Option<String>,
    pub value: Expr,
    pub span: Span,
}

/// One item in a brace-literal construction's field list — either a field
/// written directly, or an `@for`/`@if` that generates zero or more fields
/// once evaluated. The value-expression counterpart of
/// [`crate::types::StructBodyItem`]: a struct *declaration*'s body is
/// field-shaped types waiting to be resolved, a *construction*'s body is
/// field-shaped values waiting to be evaluated, so they're deliberately
/// separate types even though the `@for`/`@if` shape is identical.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstructItem {
    Field {
        name: SplicedName,
        value: Expr,
        span: Span,
    },

    For {
        var: String,
        source: Expr,
        body: Vec<ConstructItem>,
        span: Span,
    },

    If {
        condition: Expr,
        body: Vec<ConstructItem>,
        else_body: Option<Vec<ConstructItem>>,
        span: Span,
    },
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

    /// `@if`'s trailing `@else { ... }`, when present. `None` for every
    /// other meta, including an `@if` with no `@else`.
    pub else_body: Option<Vec<Statement>>,

    /// `@match`'s ordered arms. A `None` pattern is Rust's `_` wildcard.
    /// Empty for every other meta statement.
    pub match_arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Option<Expr>,
    pub body: Vec<Statement>,
    pub span: Span,
}

/// One piece of a name that may be built from evaluated fragments —
/// `` r`id` `` is `[Literal("r"), Splice(id)]`: the literal text `"r"`
/// followed by `id` evaluated and pasted in as text, the same "evaluate
/// this now and paste the result in its place" semantics [`Expr::Splice`]
/// already has for a value position, just applied to build up a name one
/// piece at a time. An ordinary, non-computed name (the overwhelming
/// majority) is just `[Literal(name)]`.
#[derive(Debug, Clone, PartialEq)]
pub enum NamePart {
    Literal(String),
    Splice(Expr),
}

pub type SplicedName = Vec<NamePart>;

/// `Some(name)` if every part is a literal (i.e. there's nothing left to
/// evaluate), `None` if a `Splice` remains — used to require an
/// already-fully-resolved name in a position that can't evaluate one
/// itself (e.g. a top-level declaration, checked in
/// `resolver::collect_symbols`).
pub fn literal_name(parts: &[NamePart]) -> Option<String> {
    let mut out = String::new();

    for part in parts {
        match part {
            NamePart::Literal(text) => out.push_str(text),
            NamePart::Splice(_) => return None,
        }
    }

    Some(out)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDeclaration {
    pub name: SplicedName,
    pub is_pub: bool,
    /// Optional explicit annotation
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

/// An enum declaration. A variant may be payload-free (`None`) or carry one
/// typed value (`Some: T`).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDeclaration {
    pub name: String,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub variants: Vec<EnumVariantDeclaration>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantDeclaration {
    pub name: String,
    pub payload: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {
    pub name: String,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub facets: Vec<Facet>,
    pub fields: Vec<StructBodyItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDeclaration {
    pub name: String,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub facets: Vec<Facet>,
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
    pub default: Option<Expr>,
    pub span: Span,
}

// ===============
// facets
// ===============
//
// `| name ...` modifiers on declarations. `pub` and `-> Type` are dedicated
// declaration fields and do not appear here. See `crate::facets` for what
// each facet means. What names are valid,
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
