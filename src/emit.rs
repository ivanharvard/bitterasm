//! Reifies a resolved [`Value`] into [`EmittedValue`] — a self-contained,
//! serializable shape with no dependency on any in-process state, unlike
//! `Value` itself. This is what `bitterasm compile` writes to a `.em` file
//! and what `bitter` is meant to read back.
//!
//! The one thing that actually needs rewriting is `Value::Struct`'s
//! [`SymbolId`] — it only means anything against the [`SymbolTable`] that
//! produced it, so [`reify_value`] resolves it to the struct's `.em` id
//! once, here, rather than asking every later reader to carry a
//! `SymbolTable` around just to make sense of an id.
//!
//! A whole `.em` file is an [`EmFile`]: a versioned header around the
//! entries. `docs/book/src/tools/em-format.md` is its specification.

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

use serde::{Deserialize, Serialize};

use crate::resolver::{BuiltinType, ResolvedGenericArg, ResolvedType, SymbolId, SymbolTable, Value};

/// The `.em` format version this crate writes and reads. Goes up only on an
/// incompatible change to the file's structure.
pub const EM_VERSION: u64 = 1;

/// Language features a `.em` file can require its reader to understand —
/// see [`EmFile::requires`].
pub mod features {
    /// Some entry carries a `section`: the reader must group entries by
    /// section (in first-appearance order) before laying them out.
    pub const SECTIONS: &str = "sections";
    /// Some value is a `Deferred` reference to another file's `pub` label,
    /// which only a linker can resolve.
    pub const EXTERN_LABELS: &str = "extern-labels";
    /// The program has a link stage (`@link`): the entries in the
    /// [`LINK_SECTION`](super::LINK_SECTION) describe the output, writing
    /// and placing the other sections, rather than all being written in
    /// order.
    pub const LINK: &str = "link";

    /// Every feature this version of the format defines.
    pub const ALL: &[&str] = &[SECTIONS, EXTERN_LABELS, LINK];
}

/// The section a program's `@link` macro emits into. No `section` line can
/// name it.
pub const LINK_SECTION: &str = "@link";

/// A whole `.em` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmFile {
    /// Always [`EM_VERSION`] when written by this crate.
    pub version: u64,

    /// The language features this program actually uses (see
    /// [`features`]). A reader must refuse a file that requires a feature
    /// it doesn't know — ignoring one would produce wrong output silently.
    pub requires: Vec<String>,

    /// The compiled file's own module path, which other files'
    /// `Deferred { module, .. }` references name it by.
    pub module: String,

    /// Every top-level `pub` label's position: how many entries precede
    /// it, counted in `entries`.
    pub exports: BTreeMap<String, u64>,

    pub entries: Vec<EmittedEntry>,
}

impl EmFile {
    /// A file for `entries`, requiring exactly the features they use.
    pub fn new(module: String, exports: BTreeMap<String, u64>, entries: Vec<EmittedEntry>) -> Self {
        let requires = required_features(
            entries.iter().any(|entry| entry.section.is_some()),
            entries.iter().any(|entry| contains_deferred(&entry.value)),
            entries.iter().any(|entry| entry.section.as_deref() == Some(LINK_SECTION)),
        );

        Self { version: EM_VERSION, requires, module, exports, entries }
    }

    /// Reads a `.em` file, refusing one this reader can't handle correctly:
    /// the pre-v1 plain-list format, another `version`, or a required
    /// feature not in `supported`.
    pub fn parse(json: &str, supported: &[&str]) -> Result<Self, String> {
        match json.trim_start().as_bytes().first() {
            Some(b'{') => {}
            Some(b'[') => {
                return Err(format!(
                    "this is an unversioned `.em` file from before `.em` version {EM_VERSION}; \
                     recompile its source with this `bitterasm`"
                ));
            }
            _ => return Err("a `.em` file must be a JSON object".to_string()),
        }

        // Only `version` first, skipping everything else unread, so a file
        // from another version is refused before its entries are parsed
        // against this version's shapes.
        #[derive(Deserialize)]
        struct Header {
            version: Option<serde_json::Value>,
        }
        let header: Header = serde_json::from_str(json).map_err(|error| error.to_string())?;

        match header.version.as_ref().and_then(serde_json::Value::as_u64) {
            Some(EM_VERSION) => {}
            Some(other) => {
                return Err(format!(
                    "`.em` version {other} isn't supported; this reader understands version {EM_VERSION}"
                ));
            }
            None => return Err("this `.em` file has no `version`".to_string()),
        }

        let file: EmFile = serde_json::from_str(json).map_err(|error| error.to_string())?;

        for feature in &file.requires {
            if !supported.contains(&feature.as_str()) {
                return Err(format!(
                    "this `.em` file requires `{feature}`, which this reader doesn't support"
                ));
            }
        }

        Ok(file)
    }
}

