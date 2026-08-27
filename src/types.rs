//! Type-expression AST shared by the parser (as written in source, via
//! [`TypeExpr`]) and the resolver (as resolved against declared symbols,
//! via [`crate::resolver::ResolvedType`]). Kept separate from the rest of
//! [`crate::ast`] because both the parser and resolver need to build and
//! walk these trees in ways that don't apply to any other AST node.

use crate::ast::{Expr, SplicedName};
use crate::token::Span;

// ===============
// type expressions
// ===============

// A type as written in BitterASM.
// Examples:
// 
//   Reg
//   foo.Reg
//   Reg<64>
//   bits<8>
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    // Named type, e.g., `Reg` or `foo.Reg`.
    Named {
        path: Vec<String>,
        span: Span,
    },

    // Applications of generic arguments, e.g., `Reg<64>` or `bits<8>`.
    Apply {
        base: Box<TypeExpr>,
        args: Vec<TypeArgument>,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named { span, .. }
            | TypeExpr::Apply { span, .. } => *span,
        }
    }

    /// The type's own name, ignoring any module-path qualification and any
    /// applied generic arguments — `foo.bar.Reg<64>` and `Reg` both give
    /// `"Reg"`. Used to look up a generic signature so the parser knows
    /// whether `<...>` arguments are types or consts; see
    /// [`crate::parser::parse_seeded`].
    ///
    /// ```
    /// use bitterasm::token::Span;
    /// use bitterasm::types::TypeExpr;
    ///
    /// let span = Span::new(0, 0);
    /// let ty = TypeExpr::Named {
    ///     path: vec!["foo".to_string(), "Reg".to_string()],
    ///     span,
    /// };
    ///
    /// assert_eq!(ty.name(), Some("Reg"));
    /// ```
    pub fn name(&self) -> Option<&str> {
        match self {
            TypeExpr::Named { path, .. } => path.last().map(String::as_str),
            TypeExpr::Apply { base, .. } => base.name(),
        }
    }
}

// ===============
// generic arguments
// ===============

#[derive(Debug, Clone, PartialEq)]
pub enum TypeArgument {
    Type(TypeExpr),
    Const(Expr),
    /// `...` in a signature accepts any argument in this position without
    /// binding it (for example `Array<T, ...>`).
    Wildcard(Span),
}

// ===============
// generic parameters
// ===============

#[derive(Debug, Clone, PartialEq)]
pub enum GenericParameter {
    Const {
        name: String,
        ty: TypeExpr,
        span: Span,
    },

    Type {
        name: String,
        span: Span,
    }
}

// ===============
// struct fields
// ===============

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: SplicedName,
    pub ty: TypeExpr,

    /// Whether `@for i in X` (`X` resolving to this struct's type) visits
    /// this field — a non-`pub` field stays internal bookkeeping (e.g.
    /// whatever an `invariant` needs) even when the rest of the struct is
    /// iterated. See `resolver::generated::eval_for_source`.
    pub is_pub: bool,

    /// `const name: Type` — parsed, not yet enforced (no additional
    /// resolver checking beyond ordinary type-checking). See
    /// `std/tinycpu/native.basm`'s `pub const id: bits<2>`.
    pub is_const: bool,

    /// `= expr` — evaluated (against the struct's own bound generic const
    /// args, not sibling field values) and used only when this field is
    /// omitted from a paren-call or brace-literal construction.
    pub default: Option<Expr>,

    pub span: Span,
}

/// One item in a struct declaration's body — either a field written
/// directly, or an `@for`/`@if` that generates zero or more fields once
/// its range/condition can be evaluated (e.g. `Array<T, N>`'s
/// `@for i in 0..N { pub __el\`i\`: T, }`). Deliberately its own type
/// rather than reusing `ast::MetaStatement`/`ast::Statement`: a struct
/// body's items are field-shaped, not statement-shaped, so `@for`/`@if`
/// here carry a nested `Vec<StructBodyItem>` body instead of
/// `Vec<Statement>`.
#[derive(Debug, Clone, PartialEq)]
pub enum StructBodyItem {
    Field(StructField),

    For {
        var: String,
        source: Expr,
        body: Vec<StructBodyItem>,
        span: Span,
    },

    If {
        condition: Expr,
        body: Vec<StructBodyItem>,
        else_body: Option<Vec<StructBodyItem>>,
        span: Span,
    },
}
