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

/// How `AliasResolver::resolve_label_value` treats a known top-level label
/// whose position hasn't been recorded yet — i.e. a forward reference.
/// `Tolerant` (position-discovery pass) substitutes a placeholder so
/// expansion can keep running long enough to discover *every* label's
/// position, including ones after the current point; `Strict` (the real
/// pass) trusts that discovery already ran to completion, so an
/// unresolved-but-known label at that point is a resolver bug, not a
/// legitimate forward reference. See `main::resolve_and_expand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelMode {
    Tolerant,
    Strict,
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

    // Shared by statement invocations and expression-position macro calls.
    // Repeated symbols are allowed because terminating recursion is useful
    // for compile-time algorithms; `macro_body` enforces a finite depth.
    pub(super) macro_call_stack: Vec<SymbolId>,
    pub(super) pending_tail_call: Option<Vec<super::Value>>,

    pub(super) generic_scope: HashMap<String, GenericBinding>,

    // How many values have been `@emit`'d so far, in whole-program
    // emission order — read by `@here`, advanced once per `@emit` (see
    // `macro_body::walk_macro_body`). Unlike `stack` above, this must
    // *not* reset per top-level invocation: it tracks a position in the
    // same flattened stream `main::resolve_and_expand`'s own `emitted`
    // accumulates across the whole program, so it only resets when a
    // fresh `AliasResolver` is constructed for a new pass.
    pub(super) values_emitted: Int,
    pub(super) label_positions: HashMap<SymbolId, Int>,
    pub(super) label_mode: LabelMode,

    // Declarations discovered mid-resolution — a macro's `generated`
    // output, or a `0..N` range's synthesized struct — rather than present
    // in `program` from the start. `generated_symbols` shares the same
    // `SymbolId` space as `symbols` (via `SymbolTable::with_base`, offset
    // past every id `symbols` could ever hand out) so a `SymbolId` alone
    // tells `get_symbol`/`lookup_symbol` which table to check; `generated`
    // holds the actual declaration bodies, scanned by
    // `find_struct_declaration`/`find_macro_declaration`/etc. as a fallback
    // after `program.statements`. See `super::generated`.
    pub(super) generated_symbols: SymbolTable,
    pub(super) generated: Vec<Statement>,
}

