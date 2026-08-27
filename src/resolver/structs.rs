//! Resolves struct field types against a particular instantiation of a
//! (possibly generic) struct, e.g. binding `Reg<64>`'s `width` to `64`
//! before resolving `Reg`'s field types under that substitution. Builds on
//! [`super::aliases`]'s [`AliasResolver::resolve_type_expr`] and
//! [`AliasResolver::find_struct_declaration`] — this file only adds the
//! struct-specific instantiation logic on top.
//!
//! A struct body isn't always a flat field list — `@for`/`@if` items
//! ([`StructBodyItem`]) generate zero or more fields once their
//! range/condition can be evaluated (e.g. `Array<T, N>`'s
//! `@for i in 0..N { pub __el\`i\`: T, }`). [`AliasResolver::unroll_struct_body`]
//! expands those into concrete, literally-named fields, given
//! `self.generic_scope` already reflects the instantiation to unroll
//! against — every caller here sets that up first (an abstract, unbound
//! scope for [`resolve_struct_fields`]/[`AliasResolver::resolve_all_structs`],
//! a concrete one for [`AliasResolver::instantiate_struct_fields`]/[`AliasResolver::field_type`]).

use std::collections::HashMap;

use crate::ast::{literal_name, Expr, NamePart};
use crate::eval::Int;
use crate::facets;
use crate::token::Span;
use crate::types::{GenericParameter, StructBodyItem, TypeExpr};

use super::aliases::{AliasResolver, GenericBinding};
use super::symbols::{SymbolId, SymbolKind, SymbolTable};
use super::types::{BuiltinType, ResolvedGenericArg, ResolvedType};
use super::values::Value;
use super::ResolveError;

/// A struct body item, fully unrolled down to a concrete field: a
/// literal (never-spliced) name paired with its still-unresolved
/// [`TypeExpr`] — see the module doc.
struct UnrolledField {
    name: String,
    ty: TypeExpr,
    is_pub: bool,
    default: Option<Expr>,
}

impl<'a> AliasResolver<'a> {
    pub(super) fn instantiate_enum_payload(
        &mut self,
        id: SymbolId,
        args: &[ResolvedGenericArg],
        variant_name: &str,
        span: Span,
    ) -> Result<Option<ResolvedType>, ResolveError> {
        let declaration = self.find_enum_declaration(id)?;
        let scope = generic_arg_scope(&declaration.generic_params, args);
        let variant = declaration
            .variants
            .iter()
            .find(|variant| variant.name == variant_name)
            .cloned()
            .ok_or_else(|| ResolveError::UnknownField {
                type_name: declaration.name.clone(),
                field: variant_name.to_string(),
                span,
            })?;
        let previous = std::mem::replace(&mut self.generic_scope, scope);
        let result = variant.payload.as_ref().map(|ty| self.resolve_type_expr(ty)).transpose();
        self.generic_scope = previous;
        result
    }
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

    // Like `instantiate_struct_fields`, but keeps each field's name
    // (plus its `is_pub`/`default`) alongside its resolved type instead of
    // discarding them — brace-literal construction
    // (`resolver::values::eval_construct_value`) needs the name to match a
    // declared field against a provided one and the type to check the
    // provided value against it; `is_pub` is what `eval_for_source`
    // (`struct_field_pub_flags`, just below) filters `@for` on, and
    // `default` is what a construction falls back to when a field's
    // omitted.
    pub(super) fn instantiate_struct_fields_named(
        &mut self,
        id: SymbolId,
        args: &[ResolvedGenericArg],
    ) -> Result<Vec<(String, ResolvedType, bool, Option<Expr>)>, ResolveError> {
        let declaration = self.find_struct_declaration(id)?;
        let scope = generic_arg_scope(&declaration.generic_params, args);
        let items = declaration.fields.clone();

        let previous = std::mem::replace(&mut self.generic_scope, scope);

        let result = match self.unroll_struct_body(&items) {
            Ok(fields) => fields
                .into_iter()
                .map(|field| {
                    let ty = self.resolve_type_expr(&field.ty)?;
                    Ok((field.name, ty, field.is_pub, field.default))
                })
                .collect(),
            Err(error) => Err(error),
        };

        self.generic_scope = previous;

        result
    }

