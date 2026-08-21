//! A flat, whole-program table of top-level declarations (structs, type
//! aliases, consts), keyed by name. By the time [`SymbolTable`] is built
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
    TypeAlias,
    Const,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub name: String,
    pub span: Span,
}

/// ```
/// use bitterasm::resolver::{SymbolKind, SymbolTable};
/// use bitterasm::token::Span;
///
/// let mut table = SymbolTable::new();
/// let span = Span::new(0, 0);
///
/// let id = table.insert("Reg".to_string(), SymbolKind::Struct, span).unwrap();
/// assert_eq!(table.lookup("Reg"), Some(id));
///
/// // Re-inserting the same name fails rather than shadowing it.
/// assert!(table.insert("Reg".to_string(), SymbolKind::Struct, span).is_err());
/// ```
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
    by_name: HashMap<String, SymbolId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(
        &mut self, name: String, kind: SymbolKind, span: Span
    ) -> Result<SymbolId, DuplicateSymbol> {
        if let Some(&existing) = self.by_name.get(&name) {
            return Err(DuplicateSymbol {
                name,
                existing,
                span
            });
        }

        let id = SymbolId(self.symbols.len());

        self.symbols.push(Symbol {
            id,
            kind,
            name: name.clone(),
            span,
        });

        self.by_name.insert(name, id);

        Ok(id)
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolId> {
        self.by_name.get(name).copied()
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }
}

#[derive(Debug)]
pub struct DuplicateSymbol {
    pub name: String,
    pub existing: SymbolId,
    pub span: Span,
}