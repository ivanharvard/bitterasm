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
use crate::eval::{self, EvalError, Int};
use crate::token::Span;
use crate::types::{TypeArgument, TypeExpr};

use super::symbols::{SymbolId, SymbolKind, SymbolTable};
use super::types::{
    BuiltinType,
    ResolvedGenericArg,
    ResolvedType,
};
use super::values::ConstValueState;
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
    Const(Option<Int>),
}

pub struct AliasResolver<'a> {
    pub(super) program: &'a Program,
    pub(super) symbols: &'a SymbolTable,
    consts: &'a HashMap<String, Int>,

    states: HashMap<SymbolId, AliasState>,
    stack: Vec<SymbolId>,

    // Lazy-resolve-and-memoize, same shape as `states`/`stack` above, but
    // for a top-level const's fully-evaluated `Value` (struct or `Int`) —
    // see `AliasResolver::resolve_const_value` in `super::values`. Kept
    // separate from `states`/`stack` rather than reusing them: a type
    // alias's own resolution never needs a struct-valued const (only
    // `consts`, the pre-folded `Int`-only table, matters there), so the two
    // recursion chains are independent and shouldn't share one cycle guard.
    pub(super) const_value_states: HashMap<SymbolId, ConstValueState>,
    pub(super) const_value_stack: Vec<SymbolId>,

    pub(super) generic_scope: HashMap<String, GenericBinding>,
}

impl<'a> AliasResolver<'a> {
    /// `consts` is every top-level const already evaluated to an `Int`
    /// (see [`super::ConstEvaluator`]) — needed so a generic const argument
    /// that references one, e.g. `bits<SOME_WIDTH>`, can fold to a
    /// concrete value the same way a literal or arithmetic expression does.
    pub fn new(
        program: &'a Program,
        symbols: &'a SymbolTable,
        consts: &'a HashMap<String, Int>,
    ) -> Self {
        let mut states = HashMap::new();
        let mut const_value_states = HashMap::new();

        for symbol in symbols.iter() {
            if symbol.kind == SymbolKind::TypeAlias {
                states.insert(symbol.id, AliasState::Unvisited);
            }

            if symbol.kind == SymbolKind::Const {
                const_value_states.insert(symbol.id, ConstValueState::Unvisited);
            }
        }

        Self {
            program,
            symbols,
            consts,
            states,
            stack: Vec::new(),
            const_value_states,
            const_value_stack: Vec::new(),
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

    pub(super) fn resolve_named_type(
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
                // A bare reference to a const generic param that isn't
                // bound to a concrete value yet (e.g. resolving a struct's
                // own field types in the abstract, before any particular
                // instantiation) stays symbolic rather than being folded —
                // there's nothing to fold it *to*.
                if let Expr::Identifier { name, .. } = expr {
                    if let Some(GenericBinding::Const(None)) = self.generic_scope.get(name) {
                        return Ok(ResolvedGenericArg::ConstParam(name.clone()));
                    }
                }

                Ok(ResolvedGenericArg::Const(self.eval_const_expr(expr)?))
            }
        }
    }

    fn eval_const_expr(&self, expr: &Expr) -> Result<Int, ResolveError> {
        let mut scope = self.consts.clone();

        for (name, binding) in &self.generic_scope {
            if let GenericBinding::Const(Some(value)) = binding {
                scope.insert(name.clone(), value.clone());
            }
        }

        eval::eval(expr, &scope).map_err(|error| self.into_resolve_error(error))
    }

    fn into_resolve_error(&self, error: EvalError) -> ResolveError {
        match error {
            EvalError::UnknownConstant { name, span } => {
                // `name` names *something* (a struct, a type alias, a
                // macro, a type-generic param, or a const that isn't
                // int-valued) rather than nothing at all — a clearer
                // diagnostic than a bare "unknown" either way.
                let known = self.symbols.lookup(&name).is_some()
                    || self.generic_scope.contains_key(&name);

                if known {
                    ResolveError::ExpectedConstant { name, span }
                } else {
                    ResolveError::UnknownConstant { name, span }
                }
            }

            EvalError::NotConstant { span } => {
                ResolveError::ExpectedConstantExpression { span }
            }

            EvalError::DivisionByZero { span } => ResolveError::DivisionByZero { span },
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

    pub(super) fn find_macro_symbol(
        &self,
        name: &str,
        span: Span,
    ) -> Result<SymbolId, ResolveError> {
        let Some(id) = self.symbols.lookup(name) else {
            return Err(ResolveError::UnknownMacro {
                name: name.to_string(),
                span,
            });
        };

        if self.symbols.get(id).kind != SymbolKind::Macro {
            return Err(ResolveError::ExpectedMacro {
                name: name.to_string(),
                span,
            });
        }

        Ok(id)
    }

    pub(super) fn find_macro_declaration(
        &self,
        id: SymbolId,
    ) -> Result<&crate::ast::MacroDeclaration, ResolveError> {
        let symbol = self.symbols.get(id);

        for statement in &self.program.statements {
            if let Statement::Macro(declaration) = statement {
                if declaration.name == symbol.name {
                    return Ok(declaration);
                }
            }
        }

        // there is some internal compiler inconsistency rather than bad
        // BitterASM source.
        Err(ResolveError::Internal {
            message: format!(
                "symbol table contains macro `{}` but no matching AST declaration exists",
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
