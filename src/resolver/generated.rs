//! Declarations discovered mid-resolution rather than present in the
//! program from the start: a macro's `generated` output (`pub struct`/
//! `pub const`/`pub type`/`pub macro`/label bubbled up from a nested
//! expansion), a `start..end` range's synthesized private struct (see
//! [`AliasResolver::eval_range_value`]), or a string literal's synthesized
//! private struct (see [`AliasResolver::eval_string_value`]). All go through
//! [`AliasResolver::register_generated`], which is what lets a symbol
//! discovered *after* [`super::collect_symbols`] already ran still be
//! looked up correctly by every later reference to it, without giving
//! `AliasResolver` a mutable/growable `Program` or doing lifetime surgery —
//! see [`AliasResolver::get_symbol`]/[`AliasResolver::lookup_symbol`].


use crate::ast::{Expr, Statement, StructDeclaration};
use crate::eval::Int;
use crate::token::Span;
use crate::types::StructBodyItem;

use super::aliases::AliasResolver;
use super::macro_body::MAX_FOR_ITERATIONS;
use super::symbols::{Symbol, SymbolId, SymbolKind};
use super::values::Value;
use super::ResolveError;
use crate::resolver::Scope;

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
        if is_overload_set(name) {
            return self.lookup_symbols(name).first().copied();
        }
        self.symbols.lookup(name).or_else(|| self.generated_symbols.lookup(name))
    }

    pub(super) fn lookup_symbols(&self, name: &str) -> Vec<SymbolId> {
        if is_overload_set(name) {
            let members = overload_set_members(name).expect("checked just above");
            return members.iter().flat_map(|member| self.lookup_symbols(member)).collect();
        }
        self.symbols
            .lookup_all(name)
            .iter()
            .chain(self.generated_symbols.lookup_all(name))
            .copied()
            .collect()
    }

    /// The internal name the loader rewrote `name` to in the current
    /// module, for a name built during resolution (`` x`i` ``) that the
    /// loader never saw.
    pub(super) fn scoped_name(&self, name: &str) -> Option<String> {
        self.module_scopes.get(self.current_module)?.get(name).cloned()
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

            Statement::Import(_) | Statement::Invocation(_) | Statement::Meta(_)
            | Statement::SyntaxOverride(_) | Statement::Section(_) | Statement::ExternLabel(_) => {
                return Err(ResolveError::Internal {
                    message: "register_generated only accepts Struct/TypeAlias/Const/Macro/Label"
                        .to_string(),
                    span: statement_span(statement),
                });
            }
        };

        self.generated_symbols
            .insert(name.clone(), kind, span, self.current_module)
            .map_err(|duplicate| ResolveError::DuplicateSymbol {
                name: duplicate.name,
                span: duplicate.span,
            })?;

        self.generated.push(statement.clone());

        Ok(())
    }

    /// The type of an anonymous struct with these fields, in order:
    /// `struct { x: int, y: int }`. Every anonymous struct with the same
    /// field names shares one compiler-made generic struct, one type
    /// parameter per field, so two have the same type exactly when their
    /// fields have the same names and types.
    pub(super) fn anon_struct_type(
        &mut self,
        names: Vec<String>,
        types: Vec<super::types::ResolvedType>,
        span: Span,
    ) -> Result<super::types::ResolvedType, ResolveError> {
        let symbol = self.anon_struct_symbol(names, span)?;
        Ok(super::types::ResolvedType::Struct {
            symbol,
            args: types.into_iter().map(|ty| super::types::ResolvedGenericArg::Type(Box::new(ty))).collect(),
        })
    }

    /// The field names of an anonymous struct's type, if `symbol` is one.
    pub(super) fn anon_struct_fields(&self, symbol: SymbolId) -> Option<&[String]> {
        self.anon_struct_names.get(&symbol).map(Vec::as_slice)
    }

    fn anon_struct_symbol(&mut self, names: Vec<String>, span: Span) -> Result<SymbolId, ResolveError> {
        if let Some(symbol) = self.anon_struct_shapes.get(&names) {
            return Ok(*symbol);
        }

        let generic_params = (0..names.len())
            .map(|index| crate::types::GenericParameter::Type { name: format!("T{index}"), span })
            .collect();
        let fields = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                StructBodyItem::Field(crate::types::StructField {
                    name: vec![crate::ast::NamePart::Literal(name.clone())],
                    ty: crate::types::TypeExpr::Named { path: vec![format!("T{index}")], span },
                    is_pub: true,
                    is_skip: false,
                    default: None,
                    doc: None,
                    span,
                })
            })
            .collect();

        let name = format!("__struct${}", self.generated_symbols.len());
        let decl = Statement::Struct(StructDeclaration {
            name: vec![crate::ast::NamePart::Literal(name.clone())],
            is_pub: false,
            generic_params,
            facets: Vec::new(),
            doc: None,
            fields,
            span,
        });

        self.register_generated(&decl)?;

        let symbol = self
            .lookup_symbol(&name)
            .expect("register_generated just inserted this name");
        self.anon_struct_shapes.insert(names.clone(), symbol);
        self.anon_struct_names.insert(symbol, names);
        Ok(symbol)
    }

    /// `{ 42, 43 }` with its fields renamed to `names`, keeping their
    /// values and types, if it's a positional anonymous struct with that
    /// many fields. This is how a parameter or const declared as
    /// `struct { x: int, y: int }` gives `{ 42, 43 }` its field names.
    pub(super) fn name_positional_fields(&mut self, value: &Value, names: &[String], span: Span) -> Result<Option<Value>, ResolveError> {
        let Value::Struct { symbol, fields, .. } = value else { return Ok(None) };
        let Some(current) = self.anon_struct_fields(*symbol) else { return Ok(None) };
        let positional = current.iter().enumerate().all(|(index, name)| *name == format!("__el{index}"));
        if !positional || current.len() != names.len() || current == names {
            return Ok(None);
        }
        let mut types = Vec::with_capacity(fields.len());
        for (_, field) in fields {
            types.push(self.value_type(field)?);
        }
        let renamed: Vec<(String, Value)> = names.iter().cloned().zip(fields.iter().map(|(_, field)| field.clone())).collect();
        let super::types::ResolvedType::Struct { symbol, args } = self.anon_struct_type(names.to_vec(), types, span)? else {
            unreachable!("anon_struct_type always gives a struct type");
        };
        Ok(Some(Value::Struct { symbol, args, fields: renamed, nominal: None }))
    }

    /// `{ x: 42 }` or `{ 42, 43 }`: an anonymous struct whose field types
    /// are its values' types.
    pub(super) fn eval_anon_struct_value(
        &mut self,
        items: &[crate::ast::ConstructItem],
        span: Span,
        scope: &Scope,
    ) -> Result<Value, ResolveError> {
        let fields = self.without_fold_target(|this| this.unroll_construct_items(items, scope))?;

        let mut names = Vec::with_capacity(fields.len());
        let mut types = Vec::with_capacity(fields.len());
        for (name, value) in &fields {
            if names.contains(name) {
                return Err(ResolveError::DuplicateRecordField {
                    subject: "this struct".to_string(),
                    field: name.clone(),
                    span,
                });
            }
            names.push(name.clone());
            types.push(self.value_type(value)?);
        }

        let super::types::ResolvedType::Struct { symbol, args } = self.anon_struct_type(names, types, span)? else {
            unreachable!("anon_struct_type always gives a struct type");
        };
        Ok(Value::Struct { symbol, args, fields, nominal: None })
    }

    /// The private struct for a literal of this shape: one public `int`
    /// field per `(name, skip)`, in order. A shape seen before reuses its
    /// struct, so same-shaped literals share one type and compare by their
    /// values. A new shape is synthesized under an unspellable name,
    /// `{prefix}$N`, the same idiom as `loader::build_rename_map`'s
    /// module-private renaming.
    fn int_struct_for_shape(
        &mut self,
        prefix: &str,
        shape: Vec<(String, bool)>,
        span: Span,
    ) -> Result<SymbolId, ResolveError> {
        if let Some(symbol) = self.generated_shapes.get(&shape) {
            return Ok(*symbol);
        }

        let fields = shape
            .iter()
            .map(|(name, is_skip)| {
                StructBodyItem::Field(crate::types::StructField {
                    name: vec![crate::ast::NamePart::Literal(name.clone())],
                    ty: crate::types::TypeExpr::Named {
                        path: vec!["int".to_string()],
                        span,
                    },
                    is_pub: true,
                    is_skip: *is_skip,
                    default: None,
                    doc: None,
                    span,
                })
            })
            .collect();

        let name = format!("{prefix}${}", self.generated_symbols.len());
        let decl = Statement::Struct(StructDeclaration {
            name: vec![crate::ast::NamePart::Literal(name.clone())],
            is_pub: false,
            generic_params: Vec::new(),
            facets: Vec::new(),
            doc: None,
            fields,
            span,
        });

        self.register_generated(&decl)?;

        let symbol = self
            .lookup_symbol(&name)
            .expect("register_generated just inserted this name");
        self.generated_shapes.insert(shape, symbol);
        Ok(symbol)
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
        inclusive: bool,
        span: Span,
        scope: &Scope,
    ) -> Result<Value, ResolveError> {
        let start_value = self.eval_int(start, scope)?;
        let end_value = self.eval_int(end, scope)?;

        let mut values = Vec::new();

        let mut i = start_value;
        let mut iterations: u64 = 0;

        while if inclusive { i <= end_value } else { i < end_value } {
            iterations += 1;

            if iterations > MAX_FOR_ITERATIONS {
                return Err(ResolveError::ForLoopTooLarge { span });
            }

            values.push((format!("__el{i}"), Value::Int(i.clone())));

            i += Int::from(1);
        }

        let shape = values.iter().map(|(name, _)| (name.clone(), false)).collect();
        let symbol = self.int_struct_for_shape("__range", shape, span)?;

        Ok(Value::Struct {
            symbol,
            args: Vec::new(),
            fields: values,
            nominal: None,
        })
    }

    /// `"text"` sugar: synthesizes a private struct with one public `int`
    /// field per Unicode scalar value, plus a public `skip len` field for
    /// indexed std-library conversions. The iterable surface stays just
    /// the characters, matching `std.array.Array`'s own hidden length
    /// bookkeeping.
    pub(super) fn eval_string_value(
        &mut self,
        value: &str,
        span: Span,
    ) -> Result<Value, ResolveError> {
        let mut values = Vec::new();

        for (index, ch) in value.chars().enumerate() {
            if index as u64 >= MAX_FOR_ITERATIONS {
                return Err(ResolveError::ForLoopTooLarge { span });
            }

            values.push((format!("__el{index}"), Value::Int(Int::from(ch as u32))));
        }

        let len = values.len();
        values.push(("len".to_string(), Value::Int(Int::from(len))));

        let shape = values.iter().map(|(name, _)| (name.clone(), name == "len")).collect();
        let symbol = self.int_struct_for_shape("__string", shape, span)?;

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
        scope: &Scope,
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
        if let Expr::Range { start, end, inclusive, span } = source {
            return self.eval_range_for_source(start, end, *inclusive, *span, scope);
        }

        let value = self.eval_value(source, scope)?;

        let Value::Struct { symbol, args, fields, .. } = value else {
            return Err(ResolveError::ExpectedStructValue { span: source.span() });
        };

        let iterable_flags = self.struct_field_iterable_flags(symbol, &args)?;

        Ok(fields
            .into_iter()
            .zip(iterable_flags)
            .filter(|(_, is_iterable)| *is_iterable)
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
        inclusive: bool,
        span: Span,
        scope: &Scope,
    ) -> Result<Vec<(String, Value)>, ResolveError> {
        let start_value = self.eval_int(start, scope)?;
        let end_value = self.eval_int(end, scope)?;

        let mut values = Vec::new();
        let mut i = start_value;
        let mut iterations: u64 = 0;

        while if inclusive { i <= end_value } else { i < end_value } {
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

pub(super) fn statement_span(statement: &Statement) -> Span {
    match statement {
        Statement::Import(s) => s.span,
        Statement::Struct(s) => s.span,
        Statement::Enum(s) => s.span,
        Statement::TypeAlias(s) => s.span,
        Statement::Const(s) => s.span,
        Statement::Label(s) => s.span,
        Statement::ExternLabel(s) => s.span,
        Statement::Section(s) => s.span,
        Statement::Invocation(s) => s.span,
        Statement::Macro(s) => s.span,
        Statement::Meta(s) => s.span,
        Statement::SyntaxOverride(s) => s.span,
    }
}

/// A macro reference the loader rewrote to `name#a,b,...` means every
/// overload of `name` declared in modules `a`, `b`, ... (whose own internal
/// names are `name#a`, `name#b`, ...). See the loader's "namespaces".
/// Whether `name` names an overload set spread over several modules
/// (`mov#3,5`), without building its members' names.
fn is_overload_set(name: &str) -> bool {
    name.rsplit_once('#').is_some_and(|(_, modules)| modules.contains(','))
}

fn overload_set_members(name: &str) -> Option<Vec<String>> {
    let (base, modules) = name.rsplit_once('#')?;
    if !modules.contains(',') {
        return None;
    }
    Some(modules.split(',').map(|module| format!("{base}#{module}")).collect())
}
