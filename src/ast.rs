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
            | Expr::Binary { span, .. } => *span,
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