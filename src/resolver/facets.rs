//! Drives structural validation of facets by calling into each facet's own
//! `check` function ([`crate::facets`]) — this file doesn't decide what's
//! valid for a given facet itself, it just counts occurrences per name and
//! translates the [`crate::facets::Violation`] each `check` call returns
//! into a [`ResolveError`]. Actually *enforcing* a facet's own behavior
//! (running `invariant`'s check, firing `before`/`after` hooks) is
//! separate, later work that needs infrastructure (const evaluation,
//! invocation binding) this crate doesn't have yet.

use std::collections::HashMap;

use crate::ast::{Facet, Program, Statement};
use crate::facets::{self, DeclKind, Violation};

use super::ResolveError;

pub fn validate(program: &Program) -> Result<(), ResolveError> {
    for statement in &program.statements {
        match statement {
            Statement::Struct(decl) => {
                validate_facets(DeclKind::Struct, &decl.facets)?;
            }

            Statement::Macro(decl) => {
                validate_facets(DeclKind::Macro, &decl.facets)?;
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
