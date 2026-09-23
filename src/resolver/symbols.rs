//! A flat, whole-program table of top-level declarations (structs, type
//! aliases, consts, macros), keyed by name. Macro names may map to an
//! overload set; all other declaration names remain unique. By the time [`SymbolTable`] is built
//! the [`crate::loader`] has already flattened every imported module into
//! one [`crate::ast::Program`], so a single flat table — rather than one
//! scoped per module — is enough.

use std::collections::HashMap;

use crate::token::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Struct,
    Enum,
    TypeAlias,
    Const,
    Macro,
    Label,

    /// A `from file import label_name` deferred cross-unit reference —
    /// see `ast::ExternLabel` and `docs/sections-and-linking/PROGRESS.md`'s
    /// Phase 5. Deliberately distinct from `Label`: an ordinary `Label`
    /// resolves through `AliasResolver::label_positions` (known by the end
    /// of this compilation), while an `ExternLabel` never does — its value
    /// isn't known until a later `bitter build`/`bitter exec` link step.
    ExternLabel,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub name: String,
    pub span: Span,

    /// Which module (an index assigned by `crate::loader`, meaningful only
    /// as "same or different" — not a path) declared this symbol. Backs
    /// non-`pub` struct field visibility: a field access is only allowed
    /// when `AliasResolver::current_module` matches the accessed struct's
    /// own `module` here. See `crate::loader::ModuleOrigins`.
    pub module: usize,
}

/// ```
/// use bitterasm::resolver::{SymbolKind, SymbolTable};
/// use bitterasm::token::Span;
///
/// let mut table = SymbolTable::new();
/// let span = Span::new(0, 0);
///
/// let id = table.insert("Reg".to_string(), SymbolKind::Struct, span, 0).unwrap();
/// assert_eq!(table.lookup("Reg"), Some(id));
///
/// // Re-inserting the same name fails rather than shadowing it.
/// assert!(table.insert("Reg".to_string(), SymbolKind::Struct, span, 0).is_err());
/// ```
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
    by_name: HashMap<String, Vec<SymbolId>>,

    /// `SymbolId`s handed out by this table start at `base` rather than 0
    /// — lets a second table (e.g. `AliasResolver::generated_symbols`)
    /// share the same `SymbolId` space as a first one without either
    /// table's ids colliding. See `AliasResolver::get_symbol`.
    base: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// A table whose ids start at `base` instead of 0 — see the `base`
    /// field's doc.
    pub fn with_base(base: usize) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    pub fn insert(
        &mut self, name: String, kind: SymbolKind, span: Span, module: usize
    ) -> Result<SymbolId, DuplicateSymbol> {
        if let Some(existing) = self.by_name.get(&name) {
            // Macros form overload sets. Every other declaration remains
            // unique by name, and a macro may not overload a non-macro.
            if kind != SymbolKind::Macro
                || existing.iter().any(|id| self.get(*id).kind != SymbolKind::Macro)
            {
                return Err(DuplicateSymbol {
                    name,
                    existing: existing[0],
                    span
                });
            }
        }

        let id = SymbolId(self.base + self.symbols.len());

        self.symbols.push(Symbol {
            id,
            kind,
            name: name.clone(),
            span,
            module,
        });

        self.by_name.entry(name).or_default().push(id);

        Ok(id)
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.by_name.get(name).and_then(|ids| ids.first()).copied()
    }

    /// All declarations registered under `name`. More than one result is
    /// possible only for macro overload sets.
    pub fn lookup_all(&self, name: &str) -> &[SymbolId] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 - self.base]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    pub fn extend_from(&mut self, other: &SymbolTable) {
        for symbol in &other.symbols {
            debug_assert_eq!(
                symbol.id.0,
                self.base + self.symbols.len(),
                "generated symbols must be appended in their original id order",
            );
            self.symbols.push(symbol.clone());
            self.by_name.entry(symbol.name.clone()).or_default().push(symbol.id);
        }
    }
}

#[derive(Debug)]
pub struct DuplicateSymbol {
    pub name: String,
    pub existing: SymbolId,
    pub span: Span,
}