    // Thin sibling of `instantiate_struct_fields_named` that only the
    // `is_pub` flag survives from — `resolver::generated::eval_for_source`
    // uses this to filter a struct value's fields down to the ones `@for`
    // should actually visit, without needing to resolve every field's full
    // type just to read one bit off it.
    pub(super) fn struct_field_pub_flags(
        &mut self,
        id: SymbolId,
        args: &[ResolvedGenericArg],
    ) -> Result<Vec<bool>, ResolveError> {
        Ok(self
            .instantiate_struct_fields_named(id, args)?
            .into_iter()
            .map(|(_, _, is_pub, _)| is_pub)
            .collect())
    }

    // Checks every `invariant` facet (`crate::facets::invariant`) declared
    // on `symbol`'s struct against a construction that just produced
    // `fields` under `args` — called from both `values::eval_call_value`
    // and `values::eval_construct_value`, right before they hand back the
    // `Value::Struct` they built. `bits<const width: int> | invariant
    // fits_inside_width(width, value) { value: int }` is the motivating
    // case: `width` comes from `args` (bound to the generic param it
    // instantiates), `value` from `fields` (bound to the field it names) —
    // together they're exactly the scope `fits_inside_width(width, value)`
    // needs to evaluate.
    pub(super) fn check_struct_invariants(
        &mut self,
        symbol: SymbolId,
        args: &[ResolvedGenericArg],
        fields: &[(String, Value)],
        span: Span,
    ) -> Result<(), ResolveError> {
        let declaration = self.find_struct_declaration(symbol)?;
        let invariants = facets::extract_invariants(&declaration.facets);

        if invariants.is_empty() {
            return Ok(());
        }

        let generic_params = declaration.generic_params.clone();

        let mut scope: HashMap<String, Value> = HashMap::new();

        for (param, arg) in generic_params.iter().zip(args) {
            if let (param @ GenericParameter::Const { name, .. }, ResolvedGenericArg::Const(value)) =
                (param, arg)
            {
                scope.insert(name.clone(), self.const_generic_value(param, value));
            }
        }

        for (name, value) in fields {
            scope.insert(name.clone(), value.clone());
        }

        scope.insert(
            "source".to_string(),
            Value::Struct {
                symbol,
                args: args.to_vec(),
                fields: fields.to_vec(),
                nominal: None,
            },
        );

        for invariant in &invariants {
            if !self.eval_truthy(invariant, &scope)? {
                return Err(ResolveError::InvariantViolated {
                    type_name: self.get_symbol(symbol).name.clone(),
                    invariant: crate::printer::print_expr(invariant),
                    span,
                });
            }
        }

        Ok(())
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
        // Field access reaches through nominal wrapping transparently —
        // only type-*equality* checks (macro param binding, construction
        // field checks) care whether a value went through the checked gate
        // a nominal alias requires; a field genuinely on the struct
        // underneath is still just there.
        let ResolvedType::Struct { symbol, args } = ty.strip_alias() else {
            return Err(ResolveError::UnknownField {
                type_name: describe_type(ty, self.symbols),
                field: field_name.to_string(),
                span,
            });
        };

        let declaration = self.find_struct_declaration(*symbol)?;
        let items = declaration.fields.clone();
        let scope = generic_arg_scope(&declaration.generic_params, args);

        let previous = std::mem::replace(&mut self.generic_scope, scope);

        let result = match self.unroll_struct_body(&items) {
            Ok(fields) => match fields.into_iter().find(|field| field.name == field_name) {
                Some(field) => self.resolve_type_expr(&field.ty),

                None => Err(ResolveError::UnknownField {
                    type_name: self.get_symbol(*symbol).name.clone(),
                    field: field_name.to_string(),
                    span,
                }),
            },

            Err(error) => Err(error),
        };

        self.generic_scope = previous;

        result
    }

    fn resolve_fields_in_scope(
        &mut self,
        id: SymbolId,
        scope: HashMap<String, GenericBinding>,
    ) -> Result<Vec<ResolvedType>, ResolveError> {
        let items = self.find_struct_declaration(id)?.fields.clone();

        let previous = std::mem::replace(&mut self.generic_scope, scope);

        let result = match self.unroll_struct_body(&items) {
            Ok(fields) => fields.iter().map(|field| self.resolve_type_expr(&field.ty)).collect(),
            Err(error) => Err(error),
        };

        self.generic_scope = previous;

        result
    }

