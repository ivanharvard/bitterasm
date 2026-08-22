//! Resolves struct field types against a particular instantiation of a
//! (possibly generic) struct, e.g. binding `Reg<64>`'s `width` to `64`
//! before resolving `Reg`'s field types under that substitution. Builds on
//! [`super::aliases`]'s [`AliasResolver::resolve_type_expr`] and
//! [`AliasResolver::find_struct_declaration`] — this file only adds the
//! struct-specific instantiation logic on top.

use std::collections::HashMap;

use crate::token::Span;
use crate::types::GenericParameter;

use super::aliases::{AliasResolver, GenericBinding};
use super::symbols::{SymbolId, SymbolKind, SymbolTable};
use super::types::{BuiltinType, ResolvedGenericArg, ResolvedType};
use super::ResolveError;

impl<'a> AliasResolver<'a> {
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
        let field_types: Vec<crate::types::TypeExpr> = self
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
            ResolvedGenericArg::Const(value) => GenericBinding::Const(Some(value.clone())),
            // Passing an unbound param through as an argument (e.g. one
            // generic struct instantiating another with its own still-open
            // param) stays unbound in the new scope too.
            ResolvedGenericArg::ConstParam(_) => GenericBinding::Const(None),
        };

        scope.insert(param_name(param).to_string(), binding);
    }

    scope
}

// Produces a human-readable name for a resolved type, used in diagnostics
// like `UnknownField` when the type isn't the kind of thing that error
// needs to describe more precisely (e.g. field access on a non-struct).
pub(super) fn describe_type(ty: &ResolvedType, symbols: &SymbolTable) -> String {
    match ty {
        ResolvedType::Builtin(BuiltinType::Int) => "int".to_string(),
        ResolvedType::Struct { symbol, .. } => symbols.get(*symbol).name.clone(),
        ResolvedType::TypeParameter { name } => name.clone(),
    }
}
