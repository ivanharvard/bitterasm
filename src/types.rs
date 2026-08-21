//! Type-expression AST shared by the parser (as written in source, via
//! [`TypeExpr`]) and the resolver (as resolved against declared symbols,
//! via [`crate::resolver::ResolvedType`]). Kept separate from the rest of
//! [`crate::ast`] because both the parser and resolver need to build and
//! walk these trees in ways that don't apply to any other AST node.

use crate::ast::Expr;
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
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}