    // Expands every `@for`/`@if` item in a struct body into concrete
    // fields with fully-literal names, given `self.generic_scope` already
    // reflects the instantiation to unroll against. A `@for`/`@if` whose
    // range bound or condition is a bare reference to a still-unbound
    // generic const (the abstract, not-yet-instantiated case — e.g.
    // resolving `Array<T, N>` on its own, before any real `Array<u8, 4>`
    // use) can't be unrolled without a concrete value, so it's skipped
    // rather than erroring: whatever fields it *would* generate simply
    // aren't visible abstractly, the same way a field's own *type*
    // (`bits<N>`) doesn't force `N` to a concrete value either.
    fn unroll_struct_body(
        &mut self,
        items: &[StructBodyItem],
    ) -> Result<Vec<UnrolledField>, ResolveError> {
        let mut fields = Vec::new();

        for item in items {
            match item {
                StructBodyItem::Field(field) => {
                    let name = self.resolve_spliced_name_as_const(&field.name)?;
                    fields.push(UnrolledField {
                        name,
                        ty: field.ty.clone(),
                        is_pub: field.is_pub,
                        default: field.default.clone(),
                    });
                }

                StructBodyItem::For { var, source, body, span } => {
                    if references_unbound_generic(source, &self.generic_scope) {
                        continue;
                    }

                    // Struct-body `@for` stays in the const-generic world:
                    // it has no general `Value`-scope, only whatever's
                    // currently bound in `self.generic_scope`, so build a
                    // throwaway `Value` scope out of that (dropping
                    // `Type`/still-unbound `Const(None)` entries, which have
                    // no `Value` form) to evaluate `source` against, and
                    // require every yielded element to be an `Int` — a
                    // struct-valued element doesn't fit this model (see
                    // `references_unbound_generic`'s doc for why this isn't
                    // extended further).
                    let value_scope: HashMap<String, Value> = self
                        .generic_scope
                        .iter()
                        .filter_map(|(name, binding)| match binding {
                            GenericBinding::Const(Some(i)) => Some((name.clone(), Value::Int(i.clone()))),
                            _ => None,
                        })
                        .collect();

                    let bindings = self.eval_for_source(source, &value_scope)?;

                    for (_, value) in bindings {
                        let Value::Int(i) = value else {
                            return Err(ResolveError::ExpectedIntValue { span: *span });
                        };

                        let previous = self
                            .generic_scope
                            .insert(var.clone(), GenericBinding::Const(Some(i)));

                        let nested = self.unroll_struct_body(body);

                        match previous {
                            Some(previous) => {
                                self.generic_scope.insert(var.clone(), previous);
                            }
                            None => {
                                self.generic_scope.remove(var);
                            }
                        }

                        fields.extend(nested?);
                    }
                }

                StructBodyItem::If { condition, body, else_body, .. } => {
                    if references_unbound_generic(condition, &self.generic_scope) {
                        continue;
                    }

                    let truthy = self.eval_const_expr(condition)? != Int::from(0);
                    let chosen = if truthy { Some(body) } else { else_body.as_ref() };

                    if let Some(chosen) = chosen {
                        fields.extend(self.unroll_struct_body(chosen)?);
                    }
                }
            }
        }

        Ok(fields)
    }

    // Resolves a struct field's (possibly spliced) name to a literal
    // string against the const-expression evaluator (`self.generic_scope`
    // + top-level consts) — the counterpart, for names, of
    // `AliasResolver::eval_const_expr` for values. Used only here; a
    // macro-body-generated `pub const`'s name goes through
    // `values::AliasResolver::resolve_spliced_name` instead, which
    // evaluates against a live macro invocation's `Value` scope rather
    // than `generic_scope`.
    fn resolve_spliced_name_as_const(&self, parts: &[NamePart]) -> Result<String, ResolveError> {
        if let Some(literal) = literal_name(parts) {
            return Ok(literal);
        }

        let mut out = String::new();

        for part in parts {
            match part {
                NamePart::Literal(text) => out.push_str(text),
                NamePart::Splice(expr) => out.push_str(&self.eval_const_expr(expr)?.to_string()),
            }
        }

        Ok(out)
    }
}

// A bare reference to a generic const parameter that's in scope but not
// yet bound to a concrete value — mirrors `AliasResolver::resolve_generic_args`'s
// identical check for a field's *type* (`bits<N>` staying symbolic rather
// than folded), applied here to a `@for`/`@if`'s bound/condition instead.
fn references_unbound_generic(expr: &Expr, generic_scope: &HashMap<String, GenericBinding>) -> bool {
    match expr {
        Expr::Identifier { name, .. } => {
            matches!(generic_scope.get(name), Some(GenericBinding::Const(None)))
        }

        Expr::Range { start, end, .. } => {
            references_unbound_generic(start, generic_scope) || references_unbound_generic(end, generic_scope)
        }

        _ => false,
    }
}