/// `requires` for a file that has sections and/or deferred values, in the
/// order [`features::ALL`] lists them.
fn required_features(has_sections: bool, has_deferred: bool, has_link: bool) -> Vec<String> {
    let mut requires = Vec::new();
    if has_sections {
        requires.push(features::SECTIONS.to_string());
    }
    if has_deferred {
        requires.push(features::EXTERN_LABELS.to_string());
    }
    if has_link {
        requires.push(features::LINK.to_string());
    }
    requires
}

/// Writes a `.em` file as compact JSON, converting one value at a time
/// rather than building the whole file in memory first. `sections` is
/// index-aligned with `values`. Reads back as the same [`EmFile`] that
/// [`EmFile::new`] would build from the converted entries.
pub fn write_em<W: Write>(
    out: W,
    module: &str,
    exports: &BTreeMap<String, u64>,
    ids: &TypeIds,
    values: &[Value],
    sections: &[Option<String>],
) -> std::io::Result<()> {
    let mut out = BufWriter::with_capacity(1 << 16, out);
    let requires = required_features(
        sections.iter().any(Option::is_some),
        values.iter().any(value_contains_extern_label),
        sections.iter().any(|section| section.as_deref() == Some(LINK_SECTION)),
    );

    write!(out, "{{\"version\":{EM_VERSION},\"requires\":")?;
    serde_json::to_writer(&mut out, &requires)?;
    out.write_all(b",\"module\":")?;
    serde_json::to_writer(&mut out, module)?;
    out.write_all(b",\"exports\":")?;
    serde_json::to_writer(&mut out, exports)?;
    out.write_all(b",\"entries\":[")?;
    for (index, (value, section)) in values.iter().zip(sections).enumerate() {
        if index > 0 {
            out.write_all(b",")?;
        }
        let entry = EmittedEntry { value: reify_value(ids, value), section: section.clone() };
        serde_json::to_writer(&mut out, &entry)?;
    }
    out.write_all(b"]}")?;
    out.flush()
}

fn value_contains_extern_label(value: &Value) -> bool {
    match value {
        Value::ExternLabel { .. } => true,
        Value::Int(_) | Value::Macro(_) | Value::Label(_) | Value::Record(_) => false,
        Value::Struct { fields, .. } => fields.iter().any(|(_, field)| value_contains_extern_label(field)),
        Value::Enum { payload, .. } => payload.as_deref().is_some_and(value_contains_extern_label),
    }
}

/// Indents compact JSON as it's written through, in one pass and without
/// parsing it into a tree: two spaces per level, `": "` after keys, and
/// `[]`/`{}` for empty containers, the same layout as
/// `serde_json::to_string_pretty`. Expects JSON with no whitespace outside
/// strings, which is what `serde_json::to_writer` produces.
pub struct PrettyJson<W: Write> {
    out: W,
    depth: usize,
    in_string: bool,
    escaped: bool,
    // A `[` or `{` not yet written: whether it's empty depends on the next
    // byte.
    pending_open: Option<u8>,
}

impl<W: Write> PrettyJson<W> {
    pub fn new(out: W) -> Self {
        Self { out, depth: 0, in_string: false, escaped: false, pending_open: None }
    }

    /// Writes any trailing newline and returns the inner writer.
    pub fn finish(mut self) -> std::io::Result<W> {
        if let Some(open) = self.pending_open.take() {
            self.out.write_all(&[open])?;
        }
        self.out.write_all(b"\n")?;
        self.out.flush()?;
        Ok(self.out)
    }