impl<'a> AliasResolver<'a> {
    /// `consts` is every top-level const already evaluated to an `Int`
    /// (see [`super::ConstEvaluator`]) — needed so a generic const argument
    /// that references one, e.g. `bits<SOME_WIDTH>`, can fold to a
    /// concrete value the same way a literal or arithmetic expression does.
    ///
    /// `label_mode`/`known_label_positions` select which of the two label
    /// -resolution passes this instance runs: `Tolerant` with an empty map
    /// for position-discovery, `Strict` with that pass's completed map for
    /// the real expansion. See `main::resolve_and_expand`.
    pub fn new(
        program: &'a Program,
        symbols: &'a SymbolTable,
        consts: &'a HashMap<String, Int>,
        label_mode: LabelMode,
        known_label_positions: HashMap<SymbolId, Int>,
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
            macro_call_stack: Vec::new(),
            pending_tail_call: None,
            generic_scope: HashMap::new(),
            values_emitted: Int::from(0),
            label_positions: known_label_positions,
            label_mode,
            generated_symbols: SymbolTable::with_base(symbols.len()),
            generated: Vec::new(),
        }
    }

    /// Convenience for callers that don't care about label/`@here`
    /// resolution across a whole-program two-pass expansion (unit tests
    /// resolving a single fixture's types/macro body in isolation) —
    /// equivalent to [`AliasResolver::new`] with `LabelMode::Strict` and no
    /// pre-recorded label positions. A real `bitterasm compile`/`expand`
    /// run should go through the two-pass driver in `main.rs` instead.
    pub fn new_single_pass(
        program: &'a Program,
        symbols: &'a SymbolTable,
        consts: &'a HashMap<String, Int>,
    ) -> Self {
        Self::new(program, symbols, consts, LabelMode::Strict, HashMap::new())
    }

    /// Records `id`'s position as "however many values have been emitted
    /// so far" — called when a top-level `Statement::Label` is walked (see
    /// `main::resolve_and_expand`; nested, in-macro-body labels never call
    /// this, they stay uninvolved in label resolution entirely).
    pub fn record_label_position(&mut self, id: SymbolId) {
        self.label_positions.insert(id, self.values_emitted.clone());
    }

    /// Consumes a position-discovery-pass resolver, handing back its
    /// completed label-position map to seed the real pass's resolver.
    pub fn into_label_positions(self) -> HashMap<SymbolId, Int> {
        self.label_positions
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

            // Not pre-seeded by `AliasResolver::new` — a type alias
            // generated mid-resolution starts `Unvisited` the first time
            // it's actually referenced, same as if it had been seeded from
            // the start. A symbol that isn't a type alias at all (wrong
            // `SymbolKind`) still isn't reachable through here — this arm
            // only ever sees ids `resolve_named_type` already confirmed are
            // `SymbolKind::TypeAlias`.
            None if self.get_symbol(id).kind == SymbolKind::TypeAlias => {
                self.states.insert(id, AliasState::Unvisited);
            }

            None => {
                let symbol = self.get_symbol(id);

                return Err(ResolveError::ExpectedType {
                    name: symbol.name.clone(),
                    span: symbol.span
                })
            }
        }

        self.states.insert(id, AliasState::Visiting);
        self.stack.push(id);

        let declaration = self.find_alias_declaration(id)?.clone();

        let result = self.resolve_type_expr(&declaration.ty).and_then(|underlying| {
            self.wrap_if_invariant(id, &declaration, underlying)
        });

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

    // Wraps `underlying` (what `id`'s target resolved to) in
    // `ResolvedType::Alias` iff `id`'s own declaration carries at least one
    // `invariant` facet — a plain alias with none of its own stays
    // transparent, returning `underlying` untouched, even when `underlying`
    // is itself already `Alias` (a deeper layer's invariant "holds all the
    // way down" without this layer needing to add anything — see that
    // variant's doc). See `crate::resolver::facets::alias_invariant_binder`
    // for how the binder name is chosen.
    fn wrap_if_invariant(
        &self,
        id: SymbolId,
        declaration: &crate::ast::TypeAliasDeclaration,
        underlying: ResolvedType,
    ) -> Result<ResolvedType, ResolveError> {
        let invariants = crate::facets::extract_invariants(&declaration.facets);

        let has_conversions = declaration
            .facets
            .iter()
            .any(|facet| matches!(facet.name.as_str(), "to" | "from"));

        if invariants.is_empty() && !has_conversions {
            return Ok(underlying);
        }

        let binder = super::facets::alias_invariant_binder(
            &declaration.name,
            &declaration.generic_params,
            &declaration.facets,
            self.symbols,
            declaration.span,
        )?;

        Ok(ResolvedType::Alias {
            symbol: id,
            binder,
            invariants,
            underlying: Box::new(underlying),
        })
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

        let Some(id) = self.lookup_symbol(name) else {
            return Err(ResolveError::UnknownType {
                name: name.clone(),
                span,
            });
        };

        let symbol = self.get_symbol(id);

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

            SymbolKind::Label => {
                Err(ResolveError::ExpectedType {
                    name: name.clone(),
                    span,
                })
            }

            // Not wired into `Value`/`ResolvedType` yet — see
            // `ast::EnumDeclaration`'s doc.
            SymbolKind::Enum => {
                Ok(ResolvedType::Enum { symbol: id, args: Vec::new() })
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
                    let name = self.get_symbol(symbol).name.clone();

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

            ResolvedType::Enum { symbol, .. } => {
                let expected = self.find_enum_declaration(symbol)?.generic_params.len();
                if args.len() != expected {
                    return Err(ResolveError::InvalidGenericArity {
                        name: self.get_symbol(symbol).name.clone(),
                        expected,
                        actual: args.len(),
                        span,
                    });
                }
                let resolved_args = args
                    .iter()
                    .map(|arg| self.resolve_generic_args(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedType::Enum { symbol, args: resolved_args })
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

            // Applying generic args to an already-resolved alias isn't
            // supported (a generic alias's own target has no mechanism to
            // substitute its params into today, nominal or not) — same
            // rejection as `Builtin`/`TypeParameter` just above, not a
            // regression this variant introduces.
            ResolvedType::Alias { symbol, .. } => {
                Err(ResolveError::ExpectedType {
                    name: self.get_symbol(symbol).name.clone(),
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

    pub(super) fn eval_const_expr(&self, expr: &Expr) -> Result<Int, ResolveError> {
        // Payload-free enum variants are valid const-generic arguments. They
        // are stored by stable declaration-order discriminant; the declared
        // generic parameter type reconstructs the typed enum value whenever
        // the argument enters an expression scope.
        if let Expr::Member { object, member, span } = expr {
            if let Expr::Identifier { name, .. } = object.as_ref() {
                if let Some(id) = self.lookup_symbol(name) {
                    if self.get_symbol(id).kind == SymbolKind::Enum {
                        let declaration = self.find_enum_declaration(id)?;
                        if let Some((index, variant)) = declaration
                            .variants
                            .iter()
                            .enumerate()
                            .find(|(_, variant)| variant.name == *member)
                        {
                            if variant.payload.is_none() {
                                return Ok(Int::from(index));
                            }
                        }
                        return Err(ResolveError::UnknownField {
                            type_name: name.clone(),
                            field: member.clone(),
                            span: *span,
                        });
                    }
                }
            }
        }

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
                let known = self.lookup_symbol(&name).is_some()
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

    pub(super) fn find_alias_declaration(
        &self,
        id: SymbolId,
    ) -> Result<&crate::ast::TypeAliasDeclaration, ResolveError> {
        let symbol = self.get_symbol(id);

        for statement in self.program.statements.iter().chain(&self.generated) {
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
        let symbol = self.get_symbol(id);

        for statement in self.program.statements.iter().chain(&self.generated) {
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

    pub(super) fn find_enum_declaration(
        &self,
        id: SymbolId,
    ) -> Result<&crate::ast::EnumDeclaration, ResolveError> {
        let symbol = self.get_symbol(id);
        for statement in self.program.statements.iter().chain(&self.generated) {
            if let Statement::Enum(declaration) = statement {
                if declaration.name == symbol.name {
                    return Ok(declaration);
                }
            }
        }
        Err(ResolveError::Internal {
            message: format!(
                "symbol table contains enum `{}` but no matching AST declaration exists",
                symbol.name,
            ),
            span: symbol.span,
        })
    }

    pub(super) fn find_macro_declaration(
        &self,
        id: SymbolId,
    ) -> Result<&crate::ast::MacroDeclaration, ResolveError> {
        let symbol = self.get_symbol(id);
        let (table, statements): (&SymbolTable, &[Statement]) = if id.0 < self.symbols.len() {
            (self.symbols, &self.program.statements)
        } else {
            (&self.generated_symbols, &self.generated)
        };
        // Spans are local to their original source file and can coincide
        // after imports are flattened. Match the overload's ordinal in its
        // symbol table to the same ordinal in the AST instead.
        let ordinal = table
            .lookup_all(&symbol.name)
            .iter()
            .position(|candidate| *candidate == id)
            .expect("a macro symbol must occur in its table's name index");
        if let Some(declaration) = statements
            .iter()
            .filter_map(|statement| match statement {
                Statement::Macro(declaration) if declaration.name == symbol.name => {
                    Some(declaration)
                }
                _ => None,
            })
            .nth(ordinal)
        {
            return Ok(declaration);
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
            .map(|id| self.get_symbol(*id).name.clone())
            .collect();

        // Close the loop:
        //
        //     A -> B -> C -> A
        cycle.push(
            self.get_symbol(repeated).name.clone()
        );

        ResolveError::CyclicTypeAlias {
            cycle,
            span: self.get_symbol(repeated).span,
        }
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

    use super::{ResolvedGenericArg, ResolvedType};

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
    fn invariant_bearing_alias_resolves_nominal() {
        let program = parse_fixture("nominal_alias_invariant.basm");
        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let ubyte_id = symbols.lookup("UByte").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let resolved = resolver.resolve_alias(ubyte_id).unwrap();

        let ResolvedType::Alias { symbol, binder, invariants, underlying } = resolved else {
            panic!("expected a nominal Alias, got {resolved:?}");
        };

        assert_eq!(symbol, ubyte_id);
        assert_eq!(binder, Some("value".to_string()));
        assert_eq!(invariants.len(), 1);
        assert_eq!(
            *underlying,
            ResolvedType::Struct { symbol: bits_id, args: vec![ResolvedGenericArg::Const(Int::from(8))] }
        );
    }

    #[test]
    fn plain_alias_stays_transparent() {
        let program = parse_fixture("nominal_alias_invariant.basm");
        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let byte_id = symbols.lookup("Byte").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let resolved = resolver.resolve_alias(byte_id).unwrap();

        assert_eq!(
            resolved,
            ResolvedType::Struct { symbol: bits_id, args: vec![ResolvedGenericArg::Const(Int::from(8))] }
        );
    }

    #[test]
    fn nominal_alias_is_not_equal_to_its_underlying_type() {
        let program = parse_fixture("nominal_alias_invariant.basm");
        let symbols = collect_symbols(&program).unwrap();
        let bits_id = symbols.lookup("Bits").unwrap();
        let ubyte_id = symbols.lookup("UByte").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let nominal = resolver.resolve_alias(ubyte_id).unwrap();
        let underlying = ResolvedType::Struct { symbol: bits_id, args: vec![ResolvedGenericArg::Const(Int::from(8))] };

        // A plain `Bits<8>` value doesn't satisfy `UByte` just because it's
        // shaped the same — that's the whole point of nominal typing: only
        // going through the checked gate (`as`/a checked `const`, neither
        // built yet) produces a value that type-checks as `UByte`.
        assert_ne!(nominal, underlying);
    }

    #[test]
    fn ambiguous_invariant_binder_is_rejected() {
        let program = parse_fixture("ambiguous_alias_invariant.basm");
        let symbols = collect_symbols(&program).unwrap();
        let bad_id = symbols.lookup("Bad").unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        assert!(matches!(
            resolver.resolve_alias(bad_id),
            Err(ResolveError::AmbiguousInvariantBinder { .. })
        ));
    }
}
