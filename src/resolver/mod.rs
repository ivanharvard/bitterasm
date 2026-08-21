//! Resolves the flattened [`Program`] produced by [`crate::loader`] against
//! itself: [`collect_symbols`] builds a whole-program [`SymbolTable`] of
//! top-level declarations, and [`AliasResolver`] resolves [`crate::types::TypeExpr`]
//! trees against that table into [`ResolvedType`]s, instantiating generic
//! struct fields along the way. Import resolution has already happened by
//! this point, so nothing here needs to know about modules or files.

mod aliases;
mod consts;
mod facets;
mod structs;
mod symbols;
mod types;

pub use aliases::AliasResolver;
pub use consts::ConstEvaluator;
pub use facets::validate as validate_facets;
pub use symbols::*;
pub use types::*;

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

            Statement::Macro(decl) => {
                table.insert(
                    decl.name.clone(),
                    SymbolKind::Macro,
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

    CyclicConstant {
        cycle: Vec<String>,
        span: Span,
    },

    DivisionByZero {
        span: Span,
    },

    ExpectedType {
        name: String,
        span: Span,
    },

    InvalidGenericArity {
        name: String,
        expected: usize,
        actual: usize,
        span: Span,
    },

    ExpectedConstant {
        name: String,
        span: Span,
    },

    // Like `ExpectedConstant`, but for an expression (`bits<foo()>`,
    // `bits<foo.bar>`) rather than a single identifier that names the
    // wrong kind of thing — there's no single `name` to report.
    ExpectedConstantExpression {
        span: Span,
    },

    UnknownConstant {
        name: String,
        span: Span,
    },

    UnknownField {
        type_name: String,
        field: String,
        span: Span,
    },

    FacetNotApplicable {
        facet: String,
        span: Span,
    },

    DuplicateFacet {
        facet: String,
        span: Span,
    },

    Internal {
        message: String,
        span: Span,
    },
}
