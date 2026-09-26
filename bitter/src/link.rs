//! Multi-file merge + link (Phase 6, `docs/sections-and-linking/PROGRESS.md`):
//! given several already-compiled files' own `EmittedEntry` streams plus
//! each one's `pub`-label-position manifest, concatenates same-named
//! sections across all of them (in argument order — "Decided scope"),
//! builds one combined symbol table of every `pub` label, and substitutes
//! every `EmittedValue::Deferred { module, symbol }` (Phase 5) it finds —
//! however deeply nested inside a `Struct`/`Enum` tree — with the concrete
//! `EmittedValue::Int` its target actually resolved to.
//!
//! Local label positions (`Deferred.Pos`, which `std.bitter.deferred`'s
//! `span` wraps its label endpoints in) are translated from each input's own
//! entry numbering into the merged one, so a branch or `[rel label]` still
//! lands on the same entry after its section was regrouped.
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

/// One already-compiled input to a link: `file` is the `.basm` source it
/// came from (for messages), `module` its module path (matched against a
/// `Deferred` value's own `module`), `entries` its own `.em` content,
/// `labels` its own `pub`-label positions (the `.em` file's `exports`) —
/// every position is *local* to this
/// input's own `entries` (`0..=entries.len()`, the same
/// "how-many-real-entries-precede-it" count `AliasResolver::
/// record_label_position` already uses; `entries.len()` itself means
/// "after every entry this file emits").
pub struct LinkInput {
    pub file: PathBuf,
    /// `file`'s module path — what a `Deferred` names it by.
    pub module: String,
    pub entries: Vec<EmittedEntry>,
    pub labels: HashMap<String, usize>,
}

/// A linked program: the merged entry stream (section tags discarded) plus
/// every input's `pub` labels translated to positions in that stream — for
/// `bitter build --entry`, which needs to know where a named label ended up.
#[derive(Debug)]
pub struct Linked {
    pub values: Vec<EmittedValue>,
    pub labels: HashMap<String, usize>,
}

pub fn link(inputs: Vec<LinkInput>) -> Result<Linked, String> {
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

    // Each merged value remembers which input it came from, so its
    // `Deferred.Pos` label positions can be translated with that input's
    // own numbering once every entry has been placed (a forward reference
    // points at an entry that hasn't been placed yet at the time its
    // referrer is).
    let mut merged: Vec<(usize, EmittedValue)> = Vec::new();

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
                merged.push((input_index, entry.value.clone()));
            }
        }
    }

    let layout = Layout { inputs: &inputs, global_index: &global_index, merged_len: merged.len() };

    // One combined table of every `pub` label across every input, in
    // global-index terms — `name -> (declaring file, global position)`.
    // A name declared in two different inputs is a hard error ("Deferred,
    // not rejected" in the design doc confirms this, by analogy with a
    // real linker's "multiple definition" error), checked before anything
    // is substituted.
    let mut symbols: HashMap<String, (String, usize)> = HashMap::new();

    for (input_index, input) in inputs.iter().enumerate() {
        for (name, &local_position) in &input.labels {
            let position = layout.global_position(input_index, local_position, name)?;

            if let Some((existing_module, _)) = symbols.get(name)
                && existing_module != &input.module
            {
                return Err(format!(
                    "duplicate `pub` label `{name}`: declared in both `{existing_module}` and `{}`",
                    input.module,
                ));
            }

            symbols.insert(name.clone(), (input.module.clone(), position));
        }
    }

    // Local label positions first, while each value still carries its own
    // input's numbering; cross-unit `Deferred` placeholders second, since
    // those resolve straight to global positions that must not be
    // translated again.
    let mut values = Vec::with_capacity(merged.len());
    for (input_index, mut value) in merged {
        translate_positions_in_place(&mut value, input_index, &layout)?;
        resolve_deferred_in_place(&mut value, &symbols, layout.merged_len)?;
        values.push(value);
    }

    let labels = symbols.into_iter().map(|(name, (_, position))| (name, position)).collect();

    Ok(Linked { values, labels })
}

struct Layout<'a> {
    inputs: &'a [LinkInput],
    global_index: &'a HashMap<(usize, usize), usize>,
    merged_len: usize,
}

impl Layout<'_> {
    /// Where input `input_index`'s local position `local` (a label's value:
    /// "the entry that follows it", `0..=entries.len()`) ended up in the
    /// merged stream. `what` names the label or reference for errors.
    fn global_position(&self, input_index: usize, local: usize, what: &str) -> Result<usize, String> {
        let input = &self.inputs[input_index];

        if local < input.entries.len() {
            return self.global_index.get(&(input_index, local)).copied().ok_or_else(|| {
                format!(
                    "internal error: `{what}` (position {local} in `{}`) has no recorded global \
                     index after merging — every one of this file's own entries should have \
                     been placed",
                    input.file.display(),
                )
            });
        }

        if local > input.entries.len() {
            return Err(format!(
                "`{what}` refers to position {local} in `{}`, past its {} emitted entries",
                input.file.display(),
                input.entries.len(),
            ));
        }

        // Trailing: the label sits after every entry *this file* emits.
        // Well-defined when the file emits at least one entry ("one past
        // wherever my own last entry actually landed", regardless of
        // section reordering); for a file that emits nothing at all under
        // this label (e.g. a pure marker with no local code — this
        // input's `entries` is empty), there's no entry of its own to
        // anchor to, so it falls back to "the very end of the whole merged
        // program" — a deterministic, documented choice for a genuinely
        // anchorless case, not a claim that it's the only sensible one.
        match input.entries.len() {
            0 => Ok(self.merged_len),
            n => self.global_position(input_index, n - 1, what).map(|position| position + 1),
        }
    }
}

