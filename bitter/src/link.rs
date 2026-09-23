//! Multi-file merge + link (Phase 6, `docs/sections-and-linking/PROGRESS.md`):
//! given several already-compiled files' own `EmittedEntry` streams plus
//! each one's `pub`-label-position manifest, concatenates same-named
//! sections across all of them (in argument order — "Decided scope"),
//! builds one combined symbol table of every `pub` label, and substitutes
//! every `EmittedValue::Deferred { file, symbol }` (Phase 5) it finds —
//! however deeply nested inside a `Struct`/`Enum` tree — with the concrete
//! `EmittedValue::Int` its target actually resolved to.
//!
//! The result is a plain `Vec<EmittedValue>`, section tags discarded (their
//! job — deciding final entry order — is already done by the time this
//! returns), handed to `crate::pack::pack_stream` unchanged: `bitter
//! encode`'s own resolve-everything logic is reused exactly as-is, not
//! reimplemented here, matching "Decided scope"'s "single resolution pass
//! over the merged, still-symbolic result."

use std::collections::HashMap;
use std::path::PathBuf;

use bitterasm::emit::{EmittedEntry, EmittedValue};

/// One already-compiled input to a link: `file` is the canonicalized path
/// of the `.basm` source it came from (matched against a `Deferred`
/// value's own `file` field, which `bitterasm compile` records the same
/// way — see `ast::ExternLabel::file`'s doc), `entries` its own `.em`
/// content, `labels` its own `pub`-label-position manifest (from
/// `bitterasm compile --labels`) — every position is *local* to this
/// input's own `entries` (`0..=entries.len()`, the same
/// "how-many-real-entries-precede-it" count `AliasResolver::
/// record_label_position` already uses; `entries.len()` itself means
/// "after every entry this file emits").
pub struct LinkInput {
    pub file: PathBuf,
    pub entries: Vec<EmittedEntry>,
    pub labels: HashMap<String, usize>,
}

pub fn link(inputs: Vec<LinkInput>) -> Result<Vec<EmittedValue>, String> {
    // Section order = first-seen order across every input, in argument
    // order — matches "Decided scope"'s "concatenate same-named sections
    // across all inputs (order = command-line argument order)". The
    // implicit default section (`None`) participates in this ordering
    // exactly like any named one; it just usually appears first, since
    // most programs never declare a `section` statement at all.
    let mut section_order: Vec<Option<String>> = Vec::new();
    for input in &inputs {
        for entry in &input.entries {
            if !section_order.contains(&entry.section) {
                section_order.push(entry.section.clone());
            }
        }
    }

    let mut merged: Vec<EmittedValue> = Vec::new();

    // `(input_index, local_index) -> global_index` — built up as entries
    // are actually placed, so a label's local position (see `LinkInput`'s
    // doc) can be translated to where its owning entry really ended up,
    // regardless of how much section-based reordering moved it.
    let mut global_index: HashMap<(usize, usize), usize> = HashMap::new();

    for section in &section_order {
        for (input_index, input) in inputs.iter().enumerate() {
            for (local_index, entry) in input.entries.iter().enumerate() {
                if &entry.section != section {
                    continue;
                }
                global_index.insert((input_index, local_index), merged.len());
                merged.push(entry.value.clone());
            }
        }
    }

    // One combined table of every `pub` label across every input, in
    // global-index terms — `name -> (declaring file, global position)`.
    // A name declared in two different inputs is a hard error ("Deferred,
    // not rejected" in the design doc confirms this, by analogy with a
    // real linker's "multiple definition" error), checked before anything
    // is substituted.
    let mut symbols: HashMap<String, (PathBuf, usize)> = HashMap::new();

    for (input_index, input) in inputs.iter().enumerate() {
        for (name, &local_position) in &input.labels {
            let position = if local_position < input.entries.len() {
                *global_index.get(&(input_index, local_position)).ok_or_else(|| {
                    format!(
                        "internal error: `{name}` (position {local_position} in `{}`) has no \
                         recorded global index after merging — every one of this file's own \
                         entries should have been placed",
                        input.file.display(),
                    )
                })?
            } else {
                // Trailing: this label sits after every entry *this file*
                // emits. Well-defined when the file emits at least one
                // entry ("one past wherever my own last entry actually
                // landed", regardless of section reordering); for a file
                // that emits nothing at all under this label (e.g. a pure
                // marker with no local code — this input's `entries` is
                // empty), there's no entry of its own to anchor to, so it
                // falls back to "the very end of the whole merged
                // program" — a deterministic, documented choice for a
                // genuinely anchorless case, not a claim that it's the
                // only sensible one.
                match input.entries.len() {
                    0 => merged.len(),
                    n => {
                        *global_index.get(&(input_index, n - 1)).ok_or_else(|| {
                            format!(
                                "internal error: `{name}`'s trailing position in `{}` has no \
                                 recorded global index after merging",
                                input.file.display(),
                            )
                        })? + 1
                    }
                }
            };

            if let Some((existing_file, _)) = symbols.get(name)
                && existing_file != &input.file
            {
                return Err(format!(
                    "duplicate `pub` label `{name}`: declared in both `{}` and `{}`",
                    existing_file.display(),
                    input.file.display(),
                ));
            }

            symbols.insert(name.clone(), (input.file.clone(), position));
        }
    }

    for value in &mut merged {
        resolve_deferred_in_place(value, &symbols)?;
    }

    Ok(merged)
}

