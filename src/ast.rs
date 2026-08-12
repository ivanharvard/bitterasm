#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Import(Import),
    Label(Label),
    Instruction(Instruction),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub module: Vec<String>,
    pub names: ImportNames,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportNames {
    All,
    Names(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub name: String,
    pub operands: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier(String),
    Integer(i64),
}