/// Rewrites every `Deferred.Pos(n)` inside `value` (a label position in
/// input `input_index`'s own numbering — see `std.bitter.deferred`'s doc on
/// `Pos`) to the merged-stream position of the same entry. A `Pos` whose
/// payload is a cross-unit `Deferred` placeholder is left alone: that one
/// resolves to a global position already, in `resolve_deferred_in_place`.
///
/// A label is "the entry that follows it" in source order, so one written
/// just before a `section` switch, with nothing emitted in its own section
/// after it, follows the next entry wherever that lands — the same meaning
/// it has in `bitter encode`'s unregrouped output.
fn translate_positions_in_place(value: &mut EmittedValue, input_index: usize, layout: &Layout) -> Result<(), String> {
    match value {
        EmittedValue::Enum { id, variant, payload, .. } if id == crate::pack::DEFERRED && variant == "Pos" => {
            let Some(payload) = payload else { return Ok(()) };
            let EmittedValue::Int { value: raw } = payload.as_mut() else { return Ok(()) };

            let local: usize = raw
                .parse()
                .map_err(|_| format!("`Deferred.Pos({raw})` isn't a valid entry position"))?;
            let global = layout.global_position(input_index, local, &format!("Deferred.Pos({raw})"))?;
            *raw = global.to_string();
            Ok(())
        }

        EmittedValue::Enum { payload, .. } => {
            if let Some(payload) = payload {
                translate_positions_in_place(payload, input_index, layout)?;
            }
            Ok(())
        }

        EmittedValue::Struct { fields, .. } => {
            for (_, field) in fields {
                translate_positions_in_place(field, input_index, layout)?;
            }
            Ok(())
        }

        EmittedValue::Int { .. } | EmittedValue::Deferred { .. } => Ok(()),
    }
}

/// The module whose labels `bitter` defines itself (`std/bitter/link.basm`).
pub const LINK_MODULE: &str = "std.bitter.link";

// A position `bitter` defines, in merged-stream terms: see
// `std/bitter/link.basm`.
fn link_symbol(symbol: &str, merged_len: usize) -> Result<usize, String> {
    match symbol {
        "image_start" => Ok(0),
        "image_end" => Ok(merged_len),
        other => Err(format!("`{LINK_MODULE}` has no `{other}` — it defines `image_start` and `image_end`")),
    }
}

