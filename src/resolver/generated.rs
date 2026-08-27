//! Declarations discovered mid-resolution rather than present in the
//! program from the start: a macro's `generated` output (`pub struct`/
//! `pub const`/`pub type`/`pub macro`/label bubbled up from a nested
//! expansion), or a `start..end` range's synthesized private struct (see
//! [`AliasResolver::eval_range_value`]). Both go through
//! [`AliasResolver::register_generated`], which is what lets a symbol
//! discovered *after* [`super::collect_symbols`] already ran still be
//! looked up correctly by every later reference to it, without giving
//! `AliasResolver` a mutable/growable `Program` or doing lifetime surgery —
//! see [`AliasResolver::get_symbol`]/[`AliasResolver::lookup_symbol`].

use std::collections::HashMap;

use crate::ast::{Expr, Statement, StructDeclaration};
use crate::eval::Int;
use crate::token::Span;
use crate::types::StructBodyItem;

use super::aliases::AliasResolver;
use super::macro_body::MAX_FOR_ITERATIONS;
use super::symbols::{Symbol, SymbolId, SymbolKind};
use super::values::Value;
use super::ResolveError;

impl<'a> AliasResolver<'a> {
    /// `self.symbols` first, `self.generated_symbols` as a fallback — the
    /// two tables share one `SymbolId` space (see `SymbolTable::with_base`)
    /// so which one a given id belongs to is determined purely by its
    /// numeric value, never by which table happened to hand it out.
    pub(super) fn get_symbol(&self, id: SymbolId) -> &Symbol {
        if id.0 < self.symbols.len() {
            self.symbols.get(id)
        } else {
            self.generated_symbols.get(id)
        }
    }

    pub(super) fn lookup_symbol(&self, name: &str) -> Option<SymbolId> {
        self.symbols.lookup(name).or_else(|| self.generated_symbols.lookup(name))
    }

    pub(super) fn lookup_symbols(&self, name: &str) -> Vec<SymbolId> {
        self.symbols
            .lookup_all(name)
            .iter()
            .chain(self.generated_symbols.lookup_all(name))
            .copied()
            .collect()
    }

    /// Registers a freshly-discovered declaration (a macro's generated
    /// output, or a synthesized range struct) so later lookups
    /// (`get_symbol`/`lookup_symbol`, and `find_struct_declaration`'s/
    /// `find_macro_declaration`'s/etc. fallback scan over `self.generated`)
    /// can find it. Reuses `SymbolTable::insert`'s duplicate-name rules:
    /// ordinary declarations still collide, while same-named generated
    /// macros join an overload set.
    pub(super) fn register_generated(&mut self, statement: &Statement) -> Result<(), ResolveError> {
        // Every named case below is only ever reached with an
        // already-literal name (the caller resolves any splice first — see
        // `macro_body::walk_macro_body`'s per-kind splicing before it calls
        // this).
        let (name, kind, span) = match statement {
            Statement::Struct(decl) => {
                let name = crate::ast::literal_name(&decl.name).ok_or_else(|| ResolveError::Internal {
                    message: "register_generated's Struct case requires an already-literal name"
                        .to_string(),
                    span: decl.span,
                })?;

                (name, SymbolKind::Struct, decl.span)
            }

            Statement::TypeAlias(decl) => {
                let name = crate::ast::literal_name(&decl.name).ok_or_else(|| ResolveError::Internal {
                    message: "register_generated's TypeAlias case requires an already-literal name"
                        .to_string(),
                    span: decl.span,
                })?;

                (name, SymbolKind::TypeAlias, decl.span)
            }

            Statement::Macro(decl) => {
                let name = crate::ast::literal_name(&decl.name).ok_or_else(|| ResolveError::Internal {
                    message: "register_generated's Macro case requires an already-literal name"
                        .to_string(),
                    span: decl.span,
                })?;

                (name, SymbolKind::Macro, decl.span)
            }

            Statement::Label(label) => (label.name.clone(), SymbolKind::Label, label.span),

            // Generated enums use the same symbol-registration path as
            // source-level enums.
            Statement::Enum(decl) => {
                let name = crate::ast::literal_name(&decl.name).ok_or_else(|| ResolveError::Internal {
                    message: "register_generated's Enum case requires an already-literal name"
                        .to_string(),
                    span: decl.span,
                })?;

                (name, SymbolKind::Enum, decl.span)
            }

            Statement::Const(decl) => {
                let name = crate::ast::literal_name(&decl.name).ok_or_else(|| ResolveError::Internal {
                    message: "register_generated's Const case requires an already-literal name"
                        .to_string(),
                    span: decl.span,
                })?;

                (name, SymbolKind::Const, decl.span)
            }

            Statement::Import(_) | Statement::Invocation(_) | Statement::Meta(_) => {
                return Err(ResolveError::Internal {
                    message: "register_generated only accepts Struct/TypeAlias/Const/Macro/Label"
                        .to_string(),
                    span: statement_span(statement),
                });
            }
        };

        self.generated_symbols
            .insert(name.clone(), kind, span)
            .map_err(|duplicate| ResolveError::DuplicateSymbol {
                name: duplicate.name,
                span: duplicate.span,
            })?;

        self.generated.push(statement.clone());

        Ok(())
    }

