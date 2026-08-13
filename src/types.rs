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