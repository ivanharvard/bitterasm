use std::collections::HashMap;

use crate::ast::{Expr, Program, Statement};
use crate::token::Span;
use crate::types::{GenericParameter, TypeArgument, TypeExpr};

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
enum GenericBinding {
    Type(ResolvedType),
    Const(Option<Expr>),
}

pub struct AliasResolver<'a> {
    program: &'a Program,
    symbols: &'a SymbolTable,

    states: HashMap<SymbolId, AliasState>,
    stack: Vec<SymbolId>,

    generic_scope: HashMap<String, GenericBinding>,
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
    // struct fields
    // ==============

    pub fn resolve_all_structs(
        &mut self
    ) -> Result<HashMap<SymbolId, Vec<ResolvedType>>, ResolveError> {
        let struct_ids: Vec<_> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Struct)
            .map(|symbol| symbol.id)
            .collect();

        let mut resolved = HashMap::new();

        for id in struct_ids {
            let fields = self.resolve_struct_fields(id)?;
            resolved.insert(id, fields);
        }

        Ok(resolved)
    }

    fn resolve_struct_fields(
        &mut self,
        id: SymbolId,
    ) -> Result<Vec<ResolvedType>, ResolveError> {
        let declaration = self.find_struct_declaration(id)?;
        let generic_params = declaration.generic_params.clone();

        let mut scope = HashMap::new();

        for param in &generic_params {
            let binding = match param {
                GenericParameter::Type { name, .. } => {
                    GenericBinding::Type(ResolvedType::TypeParameter { name: name.clone() })
                }

                GenericParameter::Const { .. } => GenericBinding::Const(None),
            };

            scope.insert(param_name(param).to_string(), binding);
        }

        self.resolve_fields_in_scope(id, scope)
    }

    // Resolves a struct's field types under a specific instantiation, e.g.
    // `Reg<64>`, by binding its generic parameters to the actual arguments
    // used at that call site instead of abstract placeholders.
    pub fn instantiate_struct_fields(
        &mut self,
        id: SymbolId,
        args: &[ResolvedGenericArg],
    ) -> Result<Vec<ResolvedType>, ResolveError> {
        let declaration = self.find_struct_declaration(id)?;
        let scope = generic_arg_scope(&declaration.generic_params, args);

        self.resolve_fields_in_scope(id, scope)
    }

    // Resolves the type of a single named field on a (possibly generic)
    // struct type, e.g. `field_type(Reg<64>, "value")` returns `bits<64>`.
    // This is the semantic entry point field access should go through:
    // callers hand over a `ResolvedType` and a field name, without needing
    // to know about symbol IDs or how generic substitution works.
    pub fn field_type(
        &mut self,
        ty: &ResolvedType,
        field_name: &str,
        span: Span,
    ) -> Result<ResolvedType, ResolveError> {
        let ResolvedType::Struct { symbol, args } = ty else {
            return Err(ResolveError::UnknownField {
                type_name: describe_type(ty, self.symbols),
                field: field_name.to_string(),
                span,
            });
        };

        let declaration = self.find_struct_declaration(*symbol)?;

        let Some(field) = declaration
            .fields
            .iter()
            .find(|field| field.name == field_name)
        else {
            return Err(ResolveError::UnknownField {
                type_name: self.symbols.get(*symbol).name.clone(),
                field: field_name.to_string(),
                span,
            });
        };

        let field_ty = field.ty.clone();
        let scope = generic_arg_scope(&declaration.generic_params, args);

        let previous = std::mem::replace(&mut self.generic_scope, scope);
        let result = self.resolve_type_expr(&field_ty);
        self.generic_scope = previous;

        result
    }

    fn resolve_fields_in_scope(
        &mut self,
        id: SymbolId,
        scope: HashMap<String, GenericBinding>,
    ) -> Result<Vec<ResolvedType>, ResolveError> {
        let field_types: Vec<TypeExpr> = self
            .find_struct_declaration(id)?
            .fields
            .iter()
            .map(|field| field.ty.clone())
            .collect();

        let previous = std::mem::replace(&mut self.generic_scope, scope);

        let result = field_types
            .iter()
            .map(|ty| self.resolve_type_expr(ty))
            .collect::<Result<Vec<_>, _>>();

        self.generic_scope = previous;

        result
    }

    // ==============
    // type expressions
    // ==============

    fn resolve_type_expr(
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
            "uint" => return Ok(ResolvedType::Builtin(BuiltinType::Uint)),
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

        if matches!(name.as_str(), "int" | "uint") {
            return Err(ResolveError::ExpectedConstant {
                name: name.clone(),
                span: *span,
            });
        }

        match self.symbols.lookup(name) {
            Some(id) => match self.symbols.get(id).kind {
                SymbolKind::Const => Ok(()),

                SymbolKind::Struct | SymbolKind::TypeAlias => {
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

    fn find_struct_declaration(
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

fn param_name(param: &GenericParameter) -> &str {
    match param {
        GenericParameter::Type { name, .. } => name,
        GenericParameter::Const { name, .. } => name,
    }
}

// Pairs a struct's generic parameters with the resolved arguments supplied
// at a particular use site, e.g. `Reg<64>`, into a scope suitable for
// resolving field types under that instantiation.
fn generic_arg_scope(
    generic_params: &[GenericParameter],
    args: &[ResolvedGenericArg],
) -> HashMap<String, GenericBinding> {
    let mut scope = HashMap::new();

    for (param, arg) in generic_params.iter().zip(args) {
        let binding = match arg {
            ResolvedGenericArg::Type(ty) => GenericBinding::Type((**ty).clone()),
            ResolvedGenericArg::Const(expr) => GenericBinding::Const(Some(expr.clone())),
        };

        scope.insert(param_name(param).to_string(), binding);
    }

    scope
}

// Produces a human-readable name for a resolved type, used in diagnostics
// like `UnknownField` when the type isn't the kind of thing that error
// needs to describe more precisely (e.g. field access on a non-struct).
fn describe_type(ty: &ResolvedType, symbols: &SymbolTable) -> String {
    match ty {
        ResolvedType::Builtin(BuiltinType::Int) => "int".to_string(),
        ResolvedType::Builtin(BuiltinType::Uint) => "uint".to_string(),
        ResolvedType::Struct { symbol, .. } => symbols.get(*symbol).name.clone(),
        ResolvedType::TypeParameter { name } => name.clone(),
    }
}