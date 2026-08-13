use std::collections::HashMap;

use crate::ast::{Expr, Program, Statement};
use crate::ast::Expr::Integer;
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

pub struct AliasResolver<'a> {
    program: &'a Program,
    symbols: &'a SymbolTable,

    states: HashMap<SymbolId, AliasState>,
    stack: Vec<SymbolId>,
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
        if let TypeExpr::Named { path, .. } = base {
            if path.len() == 1 && path[0] == "bits" {
                return self.resolve_bits(args, span);
            }
        }

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
    // bits<N>
    // ==============

    fn resolve_bits(
        &mut self,
        args: &[TypeArgument],
        span: Span
    ) -> Result<ResolvedType, ResolveError> {
        if args.len() != 1 {
            return Err(ResolveError::InvalidGenericArity {
                name: "bits".to_string(),
                expected: 1,
                actual: args.len(),
                span,
            })
        }

        let arg = &args[0];

        let TypeArgument::Const(expr) = arg else {
            return Err(ResolveError::ExpectedConstGeneric {
                name: "bits".to_string(),
                span,
            });
        };

        let Integer { raw, ..} = expr else {
            return Err(ResolveError::ExpectedConstGeneric {
                name: "bits".to_string(),
                span,
            });
        };

        let width = parse_integer_literal(raw).ok_or_else(|| {
            ResolveError::InvalidBitWidth { 
                raw: raw.clone(), 
                span: expr.span()
            }
        })?;

        if width == 0 {
            return Err(ResolveError::InvalidBitWidth { 
                raw: raw.clone(), 
                span: expr.span()
            });
        }

        Ok(ResolvedType::Builtin(BuiltinType::Bits { width }))
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

fn parse_integer_literal(raw: &str) -> Option<u64> {
    let cleaned = raw.replace("_", "");

    if let Some(value) = cleaned.strip_prefix("0x") {
        u64::from_str_radix(value, 16).ok()
    } else if let Some(value) = cleaned.strip_prefix("0X") {
        u64::from_str_radix(value, 16).ok()
    } else if let Some(value) = cleaned.strip_prefix("0b") {
        u64::from_str_radix(value, 2).ok()
    } else if let Some(value) = cleaned.strip_prefix("0B") {
        u64::from_str_radix(value, 2).ok()
    } else if let Some(value) = cleaned.strip_prefix("0o") {
        u64::from_str_radix(value, 8).ok()
    } else if let Some(value) = cleaned.strip_prefix("0O") {
        u64::from_str_radix(value, 8).ok()
    } else {
        cleaned.parse::<u64>().ok()
    }
}