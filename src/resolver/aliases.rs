//! Resolves [`crate::types::TypeExpr`] trees — struct field types, type
//! alias targets, and generic arguments — against a [`SymbolTable`], via
//! [`AliasResolver`]. Type aliases are resolved lazily and memoized per
//! symbol (see [`AliasState`]), with [`AliasState::Visiting`] used to
//! detect and reject reference cycles (`type A = B; type B = A`) instead of
//! recursing forever.
//!
//! Type-expression resolution ([`AliasResolver::resolve_type_expr`] and its
//! helpers) lives here rather than alongside struct-field instantiation
//! ([`super::structs`]) because it's mutually recursive with alias
//! resolution: resolving a named type may resolve an alias
//! ([`AliasResolver::resolve_alias`]), which in turn resolves the alias's
//! own target type expression.

use std::collections::HashMap;

use crate::ast::{Expr, Program, Statement};
use crate::token::Span;
use crate::types::{TypeArgument, TypeExpr};

use super::symbols::{SymbolId, SymbolKind, SymbolTable};
use super::types::{
    BuiltinType,
    ResolvedGenericArg,
    ResolvedType,
};
use super::ResolveError;

#[derive(Debug, Clone)]
enum AliasState {
    Unvisited,
    Visiting,
    Resolved(ResolvedType),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum GenericBinding {
    Type(ResolvedType),
    Const(Option<Expr>),
}

pub struct AliasResolver<'a> {
    pub(super) program: &'a Program,
    pub(super) symbols: &'a SymbolTable,

    states: HashMap<SymbolId, AliasState>,
    stack: Vec<SymbolId>,

    pub(super) generic_scope: HashMap<String, GenericBinding>,
}

impl<'a> AliasResolver<'a> {
    pub fn new(
        program: &'a Program,
        symbols: &'a SymbolTable,
    ) -> Self {
        let mut states = HashMap::new();

        for symbol in symbols.iter() {
            if symbol.kind == SymbolKind::TypeAlias {
                states.insert(symbol.id, AliasState::Unvisited);
            }
        }

        Self {
            program,
            symbols,
            states,
            stack: Vec::new(),
            generic_scope: HashMap::new(),
        }
    }

    pub fn resolve_all(
        &mut self
    ) -> Result<HashMap<SymbolId, ResolvedType>, ResolveError> {
        let alias_ids: Vec<_> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::TypeAlias)
            .map(|symbol| symbol.id)
            .collect();

        let mut resolved = HashMap::new();

        for id in alias_ids {
            let ty = self.resolve_alias(id)?;
            resolved.insert(id, ty);
        }

