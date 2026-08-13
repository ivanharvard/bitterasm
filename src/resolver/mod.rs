mod aliases;
mod symbols;

pub use symbols::*;

use crate::ast::{Program, Statement};
use crate::token::Span;

pub fn collect_symbols(program: &Program) -> Result<SymbolTable, ResolveError> {
    let mut table = SymbolTable::new();

    for statement in &program.statements {
        let result = match statement {
            Statement::Struct(decl) => {
                table.insert(
                    decl.name.clone(),
                    SymbolKind::Struct,
                    decl.span,
                )
            }

            Statement::TypeAlias(decl) => {
                table.insert(
                    decl.name.clone(),
                    SymbolKind::TypeAlias,
                    decl.span,
                )
            }

            Statement::Const(decl) => {
                table.insert(
                    decl.name.clone(),
                    SymbolKind::Const,
                    decl.span,
                )
            }

            _ => continue,
        };

        result.map_err(|duplicate| {
            ResolveError::DuplicateSymbol {
                name: duplicate.name,
                span: duplicate.span,
            }
        })?;
    }

    Ok(table)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    UnknownType {
        name: String,
        span: Span,
    },

    DuplicateSymbol {
        name: String,
        span: Span,
    },

    CyclicTypeAlias {
        cycle: Vec<String>,
        span: Span,
    },

    ExpectedType {
        name: String,
        span: Span,
    },
}