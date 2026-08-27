//! Drives structural validation of facets by calling into each facet's own
//! `check` function ([`crate::facets`]) — this file doesn't decide what's
//! valid for a given facet itself, it just counts occurrences per name and
//! translates the [`crate::facets::Violation`] each `check` call returns
//! into a [`ResolveError`]. Runtime enforcement lives at the relevant
//! construction, conversion, or macro-invocation boundary.

use std::collections::{HashMap, HashSet};

use crate::ast::{Facet, Program, Statement};
use crate::facets::{self, DeclKind, Violation};
use crate::token::Span;
use crate::types::GenericParameter;

use super::consts::referenced_identifiers;
use super::structs::param_name;
use super::symbols::SymbolTable;
use super::ResolveError;

/// The single free identifier `name`'s (a `type` alias's own) `invariant`
/// facet(s) use to refer to "the value being converted" (see
/// `crate::facets::invariant`'s module doc) — `None` if there are no
/// `invariant` facets, or none of them reference anything beyond the
/// alias's own generic params (a constant, pointless-but-harmless
/// invariant). Errors if more than one distinct candidate name appears —
/// almost certainly a typo, since the author meant one binder and wrote
/// two.
pub(super) fn alias_invariant_binder(
    name: &str,
    generic_params: &[GenericParameter],
    facets: &[Facet],
    symbols: &SymbolTable,
    span: Span,
) -> Result<Option<String>, ResolveError> {
    let invariants = facets::extract_invariants(facets);

    if invariants.is_empty() {
        return Ok(None);
    }

    let bound: HashSet<&str> = generic_params.iter().map(|param| param_name(param)).collect();

    let mut candidates: HashSet<String> = HashSet::new();

    for expr in &invariants {
        for identifier in referenced_identifiers(expr) {
            if bound.contains(identifier.as_str()) || symbols.lookup(&identifier).is_some() {
                continue;
            }

            candidates.insert(identifier);
        }
    }

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.into_iter().next()),

        _ => {
            let mut names: Vec<String> = candidates.into_iter().collect();
            names.sort();

            Err(ResolveError::AmbiguousInvariantBinder {
                type_name: name.to_string(),
                names,
                span,
            })
        }
    }
}

pub fn validate(program: &Program) -> Result<(), ResolveError> {
    for statement in &program.statements {
        match statement {
            Statement::Struct(decl) => {
                validate_facets(DeclKind::Struct, &decl.facets)?;
            }

            Statement::Macro(decl) => {
                validate_facets(DeclKind::Macro, &decl.facets)?;
            }

            Statement::TypeAlias(decl) => {
                validate_facets(DeclKind::TypeAlias, &decl.facets)?;
            }

            _ => {}
        }
    }

    Ok(())
}

fn validate_facets(decl_kind: DeclKind, decl_facets: &[Facet]) -> Result<(), ResolveError> {
    let mut counts: HashMap<&str, usize> = HashMap::new();

    for facet in decl_facets {
        let count = counts.entry(facet.name.as_str()).or_insert(0);
        *count += 1;

        // The parser already rejects unknown facet names, so `check`
        // returning `None` here would mean a parser/registry mismatch.
        let result = facets::check(&facet.name, decl_kind, *count).expect(
            "parser only produces facets with names registered in crate::facets",
        );

        result.map_err(|violation| match violation {
            Violation::NotApplicable => ResolveError::FacetNotApplicable {
                facet: facet.name.clone(),
                span: facet.span,
            },

            Violation::TooMany => ResolveError::DuplicateFacet {
                facet: facet.name.clone(),
                span: facet.span,
            },
        })?;
    }

    Ok(())
}
