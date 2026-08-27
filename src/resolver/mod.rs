//! Resolves the flattened [`Program`] produced by [`crate::loader`] against
//! itself: [`collect_symbols`] builds a whole-program [`SymbolTable`] of
//! top-level declarations, and [`AliasResolver`] resolves [`crate::types::TypeExpr`]
//! trees against that table into [`ResolvedType`]s, instantiating generic
//! struct fields along the way. Import resolution has already happened by
//! this point, so nothing here needs to know about modules or files.

mod aliases;
mod consts;
mod facets;
mod generated;
mod macro_body;
mod metas;
mod structs;
mod symbols;
mod toplevel;
mod types;
mod values;

pub use aliases::{AliasResolver, LabelMode};
pub use consts::ConstEvaluator;
pub use facets::validate as validate_facets;
pub use macro_body::MacroExpansion;
pub use symbols::*;
pub use toplevel::unroll_top_level;
pub use types::*;
pub use values::Value;

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

            Statement::Enum(decl) => {
                table.insert(
                    decl.name.clone(),
                    SymbolKind::Enum,
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
                // A top-level const's name can't contain an unevaluated
                // `` `expr` `` splice — evaluating one needs a live macro
                // invocation's scope (see `resolver::values::AliasResolver::resolve_spliced_name`),
                // which doesn't exist yet at this point (symbol collection
                // runs before any evaluation at all). A splice-generated
                // name only ever makes sense on a `pub const` produced
                // from inside a macro body — see `resolver::macro_body`.
                let Some(name) = crate::ast::literal_name(&decl.name) else {
                    return Err(ResolveError::ComputedNameNotAllowed { span: decl.span });
                };

                table.insert(name, SymbolKind::Const, decl.span)
            }

            Statement::Macro(decl) => {
                table.insert(
                    decl.name.clone(),
                    SymbolKind::Macro,
                    decl.span,
                )
            }

            // Only top-level labels are registered — `collect_symbols`
            // never descends into a `MacroDeclaration`'s body, so a label
            // nested inside a macro body (captured verbatim into
            // `MacroExpansion::generated`, still unresolved) is untouched
            // by this, exactly as before this variant existed.
            Statement::Label(label) => {
                table.insert(
                    label.name.clone(),
                    SymbolKind::Label,
                    label.span,
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

    // `@emit`/`@return` value evaluation (`values`/`macro_body`) errors below.

    InvalidArgumentCount {
        name: String,
        expected: usize,
        actual: usize,
        span: Span,
    },

    // Like `ExpectedType`, but for a value-construction callee (`Expr::Call`)
    // that names something other than a struct — an emitted value's shape
    // always bottoms out in a struct or an Int, nothing else is callable.
    ExpectedStructCallee {
        name: String,
        span: Span,
    },

    // A `Value` was needed as an `Int` (inside arithmetic) but was a struct
    // instead.
    ExpectedIntValue {
        span: Span,
    },

    // A `Value` was needed as a struct (for field access) but was an `Int`
    // instead.
    ExpectedStructValue {
        span: Span,
    },

    // Like `ExpectedConstantExpression`, but for `@emit`/`@return`'s more
    // general notion of "value" (which also allows struct construction and
    // field access) rather than a pure `Int` expression.
    ExpectedValueExpression {
        span: Span,
    },

    // A macro body statement other than `@emit`/`@return` — expanding it
    // needs macro-to-invocation binding, which doesn't exist yet.
    UnsupportedMacroStatement {
        kind: String,
        span: Span,
    },

    // A splice (`` `expr` ``) inside a declaration a macro body is
    // generating evaluated to a struct value — reifying that back into
    // source-shaped `Expr` would need to spell out the struct's resolved
    // generic args (`Reg<64>`, not just `Reg`), which `Expr::Call` has
    // nowhere to put. Only `Int`s can be spliced into generated
    // declarations today.
    UnsupportedSpliceValue {
        span: Span,
    },

    // A value-construction call (`Expr::Call`) whose callee isn't a bare
    // identifier — e.g. a dotted path or another call — which nothing
    // real-world uses today.
    UnsupportedCallExpression {
        span: Span,
    },

    // An `Invocation`'s name doesn't resolve to any known symbol at all.
    UnknownMacro {
        name: String,
        span: Span,
    },

    // Like `UnknownMacro`, but the name resolves to a real symbol of the
    // wrong kind (a struct, const, or type alias) — invocations only ever
    // call macros.
    ExpectedMacro {
        name: String,
        span: Span,
    },

    // Direct or mutual macro self-invocation (`A -> B -> A`), mirroring
    // `CyclicTypeAlias`/`CyclicConstant`'s shape.
    CyclicMacroExpansion {
        cycle: Vec<String>,
        span: Span,
    },

    // `@assert`'s condition (see `metas::assert`) evaluated to `0` —
    // falsy, under the language's `0`/`1` `Int` convention for booleans
    // (`crate::eval`'s module doc). `message` is the assertion's own
    // optional second argument, not a description this error invents.
    AssertionFailed {
        message: Option<String>,
        span: Span,
    },

    // `@assert`'s optional second argument was present but wasn't a
    // string literal.
    InvalidAssertMessage {
        span: Span,
    },

    // A value's type (derived from what it actually is — Int, or which
    // struct) doesn't match the type it was required to have — either a
    // macro invocation's operand against the matching declared parameter,
    // or a struct constructor argument against the matching declared
    // field. `name` is the parameter or field name.
    TypeMismatch {
        name: String,
        expected: String,
        actual: String,
        span: Span,
    },

    // A struct's declared `invariant` (see `crate::facets::invariant`)
    // evaluated to `0` (falsy, same `Int` convention as `AssertionFailed`)
    // against the fields/generic args a construction just produced.
    InvariantViolated {
        type_name: String,
        invariant: String,
        span: Span,
    },

    // `as`'s target, after unwrapping every nominal alias layer, is a
    // struct that isn't already the source value's own shape and doesn't
    // have exactly one field to auto-wrap the source value into — there's
    // no single rule for turning an arbitrary value into a multi-field (or
    // zero-field) struct, so this needs to be constructed directly
    // (`Struct { field: value, ... }`/`Struct(field = value, ...)`) instead.
    CannotCoerce {
        type_name: String,
        span: Span,
    },

    // A `type` alias's `invariant` facet(s) reference more than one
    // distinct identifier that isn't otherwise a known symbol or one of the
    // alias's own generic params — there's no fixed name for "the value
    // being converted" (the alias author picks their own, e.g. `invariant
    // years >= 0`), so more than one candidate means the author almost
    // certainly meant one name and wrote two by mistake.
    AmbiguousInvariantBinder {
        type_name: String,
        names: Vec<String>,
        span: Span,
    },

    Internal {
        message: String,
        span: Span,
    },

    // `@for`'s range spans more than `macro_body::MAX_FOR_ITERATIONS` —
    // guards against a runaway or accidentally-huge loop rather than
    // silently hanging on it.
    ForLoopTooLarge {
        span: Span,
    },

    // A declaration's name contains an unevaluated `` `expr` `` splice in
    // a position that has no live scope to evaluate it against — e.g. a
    // top-level `const` (see `collect_symbols`). Only a `pub const`
    // produced from inside a macro body can have a computed name.
    ComputedNameNotAllowed {
        span: Span,
    },

    // Top-level `@for` runs before symbol/struct resolution exists at all
    // (see `toplevel`'s module doc) — unlike the other three `@for` sites,
    // it can only ever unroll `start..end` range sugar, never a general
    // struct-valued source. A deliberate, confirmed exception to "`@for` is
    // uniform everywhere", not an oversight.
    TopLevelForRequiresRange {
        span: Span,
    },
}