        Ok(resolved)
    }

    pub fn resolve_alias(
        &mut self,
        id: SymbolId
    ) -> Result<ResolvedType, ResolveError> {
        match self.states.get(&id) {
            Some(AliasState::Resolved(ty)) => return Ok(ty.clone()),
            Some(AliasState::Visiting) => return Err(self.make_cycle_error(id)),
            Some(AliasState::Unvisited) => {}
            None => {
                let symbol = self.symbols.get(id);

                return Err(ResolveError::ExpectedType {
                    name: symbol.name.clone(),
                    span: symbol.span
                })
            }
        }

        self.states.insert(id, AliasState::Visiting);
        self.stack.push(id);

        let declaration_ty = self.find_alias_declaration(id)?.ty.clone();

        let result = self.resolve_type_expr(&declaration_ty);

        self.stack.pop();

        match result {
            Ok(ty) => {
                self.states.insert(id, AliasState::Resolved(ty.clone()));
                Ok(ty)
            }
            Err(err) => {
                self.states.insert(
                    id,
                    AliasState::Unvisited,
                );

                Err(err)
            }
        }
    }

    // ==============
    // type expressions
    // ==============

    pub(super) fn resolve_type_expr(
        &mut self,
        ty: &TypeExpr
    ) -> Result<ResolvedType, ResolveError> {
        match ty {
            TypeExpr::Named { path, span} => {
                self.resolve_named_type(path, *span)
            }

            TypeExpr::Apply { base, args, span} => {
                self.resolve_applied_type(base, args, *span)
            }
        }
    }

    fn resolve_named_type(
        &mut self,
        path: &[String],
        span: Span
    ) -> Result<ResolvedType, ResolveError> {
        if path.len() != 1 {
            return Err(ResolveError::UnknownType {
                name: path.join("."),
                span,
            });
        }

        let name = &path[0];

        // generic parameters in scope shadow builtins and declared symbols
        match self.generic_scope.get(name) {
            Some(GenericBinding::Type(ty)) => return Ok(ty.clone()),

            Some(GenericBinding::Const(_)) => {
                return Err(ResolveError::ExpectedType {
                    name: name.clone(),
                    span,
                });
            }

            None => {}
        }

        // builtins
        match name.as_str() {
            "int" => return Ok(ResolvedType::Builtin(BuiltinType::Int)),
            _ => {}
        }

        let Some(id) = self.symbols.lookup(name) else {
            return Err(ResolveError::UnknownType {
                name: name.clone(),
                span,
            });
        };

        let symbol = self.symbols.get(id);

        match symbol.kind {
            SymbolKind::Struct => {
                Ok(ResolvedType::Struct {
                    symbol: id,
                    args: Vec::new(),
                })
            }

            SymbolKind::TypeAlias => {
                self.resolve_alias(id)
            }

            SymbolKind::Const => {
                Err(ResolveError::ExpectedType {
                    name: name.clone(),
                    span,
                })
            }

            SymbolKind::Macro => {
                Err(ResolveError::ExpectedType {
                    name: name.clone(),
                    span,
                })
            }
        }
    }

    fn resolve_applied_type(
        &mut self,
        base: &TypeExpr,
        args: &[TypeArgument],
        span: Span
    ) -> Result<ResolvedType, ResolveError> {
        let base_type = self.resolve_type_expr(base)?;

        match base_type {
            ResolvedType::Struct { symbol, .. } => {
                let expected = self.find_struct_declaration(symbol)?.generic_params.len();

                if args.len() != expected {
                    let name = self.symbols.get(symbol).name.clone();

                    return Err(ResolveError::InvalidGenericArity {
                        name,
                        expected,
                        actual: args.len(),
                        span,
                    });
                }

                let resolved_args = args
                    .iter()
                    .map(|arg| self.resolve_generic_args(arg))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(ResolvedType::Struct {
                    symbol,
                    args: resolved_args,
                })
            }

            ResolvedType::Builtin(_) => {
                let name = base
                    .name()
                    .unwrap_or("<type>")
                    .to_string();

                Err(ResolveError::ExpectedType {
                    name,
                    span,
                })
            }

            ResolvedType::TypeParameter { name } => {
                Err(ResolveError::ExpectedType {
                    name,
                    span,
                })
            }
        }
    }

    fn resolve_generic_args(
        &mut self,
        arg: &TypeArgument
    ) -> Result<ResolvedGenericArg, ResolveError> {
        match arg {
            TypeArgument::Type(ty) => {
                Ok(ResolvedGenericArg::Type(
                    Box::new(self.resolve_type_expr(ty)?),
                ))
            }

            TypeArgument::Const(expr) => {
                self.check_const_expr(expr)?;
                Ok(ResolvedGenericArg::Const(expr.clone()))
            }
        }
    }

    fn check_const_expr(
        &self,
        expr: &Expr,
    ) -> Result<(), ResolveError> {
        let Expr::Identifier { name, span } = expr else {
            return Ok(());
        };

        // generic parameters in scope shadow builtins and declared symbols
        match self.generic_scope.get(name) {
            Some(GenericBinding::Const(_)) => return Ok(()),

            Some(GenericBinding::Type(_)) => {
                return Err(ResolveError::ExpectedConstant {
                    name: name.clone(),
                    span: *span,
                });
            }

            None => {}
        }

        if name == "int" {
            return Err(ResolveError::ExpectedConstant {
                name: name.clone(),
                span: *span,
            });
        }

        match self.symbols.lookup(name) {
            Some(id) => match self.symbols.get(id).kind {
                SymbolKind::Const => Ok(()),

                SymbolKind::Struct | SymbolKind::TypeAlias | SymbolKind::Macro => {
                    Err(ResolveError::ExpectedConstant {
                        name: name.clone(),
                        span: *span,
                    })
                }
            },

            None => Err(ResolveError::UnknownConstant {
                name: name.clone(),
                span: *span,
            }),
        }
    }

    // ==============
    // ast lookup
    // ==============

    fn find_alias_declaration(
        &self,
        id: SymbolId,
    ) -> Result<&crate::ast::TypeAliasDeclaration, ResolveError> {
        let symbol = self.symbols.get(id);

        for statement in &self.program.statements {
            if let Statement::TypeAlias(declaration) = statement {
                if declaration.name == symbol.name {
                    return Ok(declaration);
                }
            }
        }

        // there is some internal compiler inconsistency rather than bad
        // BitterASM source.
        Err(ResolveError::Internal {
            message: format!(
                "symbol table contains alias `{}` but no matching AST declaration exists",
                symbol.name,
            ),
            span: symbol.span,
        })
    }

    pub(super) fn find_struct_declaration(
        &self,
        id: SymbolId,
    ) -> Result<&crate::ast::StructDeclaration, ResolveError> {
        let symbol = self.symbols.get(id);

        for statement in &self.program.statements {
            if let Statement::Struct(declaration) = statement {
                if declaration.name == symbol.name {
                    return Ok(declaration);
                }
            }
        }

        // there is some internal compiler inconsistency rather than bad
        // BitterASM source.
        Err(ResolveError::Internal {
            message: format!(
                "symbol table contains struct `{}` but no matching AST declaration exists",
                symbol.name,
            ),
            span: symbol.span,
        })
    }

    // ==============
    // diagnostics
    // ==============

    fn make_cycle_error(
        &self,
        repeated: SymbolId,
    ) -> ResolveError {
        let start = self
            .stack
            .iter()
            .position(|id| *id == repeated)
            .unwrap_or(0);

        let mut cycle: Vec<String> = self.stack[start..]
            .iter()
            .map(|id| self.symbols.get(*id).name.clone())
            .collect();

        // Close the loop:
        //
        //     A -> B -> C -> A
        cycle.push(
            self.symbols.get(repeated).name.clone()
        );

        ResolveError::CyclicTypeAlias {
            cycle,
            span: self.symbols.get(repeated).span,
        }
    }

}