fn resolve_deferred_in_place(
    value: &mut EmittedValue,
    symbols: &HashMap<String, (String, usize)>,
    merged_len: usize,
) -> Result<(), String> {
    match value {
        EmittedValue::Deferred { module, symbol } if module == LINK_MODULE => {
            *value = EmittedValue::Int { value: link_symbol(symbol, merged_len)?.to_string() };
            Ok(())
        }

        EmittedValue::Deferred { module, symbol } => {
            let Some((declaring_module, position)) = symbols.get(symbol) else {
                return Err(format!(
                    "unresolved external symbol `{symbol}` from `{module}` — no linked input \
                     declares a `pub` label named `{symbol}`"
                ));
            };

            if declaring_module != module {
                return Err(format!(
                    "unresolved external symbol `{symbol}` from `{module}` — no linked input \
                     is that module (`{symbol}` is declared in `{declaring_module}` instead, \
                     which IS linked, but that's not the module this reference names)",
                ));
            }

            *value = EmittedValue::Int { value: position.to_string() };
            Ok(())
        }

        EmittedValue::Int { .. } => Ok(()),

        EmittedValue::Struct { fields, .. } => {
            for (_, field) in fields {
                resolve_deferred_in_place(field, symbols, merged_len)?;
            }
            Ok(())
        }

        EmittedValue::Enum { payload, .. } => {
            if let Some(payload) = payload {
                resolve_deferred_in_place(payload, symbols, merged_len)?;
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
            module: file.trim_end_matches(".basm").to_string(),
            entries,
            labels: labels.iter().map(|(name, position)| (name.to_string(), *position)).collect(),
        }
    }

    #[test]
    fn two_inputs_concatenate_in_argument_order_within_the_default_section() {
        let a = input("a.basm", vec![entry(int("1"), None), entry(int("2"), None)], &[]);
        let b = input("b.basm", vec![entry(int("3"), None)], &[]);

        let merged = link(vec![a, b]).unwrap().values;

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

        let merged = link(vec![a, b]).unwrap().values;

        assert_eq!(merged, vec![int("1"), int("2"), int("3")]);
    }

    #[test]
    fn a_deferred_value_resolves_to_its_targets_real_global_position() {
        let a = input(
            "a.basm",
            vec![entry(EmittedValue::Deferred { module: "b".to_string(), symbol: "target".to_string() }, None)],
            &[],
        );
        // `b`'s own entry precedes its `pub target:` label (position 1) —
        // one real entry, then the label.
        let b = input("b.basm", vec![entry(int("99"), None)], &[("target", 1)]);

        let merged = link(vec![a, b]).unwrap().values;

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
            vec![entry(EmittedValue::Deferred { module: "b".to_string(), symbol: "target".to_string() }, None)],
            &[],
        );
        // `target` sits *before* b's one entry this time (position 0).
        let b = input("b.basm", vec![entry(int("99"), None)], &[("target", 0)]);

        let merged = link(vec![a, b]).unwrap().values;

        assert_eq!(merged, vec![int("1"), int("99")]);
    }

    #[test]
    fn a_deferred_value_nested_inside_a_struct_field_is_still_resolved() {
        let a = input(
            "a.basm",
            vec![entry(
                EmittedValue::Struct {
                    id: crate::pack::BITS.to_string(),
                    args: vec![],
                    fields: vec![(
                        "value".to_string(),
                        EmittedValue::Deferred { module: "b".to_string(), symbol: "target".to_string() },
                    )],
                },
                None,
            )],
            &[],
        );
        let b = input("b.basm", vec![], &[("target", 0)]);

        let merged = link(vec![a, b]).unwrap().values;

        let EmittedValue::Struct { fields, .. } = &merged[0] else { panic!("expected a struct") };
        // `target` has no local entries at all in `b` (the anchorless,
        // whole-file-empty case) — falls back to "end of the whole merged
        // program", which is just index 1 here (a's own one entry).
        assert_eq!(fields[0].1, int("1"));
    }

    fn pos(position: &str) -> EmittedValue {
        EmittedValue::Enum {
            id: crate::pack::DEFERRED.to_string(),
            args: vec![],
            variant: "Pos".to_string(),
            payload: Some(Box::new(int(position))),
        }
    }

    #[test]
    fn label_positions_follow_their_entry_through_a_reopened_section() {
        // .text: pos(2) (a branch to the entry after .data), .data: 99,
        // .text: 7. Grouped: [pos, 7, 99] — local entry 2 (the `7`) is now
        // global entry 1.
        let a = input(
            "a.basm",
            vec![
                entry(pos("2"), Some(".text")),
                entry(int("99"), Some(".data")),
                entry(int("7"), Some(".text")),
            ],
            &[],
        );

        let merged = link(vec![a]).unwrap().values;

        assert_eq!(merged, vec![pos("1"), int("7"), int("99")]);
    }

    #[test]
    fn label_positions_in_a_later_input_are_offset_past_earlier_inputs() {
        let a = input("a.basm", vec![entry(int("1"), None), entry(int("2"), None)], &[]);
        // b's own local entry 0 (itself) and trailing position 1.
        let b = input("b.basm", vec![entry(pos("0"), None), entry(pos("2"), None)], &[]);

        let merged = link(vec![a, b]).unwrap().values;

        assert_eq!(merged, vec![int("1"), int("2"), pos("2"), pos("4")]);
    }

    #[test]
    fn pub_label_positions_are_reported_in_merged_terms() {
        let a = input("a.basm", vec![entry(int("1"), Some(".data"))], &[]);
        let b = input("b.basm", vec![entry(int("2"), Some(".text"))], &[("_start", 0)]);

        let linked = link(vec![a, b]).unwrap();

        assert_eq!(linked.labels.get("_start"), Some(&1));
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
    fn image_start_and_end_span_the_whole_linked_image() {
        let link_ref = |symbol: &str| {
            EmittedValue::Deferred { module: LINK_MODULE.to_string(), symbol: symbol.to_string() }
        };
        let a = input("a.basm", vec![entry(link_ref("image_start"), None), entry(int("1"), None)], &[]);
        let b = input("b.basm", vec![entry(link_ref("image_end"), None)], &[]);

        let merged = link(vec![a, b]).unwrap().values;

        // Three entries in all: image_start is 0, image_end is 3.
        assert_eq!(merged, vec![int("0"), int("1"), int("3")]);
    }

    #[test]
    fn an_unknown_link_symbol_is_a_clear_error() {
        let a = input(
            "a.basm",
            vec![entry(EmittedValue::Deferred { module: LINK_MODULE.to_string(), symbol: "nope".to_string() }, None)],
            &[],
        );
        let error = link(vec![a]).unwrap_err();
        assert!(error.contains("has no `nope`"), "{error}");
    }

    #[test]
    fn a_deferred_value_with_no_matching_linked_input_is_a_clear_error() {
        let a = input(
            "a.basm",
            vec![entry(
                EmittedValue::Deferred { module: "missing".to_string(), symbol: "ghost".to_string() },
                None,
            )],
            &[],
        );

        let error = link(vec![a]).unwrap_err();
        assert!(error.contains("ghost"), "{error}");
        assert!(error.contains("`missing`"), "{error}");
    }
}
