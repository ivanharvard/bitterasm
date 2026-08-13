use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Import(ImportStatement),
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaStatement {
    pub name: String,
    pub args: Vec<Expr>,
    pub body: Option<Vec<Statement>>,
    pub span: Span,
}