fn resolve_deferred_in_place(
    value: &mut EmittedValue,
    symbols: &HashMap<String, (PathBuf, usize)>,
) -> Result<(), String> {
    match value {
        EmittedValue::Deferred { file, symbol } => {
            let Some((declaring_file, position)) = symbols.get(symbol) else {
                return Err(format!(
                    "unresolved external symbol `{symbol}` from `{file}` — no linked input \
                     declares a `pub` label named `{symbol}`"
                ));
            };

            if declaring_file.display().to_string() != *file {
                return Err(format!(
                    "unresolved external symbol `{symbol}` from `{file}` — no linked input \
                     compiles that file (`{symbol}` is declared in `{}` instead, which IS \
                     linked, but that's not the file this reference names)",
                    declaring_file.display(),
                ));
            }

            *value = EmittedValue::Int { value: position.to_string() };
            Ok(())
        }

        EmittedValue::Int { .. } => Ok(()),

        EmittedValue::Struct { fields, .. } => {
            for (_, field) in fields {
                resolve_deferred_in_place(field, symbols)?;
            }
            Ok(())
        }

        EmittedValue::Enum { payload, .. } => {
            if let Some(payload) = payload {
                resolve_deferred_in_place(payload, symbols)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(value: &str) -> EmittedValue {
        EmittedValue::Int { value: value.to_string() }
    }

    fn entry(value: EmittedValue, section: Option<&str>) -> EmittedEntry {
        EmittedEntry { value, section: section.map(str::to_string) }
    }

    fn input(file: &str, entries: Vec<EmittedEntry>, labels: &[(&str, usize)]) -> LinkInput {
        LinkInput {
            file: PathBuf::from(file),
            entries,
            labels: labels.iter().map(|(name, position)| (name.to_string(), *position)).collect(),
        }
    }

    #[test]
    fn two_inputs_concatenate_in_argument_order_within_the_default_section() {
        let a = input("a.basm", vec![entry(int("1"), None), entry(int("2"), None)], &[]);
        let b = input("b.basm", vec![entry(int("3"), None)], &[]);

        let merged = link(vec![a, b]).unwrap();

        assert_eq!(merged, vec![int("1"), int("2"), int("3")]);
    }

    #[test]
    fn same_named_sections_are_grouped_by_first_seen_order_across_inputs() {
        // `a` declares `.text` then `.data`; `b` only has `.data`. Expected
        // grouping: every `.text` entry (only `a` has any) first, then
        // every `.data` entry, `a`'s own before `b`'s (argument order).
        let a = input(
            "a.basm",
            vec![entry(int("1"), Some(".text")), entry(int("2"), Some(".data"))],
            &[],
        );
        let b = input("b.basm", vec![entry(int("3"), Some(".data"))], &[]);

        let merged = link(vec![a, b]).unwrap();

        assert_eq!(merged, vec![int("1"), int("2"), int("3")]);
    }

    #[test]
    fn a_deferred_value_resolves_to_its_targets_real_global_position() {
        let a = input(
            "a.basm",
            vec![entry(EmittedValue::Deferred { file: "b.basm".to_string(), symbol: "target".to_string() }, None)],
            &[],
        );
        // `b`'s own entry precedes its `pub target:` label (position 1) —
        // one real entry, then the label.
        let b = input("b.basm", vec![entry(int("99"), None)], &[("target", 1)]);

        let merged = link(vec![a, b]).unwrap();

        // Global order: a's one entry (index 0), then b's one entry
        // (index 1) — `target`'s local position 1 == b's own entry count,
        // the trailing case, so it resolves to "one past b's own last
        // entry" = index 2.
        assert_eq!(merged, vec![int("2"), int("99")]);
    }

    #[test]
    fn a_deferred_value_can_resolve_to_a_non_trailing_position() {
        let a = input(
            "a.basm",
            vec![entry(EmittedValue::Deferred { file: "b.basm".to_string(), symbol: "target".to_string() }, None)],
            &[],
        );
        // `target` sits *before* b's one entry this time (position 0).
        let b = input("b.basm", vec![entry(int("99"), None)], &[("target", 0)]);

        let merged = link(vec![a, b]).unwrap();

        assert_eq!(merged, vec![int("1"), int("99")]);
    }

    #[test]
    fn a_deferred_value_nested_inside_a_struct_field_is_still_resolved() {
        let a = input(
            "a.basm",
            vec![entry(
                EmittedValue::Struct {
                    name: "bits".to_string(),
                    args: vec![],
                    fields: vec![(
                        "value".to_string(),
                        EmittedValue::Deferred { file: "b.basm".to_string(), symbol: "target".to_string() },
                    )],
                },
                None,
            )],
            &[],
        );
        let b = input("b.basm", vec![], &[("target", 0)]);

        let merged = link(vec![a, b]).unwrap();

        let EmittedValue::Struct { fields, .. } = &merged[0] else { panic!("expected a struct") };
        // `target` has no local entries at all in `b` (the anchorless,
        // whole-file-empty case) — falls back to "end of the whole merged
        // program", which is just index 1 here (a's own one entry).
        assert_eq!(fields[0].1, int("1"));
    }

    #[test]
    fn duplicate_pub_label_across_two_inputs_is_a_hard_error() {
        let a = input("a.basm", vec![], &[("target", 0)]);
        let b = input("b.basm", vec![], &[("target", 0)]);

        let error = link(vec![a, b]).unwrap_err();
        assert!(error.contains("duplicate"), "{error}");
        assert!(error.contains("target"), "{error}");
    }

    #[test]
    fn a_deferred_value_with_no_matching_linked_input_is_a_clear_error() {
        let a = input(
            "a.basm",
            vec![entry(
                EmittedValue::Deferred { file: "missing.basm".to_string(), symbol: "ghost".to_string() },
                None,
            )],
            &[],
        );

        let error = link(vec![a]).unwrap_err();
        assert!(error.contains("ghost"), "{error}");
        assert!(error.contains("missing.basm"), "{error}");
    }
}