    fn newline(&mut self) -> std::io::Result<()> {
        self.out.write_all(b"\n")?;
        for _ in 0..self.depth {
            self.out.write_all(b"  ")?;
        }
        Ok(())
    }

    fn byte(&mut self, byte: u8) -> std::io::Result<()> {
        if self.in_string {
            if self.escaped {
                self.escaped = false;
            } else if byte == b'\\' {
                self.escaped = true;
            } else if byte == b'"' {
                self.in_string = false;
            }
            return self.out.write_all(&[byte]);
        }

        if let Some(open) = self.pending_open.take() {
            if (open, byte) == (b'[', b']') || (open, byte) == (b'{', b'}') {
                return self.out.write_all(&[open, byte]);
            }
            self.out.write_all(&[open])?;
            self.depth += 1;
            self.newline()?;
        }

        match byte {
            b'[' | b'{' => self.pending_open = Some(byte),
            b']' | b'}' => {
                self.depth -= 1;
                self.newline()?;
                self.out.write_all(&[byte])?;
            }
            b',' => {
                self.out.write_all(b",")?;
                self.newline()?;
            }
            b':' => self.out.write_all(b": ")?,
            b'"' => {
                self.in_string = true;
                self.out.write_all(b"\"")?;
            }
            other => self.out.write_all(&[other])?,
        }
        Ok(())
    }
}

impl<W: Write> Write for PrettyJson<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &byte in buf {
            self.byte(byte)?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.out.flush()
    }
}

fn contains_deferred(value: &EmittedValue) -> bool {
    match value {
        EmittedValue::Deferred { .. } => true,
        EmittedValue::Int { .. } => false,
        EmittedValue::Struct { fields, .. } => fields.iter().any(|(_, field)| contains_deferred(field)),
        EmittedValue::Enum { payload, .. } => payload.as_deref().is_some_and(contains_deferred),
    }
}