pub(super) fn param_name(param: &GenericParameter) -> &str {
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
        ResolvedType::Enum { symbol, .. } => symbols.get(*symbol).name.clone(),
        ResolvedType::TypeParameter { name } => name.clone(),

        // The alias's own name reads better in a diagnostic than its
        // underlying struct's — "expected `uint8_t`, got `int`" over
        // "expected `bits`, got `int`".
        ResolvedType::Alias { symbol, .. } => symbols.get(*symbol).name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use crate::ast::Program;
    use crate::eval::Int;
    use crate::lexer;
    use crate::parser;
    use crate::resolver::{collect_symbols, AliasResolver, ResolveError};

    use super::{BuiltinType, ResolvedGenericArg, ResolvedType};

    fn parse_fixture(name: &str) -> Program {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/emit")
            .join(name);

        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()));

        let tokens = lexer::lex(&source).expect("fixture should lex");
        parser::parse(tokens).expect("fixture should parse")
    }

    #[test]
    fn for_generated_fields_resolve_under_a_concrete_instantiation() {
        let program = parse_fixture("for_generated_struct_fields.basm");
        let symbols = collect_symbols(&program).unwrap();
        let array_id = symbols.lookup("Array").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let fields = resolver
            .instantiate_struct_fields(
                array_id,
                &[
                    ResolvedGenericArg::Type(Box::new(ResolvedType::Builtin(BuiltinType::Int))),
                    ResolvedGenericArg::Const(Int::from(3)),
                ],
            )
            .unwrap();

        assert_eq!(fields, vec![ResolvedType::Builtin(BuiltinType::Int); 3]);
    }

    #[test]
    fn for_generated_field_names_are_reachable_via_field_type() {
        let program = parse_fixture("for_generated_struct_fields.basm");
        let symbols = collect_symbols(&program).unwrap();
        let array_id = symbols.lookup("Array").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let ty = ResolvedType::Struct {
            symbol: array_id,
            args: vec![
                ResolvedGenericArg::Type(Box::new(ResolvedType::Builtin(BuiltinType::Int))),
                ResolvedGenericArg::Const(Int::from(3)),
            ],
        };

        let span = program.span;

        assert_eq!(
            resolver.field_type(&ty, "__el0", span).unwrap(),
            ResolvedType::Builtin(BuiltinType::Int)
        );
        assert_eq!(
            resolver.field_type(&ty, "__el2", span).unwrap(),
            ResolvedType::Builtin(BuiltinType::Int)
        );
        assert!(resolver.field_type(&ty, "__el3", span).is_err());
    }

    #[test]
    fn abstract_resolution_skips_an_unrollable_for_without_erroring() {
        // `n` isn't bound to a concrete value in this abstract,
        // every-struct-in-the-program pass — `resolve_all_structs` should
        // still succeed, just without any `@for`-generated fields to show
        // for it (see `AliasResolver::unroll_struct_body`'s doc).
        let program = parse_fixture("for_generated_struct_fields.basm");
        let symbols = collect_symbols(&program).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        assert!(resolver.resolve_all_structs().is_ok());
    }

    #[test]
    fn struct_body_for_over_a_struct_valued_pub_field_errors() {
        // Struct-body `@for` stays in the const-generic world: its source
        // is evaluated against a throwaway scope built only from bound
        // `Const` generic args, and every visited element must be an
        // `Int` — a struct-valued element (like `Wrapper`'s `inner` field
        // here) doesn't fit that model. See `AliasResolver::unroll_struct_body`.
        let program = parse_fixture("struct_body_for_over_struct_valued_field.basm");
        let symbols = collect_symbols(&program).unwrap();
        let uses_for_id = symbols.lookup("UsesFor").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let result = resolver.instantiate_struct_fields(uses_for_id, &[ResolvedGenericArg::Const(Int::from(1))]);

        assert!(matches!(result, Err(ResolveError::ExpectedIntValue { .. })));
    }

    #[test]
    fn if_true_includes_the_field_and_false_omits_it() {
        let program = parse_fixture("if_generated_struct_field.basm");
        let symbols = collect_symbols(&program).unwrap();
        let conditional_id = symbols.lookup("Conditional").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let present = resolver
            .instantiate_struct_fields(conditional_id, &[ResolvedGenericArg::Const(Int::from(1))])
            .unwrap();
        assert_eq!(present, vec![ResolvedType::Builtin(BuiltinType::Int)]);

        let absent = resolver
            .instantiate_struct_fields(conditional_id, &[ResolvedGenericArg::Const(Int::from(0))])
            .unwrap();
        assert_eq!(absent, vec![]);
    }
}
