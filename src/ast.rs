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

use crate::token::{Span, TokenKind};
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
    ExternLabel(ExternLabel),
    Section(Section),
    Invocation(Invocation),

    Macro(MacroDeclaration),

    /// `syntax name(a, b) = { pattern }` — assigns/overrides how `name`
    /// (an existing macro, not declared here) is recognized at call sites
    /// for the rest of this file. Declares nothing: no new symbol, no
    /// change to `name`'s own declaration. Fully consumed by parsing and
    /// loader-level import propagation (`crate::parser::ParserSeed`,
    /// `crate::loader`) — the resolver never needs to look at it (an
    /// override naming a macro that doesn't actually exist just surfaces
    /// as an ordinary `UnknownMacro` wherever its call-site shape gets
    /// used, the same as any other undefined reference).
    SyntaxOverride(SyntaxOverrideStatement),

    Meta(MetaStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxOverrideStatement {
    pub name: String,
    pub pattern: crate::facets::syntax::SyntaxPattern,
    pub span: Span,
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
    pub is_pub: bool,
    pub span: Span,
}

/// A `from file import label_name` where `label_name` names a `pub` label
/// in `file` — synthesized by `crate::loader::splice_import` in place of
/// the ordinary whole-declaration splice every other imported kind gets
/// (see `docs/sections-and-linking/PROGRESS.md`'s Phase 5: labels defer
/// instead of splice, since `label_name`'s numeric position isn't known
/// here — only that it exists and is `pub`, checked at import time the
/// same way any other imported name already is). `file` is the already-
/// canonicalized absolute path of the file that declared `label_name`.
/// Registered into the symbol table as `SymbolKind::ExternLabel`, never
/// `SymbolKind::Label` — referencing it resolves to a
/// `Value::ExternLabel`, not a plain `Value::Int`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternLabel {
    pub name: String,
    pub file: String,
    pub span: Span,
}

/// `section <name>` — reopens (or, on first use, opens) a named group that
/// subsequent `@emit`s land in, until the next `section` statement. Pure
/// naming: the name (e.g. `.text`, `.rodata`, `data.rel`) carries no
/// interpreted meaning to `bitterasm` or `bitter`'s core — see
/// `docs/sections-and-linking/PROGRESS.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
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

    /// `` acc_`i` `` in expression position: a name built from literal
    /// text and evaluated splices, read exactly like an [`Expr::Identifier`]
    /// once its name is resolved. Only formed when the backtick touches
    /// the identifier — `` acc `i` `` stays two separate tokens.
    SplicedIdentifier {
        name: SplicedName,
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

    /// `start..end` (`inclusive: false`) or `start..=end` (`inclusive:
    /// true`) — range sugar, upper bound exclusive or inclusive
    /// respectively. Not a value on its own (evaluating one directly is
    /// `EvalError::NotConstant`); the only place it evaluates to something
    /// is `resolver::values::eval_value`, which turns it into a synthesized
    /// `Value::Struct` with one pub field per element (see
    /// `resolver::generated::eval_range_value`) — `@for`'s four call sites
    /// all consume it that way, uniformly with any other struct-valued
    /// `in`-expression.
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },

    /// `value in source` — true when `source` (a literal range or any
    /// expression evaluating to a struct/array value) produces some
    /// element equal to `value`. The same "does `source` contain this"
    /// relation `@for var in source` walks generatively (binding each
    /// element in turn); this is that relation tested instead of walked —
    /// see `resolver::values::eval_value`'s `Expr::In` arm, which shares
    /// `resolver::generated::eval_for_source` with `@for` for exactly this
    /// reason. Deliberately scoped to ranges and struct/array values only:
    /// there's no `x in SomeEnumType` (membership against a *type* rather
    /// than a value in hand) here, to keep `in`'s meaning uniform.
    In {
        value: Box<Expr>,
        source: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Identifier { span, .. }
            | Expr::SplicedIdentifier { span, .. }
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
            | Expr::Range { span, .. }
            | Expr::In { span, .. } => *span,
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

/// The name `parts` spells when every splice is an integer literal — what
/// top-level `@for` unrolling leaves behind (`` r`3` ``) — so a pass that
/// runs before evaluation can still see which name it reads. `None` when a
/// splice still needs evaluating.
pub fn literal_spliced_name(parts: &[NamePart]) -> Option<String> {
    let mut out = String::new();

    for part in parts {
        match part {
            NamePart::Literal(text) => out.push_str(text),
            NamePart::Splice(Expr::Integer { raw, .. }) => out.push_str(raw),
            NamePart::Splice(_) => return None,
        }
    }

    Some(out)
}

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
    pub name: SplicedName,
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
    pub name: SplicedName,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub facets: Vec<Facet>,
    pub fields: Vec<StructBodyItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDeclaration {
    pub name: SplicedName,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
    pub facets: Vec<Facet>,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDeclaration {
    pub name: SplicedName,
    pub is_pub: bool,
    pub generic_params: Vec<GenericParameter>,
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

    /// `{ ... }`'s raw tokens (balanced braces, not otherwise parsed) —
    /// `syntax`'s payload shape (`PayloadShape::Pattern`).
    Pattern(Vec<TokenKind>),
}