    /// `start..end` sugar: synthesizes a private struct (an unspellable
    /// name, same idiom as `loader::build_rename_map`'s module-private
    /// renaming — just disambiguated per-synthesis instead of per-module)
    /// with one `pub` `int` field per element, registers it via
    /// `register_generated`, and returns a `Value::Struct` over it with
    /// those fields' actual `Value::Int`s already filled in. This is a
    /// literal materialization, not a lazily-computed stand-in — see
    /// `ast::Expr::Range`'s doc and the project's "abstraction does not
    /// imply optimization" stance: a struct with a million fields really is
    /// built with a million fields, bounded only by `MAX_FOR_ITERATIONS`
    /// like any other `@for`.
    pub(super) fn eval_range_value(
        &mut self,
        start: &Expr,
        end: &Expr,
        span: Span,
        scope: &HashMap<String, Value>,
    ) -> Result<Value, ResolveError> {
        let start_value = self.eval_int(start, scope)?;
        let end_value = self.eval_int(end, scope)?;

        let mut struct_fields = Vec::new();
        let mut values = Vec::new();

        let mut i = start_value;
        let mut iterations: u64 = 0;

        while i < end_value {
            iterations += 1;

            if iterations > MAX_FOR_ITERATIONS {
                return Err(ResolveError::ForLoopTooLarge { span });
            }

            let field_name = format!("__el{i}");

            struct_fields.push(StructBodyItem::Field(crate::types::StructField {
                name: vec![crate::ast::NamePart::Literal(field_name.clone())],
                ty: crate::types::TypeExpr::Named {
                    path: vec!["int".to_string()],
                    span,
                },
                is_pub: true,
                default: None,
                span,
            }));

            values.push((field_name, Value::Int(i.clone())));

            i += Int::from(1);
        }

        let name = format!("__range#{}", self.generated_symbols.len());

        let decl = Statement::Struct(StructDeclaration {
            name: vec![crate::ast::NamePart::Literal(name.clone())],
            is_pub: false,
            generic_params: Vec::new(),
            facets: Vec::new(),
            fields: struct_fields,
            span,
        });

        self.register_generated(&decl)?;

        let symbol = self
            .lookup_symbol(&name)
            .expect("register_generated just inserted this name");

        Ok(Value::Struct {
            symbol,
            args: Vec::new(),
            fields: values,
            nominal: None,
        })
    }

    /// The shared "what does `@for i in X` iterate over" evaluation: `X`
    /// must resolve to a `Value::Struct`, and only its **pub** fields are
    /// visited, in declaration order — never all fields, so a struct can
    /// keep private bookkeeping fields (e.g. whatever backs an `invariant`)
    /// out of any `@for` that iterates it. Used uniformly by every `@for`
    /// call site that has a `Value` scope to evaluate against (macro-body
    /// statements, a brace-literal construction's own `@for`, and —
    /// against a throwaway scope built from const-generic bindings — a
    /// struct declaration's own body; top-level `@for` is the deliberate
    /// exception, see `resolver::toplevel`'s module doc).
    pub(super) fn eval_for_source(
        &mut self,
        source: &Expr,
        scope: &HashMap<String, Value>,
    ) -> Result<Vec<(String, Value)>, ResolveError> {
        // A literal `start..end` source never needs `eval_range_value`'s
        // materialized struct — nothing here treats the range as a value in
        // its own right, only as something to iterate — so this fast path
        // yields the same `(__elN, Int)` pairs directly. This is a resolver
        // implementation detail, not a semantic rewrite: `0..N` captured as
        // a value (stored, passed to a macro, etc.) still goes through
        // `eval_value` → `eval_range_value` and gets the real struct: "abstraction
        // does not imply optimization" governs what a macro's expansion
        // emits, not how the resolver internally computes compiler-owned
        // sugar like this one.
        if let Expr::Range { start, end, span } = source {
            return self.eval_range_for_source(start, end, *span, scope);
        }

        let value = self.eval_value(source, scope)?;

        let Value::Struct { symbol, args, fields, .. } = value else {
            return Err(ResolveError::ExpectedStructValue { span: source.span() });
        };

        let pub_flags = self.struct_field_pub_flags(symbol, &args)?;

        Ok(fields
            .into_iter()
            .zip(pub_flags)
            .filter(|(_, is_pub)| *is_pub)
            .map(|(field, _)| field)
            .collect())
    }

    /// `eval_range_value`'s loop, minus the struct materialization —
    /// same bound (`MAX_FOR_ITERATIONS`) and field names (`__el{i}`), kept
    /// in sync deliberately so the two are observably identical to a caller
    /// that only ever iterates the result.
    fn eval_range_for_source(
        &mut self,
        start: &Expr,
        end: &Expr,
        span: Span,
        scope: &HashMap<String, Value>,
    ) -> Result<Vec<(String, Value)>, ResolveError> {
        let start_value = self.eval_int(start, scope)?;
        let end_value = self.eval_int(end, scope)?;

        let mut values = Vec::new();
        let mut i = start_value;
        let mut iterations: u64 = 0;

        while i < end_value {
            iterations += 1;

            if iterations > MAX_FOR_ITERATIONS {
                return Err(ResolveError::ForLoopTooLarge { span });
            }

            values.push((format!("__el{i}"), Value::Int(i.clone())));
            i += Int::from(1);
        }

        Ok(values)
    }
}

fn statement_span(statement: &Statement) -> Span {
    match statement {
        Statement::Import(s) => s.span,
        Statement::Struct(s) => s.span,
        Statement::Enum(s) => s.span,
        Statement::TypeAlias(s) => s.span,
        Statement::Const(s) => s.span,
        Statement::Label(s) => s.span,
        Statement::Invocation(s) => s.span,
        Statement::Macro(s) => s.span,
        Statement::Meta(s) => s.span,
    }
}