/// One `.em` entry: a reified value plus which section was active when it
/// was `@emit`'d. `section` is flattened into the same JSON object as
/// `value`'s own tagged fields, and omitted entirely when `None` — so a
/// program that never declares a `section` at all produces `.em` output
/// byte-for-byte identical to before this field existed (see "Decided
/// scope" in `docs/sections-and-linking/PROGRESS.md`: no `section`
/// statement = today's behavior, unchanged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmittedEntry {
    #[serde(flatten)]
    pub value: EmittedValue,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub section: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EmittedValue {
    // An arbitrary-precision `Int`'s decimal string, not a JSON number —
    // JSON numbers are typically read back as f64, which can't represent
    // every value an Int can.
    Int { value: String },

    /// `id` is the struct's module path + name (`std.binary.bits`) — see
    /// [`TypeIds`].
    Struct {
        id: String,
        args: Vec<EmittedGenericArg>,
        fields: Vec<(String, EmittedValue)>,
    },
    Enum {
        id: String,
        args: Vec<EmittedGenericArg>,
        variant: String,
        payload: Option<Box<EmittedValue>>,
    },

    /// The value of a `pub` label imported from another file (Phase 5, see
    /// `docs/sections-and-linking/PROGRESS.md`) — `symbol`'s value from
    /// `file`, not yet known. `file` is the declaring file's already-
    /// canonicalized absolute path. Left for a later `bitter build`/`bitter
    /// exec` link step to resolve, once it has every input file's own
    /// emitted stream to find `symbol`'s real position in; `bitter encode`
    /// alone can never resolve one on its own, the same way it can't
    /// resolve a bare `here()`/`span()` value with no `Positioned<N>`
    /// around it either.
    Deferred { module: String, symbol: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EmittedGenericArg {
    Const { value: String },
    Type(EmittedType),
}

// Its own tag key can't also be "kind": `Type(EmittedType)` above is a
// newtype variant, so serde flattens `EmittedType`'s own tagged
// representation into the same JSON object as `EmittedGenericArg`'s "kind"
// tag — two fields both named "kind" in one object, which serde's
// internally-tagged deserializer rejects as ambiguous the moment a real
// type generic argument (as opposed to a const one) is actually emitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type_kind")]
pub enum EmittedType {
    Builtin { name: String },
    Struct { id: String, args: Vec<EmittedGenericArg> },
    Enum { id: String, args: Vec<EmittedGenericArg> },
}

/// How `.em` identifies a struct, enum or type: its declaring module's
/// module path plus its declared name, e.g. `std.binary.bits`
/// ([`crate::loader::module_path_of`]). Unique within one program: a
/// module can't declare a name twice, and generated names are already
/// unique program-wide. The loader's internal `name#module` spelling for a
/// private declaration never reaches `.em`; a synthesized struct
/// (`__range$3`) keeps its counter, which is what tells two apart.
pub struct TypeIds<'a> {
    symbols: &'a SymbolTable,
    module_paths: &'a [String],
}

impl<'a> TypeIds<'a> {
    /// `module_paths` is indexed by the module ids `symbols` records.
    pub fn new(symbols: &'a SymbolTable, module_paths: &'a [String]) -> Self {
        Self { symbols, module_paths }
    }

    pub fn id(&self, symbol: SymbolId) -> String {
        let symbol = self.symbols.get(symbol);
        let name = symbol.name.split_once('#').map_or(symbol.name.as_str(), |(base, _)| base);
        format!("{}.{name}", self.module_paths[symbol.module])
    }
}

pub fn reify_value(ids: &TypeIds, value: &Value) -> EmittedValue {
    match value {
        Value::Int(int) => EmittedValue::Int { value: int.to_string() },

        // `nominal` is compile-time-only (an invariant is already checked
        // by the time anything reaches emission) — same reasoning as
        // `reify_type`'s `ResolvedType::Alias` handling just below.
        Value::Struct { symbol, args, fields, .. } => EmittedValue::Struct {
            id: ids.id(*symbol),
            args: args.iter().map(|arg| reify_generic_arg(ids, arg)).collect(),
            fields: fields
                .iter()
                .map(|(name, value)| (name.clone(), reify_value(ids, value)))
                .collect(),
        },
        Value::Enum { symbol, args, variant, payload } => EmittedValue::Enum {
            id: ids.id(*symbol),
            args: args.iter().map(|arg| reify_generic_arg(ids, arg)).collect(),
            variant: variant.clone(),
            payload: payload.as_ref().map(|value| Box::new(reify_value(ids, value))),
        },

        // Compile-time-only metaprogramming machinery, not data — see
        // `Value::Macro`'s doc. `@emit` rejects a macro, and any struct or
        // enum holding one, before it's recorded, so none ever gets here.
        Value::Macro(_) => unreachable!(
            "a macro is compile-time-only metaprogramming machinery and can never reach emission"
        ),
        // Same as `Value::Macro`: `@emit` rejects a label, and anything
        // holding one.
        Value::Label(_) => unreachable!("a label is compile-time only and can never reach emission"),
        Value::Record(_) => unreachable!("a space or section is compile-time only and can never reach emission"),

        // Phase 5 (`docs/sections-and-linking/PROGRESS.md`): a `pub` label
        // imported from another file, not yet resolved to a real position
        // — reified as its own distinct leaf, never `EmittedValue::Int`, so
        // `.em` keeps visibly marking it unresolved rather than lying about
        // a value nobody actually knows yet (real resolution happens later,
        // at a `bitter build`/`bitter exec` link step).
        Value::ExternLabel { module, name } => {
            EmittedValue::Deferred { module: module.clone(), symbol: name.clone() }
        }
    }
}

fn reify_generic_arg(ids: &TypeIds, arg: &ResolvedGenericArg) -> EmittedGenericArg {
    match arg {
        ResolvedGenericArg::Const(int) => EmittedGenericArg::Const { value: int.to_string() },

        ResolvedGenericArg::Type(ty) => EmittedGenericArg::Type(reify_type(ids, ty)),

        // A concrete Value only ever comes from naming a fully-resolved
        // type at a call site (`eval_call_value` in resolver/values.rs) —
        // there's no path that leaves one of its own generic args pointing
        // back at an outer, not-yet-instantiated const param.
        ResolvedGenericArg::ConstParam(name) => unreachable!(
            "a fully-evaluated value's generic args can't carry an unresolved \
             const param (`{name}`)"
        ),
        ResolvedGenericArg::Wildcard => unreachable!("signature wildcard reached emitted value"),
    }
}

fn reify_type(ids: &TypeIds, ty: &ResolvedType) -> EmittedType {
    match ty {
        ResolvedType::Builtin(BuiltinType::Int) => EmittedType::Builtin { name: "int".to_string() },
        ResolvedType::Builtin(BuiltinType::Label | BuiltinType::Space | BuiltinType::Section)
        | ResolvedType::LabelIn(_)
        | ResolvedType::Space(_) => {
            unreachable!("labels, spaces and sections are compile-time only, so no emitted value's type mentions one")
        }

        ResolvedType::Struct { symbol, args } => EmittedType::Struct {
            id: ids.id(*symbol),
            args: args.iter().map(|arg| reify_generic_arg(ids, arg)).collect(),
        },
        ResolvedType::Enum { symbol, args } => EmittedType::Enum {
            id: ids.id(*symbol),
            args: args.iter().map(|arg| reify_generic_arg(ids, arg)).collect(),
        },

        // Same reasoning as `ConstParam` above, one level up: a concrete
        // value's resolved type is never left in terms of an
        // uninstantiated type parameter.
        ResolvedType::TypeParameter { name } => unreachable!(
            "a fully-evaluated value's generic args can't carry an unresolved \
             type parameter (`{name}`)"
        ),

        // Nominal wrapping is a compile-time-only concept — whatever
        // invariant an alias carries has already been checked by the time
        // anything reaches emission, so the emitted shape just describes
        // the underlying structure `bitter` actually needs to encode.
        ResolvedType::Alias { .. } => reify_type(ids, ty.strip_alias()),

        // Same reasoning as `Value::Macro` in `reify_value` just above.
        ResolvedType::MacroType { .. } => unreachable!(
            "a macro's type is compile-time-only and can never reach emission"
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;

    use crate::ast::{literal_name, MacroDeclaration, Program, Statement};
    use crate::eval::Int;
    use crate::lexer;
    use crate::parser;
    use crate::resolver::{collect_symbols, AliasResolver};
    use crate::token::Span;

    use super::*;

    fn parse_fixture(name: &str) -> Program {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/emit")
            .join(name);

        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()));

        let tokens = lexer::lex(&source).expect("fixture should lex");
        parser::parse(tokens).expect("fixture should parse")
    }

    fn find_macro<'a>(program: &'a Program, name: &str) -> &'a MacroDeclaration {
        program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Statement::Macro(decl) if literal_name(&decl.name).as_deref() == Some(name) => Some(decl),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected a macro named `{name}`"))
    }

    fn int_entry(section: Option<&str>) -> EmittedEntry {
        EmittedEntry { value: EmittedValue::Int { value: "1".to_string() }, section: section.map(str::to_string) }
    }

    #[test]
    fn requires_lists_exactly_the_features_used() {
        let plain = EmFile::new("m".to_string(), Default::default(), vec![int_entry(None)]);
        assert!(plain.requires.is_empty());

        let sectioned = EmFile::new("m".to_string(), Default::default(), vec![int_entry(Some(".text"))]);
        assert_eq!(sectioned.requires, [features::SECTIONS]);

        // A `Deferred` nested inside a struct still counts.
        let nested = EmittedValue::Struct {
            id: "m.S".to_string(),
            args: Vec::new(),
            fields: vec![(
                "target".to_string(),
                EmittedValue::Deferred { module: "other".to_string(), symbol: "label".to_string() },
            )],
        };
        let linked = EmFile::new("m".to_string(), Default::default(), vec![EmittedEntry { value: nested, section: None }]);
        assert_eq!(linked.requires, [features::EXTERN_LABELS]);
    }

    #[test]
    fn parse_round_trips_what_new_writes() {
        let file = EmFile::new("m".to_string(), [("start".to_string(), 0)].into(), vec![int_entry(Some(".text"))]);
        let json = serde_json::to_string(&file).unwrap();
        assert_eq!(EmFile::parse(&json, features::ALL).unwrap(), file);
    }

    #[test]
    fn pretty_json_matches_serde_pretty() {
        let value = serde_json::json!({
            "empty_list": [],
            "empty_object": {},
            "tricky": "a \\\"quoted\\\" [string], {with: punctuation}\\\\",
            "nested": [[1, [2]], {"k": [true, null]}],
        });
        let compact = serde_json::to_string(&value).unwrap();
        let mut pretty = PrettyJson::new(Vec::new());
        pretty.write_all(compact.as_bytes()).unwrap();
        let written = String::from_utf8(pretty.finish().unwrap()).unwrap();
        assert_eq!(written, serde_json::to_string_pretty(&value).unwrap() + "\n");
    }

    #[test]
    fn write_em_reads_back_like_em_file_new() {
        let program = parse_fixture("generic_alias.basm");
        let symbols = collect_symbols(&program, &vec![0; program.statements.len()]).unwrap();
        let module_paths = ["fixture".to_string()];
        let ids = TypeIds::new(&symbols, &module_paths);
        let bits = symbols.lookup("bits").unwrap();
        let values = vec![
            Value::Int(Int::from(-7)),
            Value::Struct {
                symbol: bits,
                args: vec![ResolvedGenericArg::Const(Int::from(8))],
                fields: vec![("value".to_string(), Value::Int(Int::from(3)))],
                nominal: None,
            },
        ];
        let sections = vec![None, Some(".text".to_string())];
        let exports = BTreeMap::from([("start".to_string(), 1)]);

        let mut written = Vec::new();
        write_em(&mut written, "fixture", &exports, &ids, &values, &sections).unwrap();

        let entries = values
            .iter()
            .zip(&sections)
            .map(|(value, section)| EmittedEntry { value: reify_value(&ids, value), section: section.clone() })
            .collect();
        let expected = EmFile::new("fixture".to_string(), exports, entries);
        let json = String::from_utf8(written).unwrap();
        assert_eq!(EmFile::parse(&json, features::ALL).unwrap(), expected);
    }

    #[test]
    fn parse_rejects_what_it_cant_read_correctly() {
        let error = |json: &str, supported: &[&str]| EmFile::parse(json, supported).unwrap_err();

        assert!(error("[]", features::ALL).contains("unversioned `.em` file"));
        assert!(error(r#"{"requires": [], "module": "m", "exports": {}, "entries": []}"#, features::ALL)
            .contains("no `version`"));
        assert!(error(r#"{"version": 2, "requires": [], "module": "m", "exports": {}, "entries": []}"#, features::ALL)
            .contains("version 2 isn't supported"));
        assert!(error(r#"{"version": 1, "requires": ["sections"], "module": "m", "exports": {}, "entries": []}"#, &[])
            .contains("requires `sections`"));
        assert!(error(r#"{"version": 1, "requires": ["teleport"], "module": "m", "exports": {}, "entries": []}"#, features::ALL)
            .contains("requires `teleport`"));
    }

    #[test]
    fn reifies_an_int() {
        let symbols =
            collect_symbols(&Program { statements: vec![], span: Span::new(0, 0), doc: None, stray_docs: vec![] }, &[]).unwrap();

        assert_eq!(
            reify_value(&TypeIds::new(&symbols, &[]), &Value::Int(Int::from(42))),
            EmittedValue::Int { value: "42".to_string() },
        );
    }

    #[test]
    fn reifies_a_struct_with_resolved_generic_args_by_id() {
        let program = parse_fixture("generic_alias.basm");

        let declaration = find_macro(&program, "make_byte");
        let symbols = collect_symbols(&program, &vec![0; program.statements.len()]).unwrap();
        let consts = HashMap::new();
        let mut resolver = AliasResolver::new_single_pass(&program, &symbols, &consts);

        let mut stack = Vec::new();
        let expansion = resolver
            .run_macro_body(
                symbols.lookup("make_byte").unwrap(),
                declaration,
                vec![Value::Int(Int::from(3))],
                &mut stack,
            )
            .unwrap();

        let module_paths = ["fixture".to_string()];
        let emitted = reify_value(&TypeIds::new(&symbols, &module_paths), &expansion.emitted[0]);

        assert_eq!(
            emitted,
            EmittedValue::Struct {
                id: "fixture.bits".to_string(),
                args: vec![EmittedGenericArg::Const { value: "8".to_string() }],
                fields: vec![("value".to_string(), EmittedValue::Int { value: "3".to_string() })],
            }
        );
    }
}
