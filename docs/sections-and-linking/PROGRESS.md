# Sections + linking: design and implementation plan

**Read this file before touching anything related to sections, `pub` on
labels, cross-file symbol references, or multi-file `bitter build`/`bitter
exec`.** It is the source of truth for what's decided, what's next, and
which design questions were already argued through — not any prior chat
conversation. This feature spans the core language (`src/`), `bitter`
(`bitter/src/`), and touches every ISA package indirectly (none of them
need their own changes for this — see "Established facts" below), so it
doesn't belong inside any one `std/<isa>/` directory the way
`std/x86_64/PROGRESS.md` did.

This whole plan came out of a long design conversation that started from
one small, concrete complaint: `examples/x86_64/hello.basm` has to smuggle
its string literal through register immediates because `bitter`'s packer
only ever produces one flat, undifferentiated stream of bytes, with no way
to mark part of it as "not code" or to split a program across multiple
files. Everything below is the result of repeatedly asking "is this
actually the most agnostic way to solve that," and several designs were
tried and rejected along the way — the rejected ones are recorded
explicitly (see "Rejected designs") specifically so a fresh implementer
doesn't re-derive and re-reject them a second time.

## Status

- [x] Phase 0 — This document
- [x] Phase 1 — `section` statement (parser/AST only)
- [x] Phase 2 — Section tagging in resolution + `.em` output
- [x] Phase 3 — Section-scope escape-hatch facet
- [x] Phase 4 — `pub` on labels
- [x] Phase 5 — Cross-unit label references (`from file import label`)
- [ ] Phase 6 — `bitter build`/`bitter exec` multi-file merge + link + wrap

Work through phases in order — each one is a real, separately verifiable
increment, and later phases assume earlier ones are done. Phase 0 is this
document itself, already done by virtue of existing.

## Decided scope (do not revisit without a real reason)

- **`section` is a new top-level statement kind**, syntactically parallel
  to `label` (`Statement::Label` in `src/ast.rs:82` is the existing
  precedent — `section` gets its own `Statement::Section` the same way).
- **A section is pure naming/grouping, nothing else.** No properties, no
  read/write/execute flags, no validation of what a section may contain.
  `bitterasm` and `bitter`'s core never interpret what a section name
  means — a program can put code in a section called `.data` and nothing
  stops it, the same way nothing stops a real NASM/GAS user from doing the
  same thing. Any meaning a section name carries (e.g. "ELF segment
  permissions for anything named `.rodata`") lives entirely in whichever
  *backend* chooses to key off that name later (e.g. the ELF writer in
  `bitter build`) — never in the language or in `bitter`'s shared core.
- **Sections are reopenable, not nested.** Declaring `section foo` a
  second time later in the same file (or, once Phase 6 lands, in a
  *different* file) appends to the same named group rather than starting a
  new one — this is exactly NASM/GAS's own `section .data` / `section
  .text` / `section .data` behavior. There is no explicit "end" — a
  section stays active until the next `section` statement (or the file
  ends). No block syntax, no braces.
- **No section declared at all = today's behavior, unchanged.** Everything
  lands in one implicit, unnamed section, byte-for-byte identical to
  current `.em` output. This is a hard backward-compatibility requirement,
  not a nice-to-have — verify it explicitly in Phase 2.
- **A macro's emitted values always land in whichever section is active at
  its *call site***, not wherever the macro was textually defined — this
  matches how position/program-order already works today (a macro's
  `@emit`s get spliced into the caller's position, so section membership
  following the same rule is the consistent choice, not an arbitrary one).
- **Section changes inside a macro body are automatically scoped to that
  macro's own call** (push the caller's current section on entry, restore
  it on return), *unless* the macro opts out via a facet (Phase 3). Without
  this, a macro that internally declares its own section would silently
  leak that change into the caller's subsequent code — a real, known
  footgun (NASM macros that switch sections without switching back
  exhibit exactly this bug in practice; disciplined NASM code works around
  it by hand with `%push`/`%pop` context directives). Save/restore around
  every macro call closes this by construction instead of relying on
  discipline.
- **`pub` on a label reuses `pub`'s existing single meaning** — "importable
  by another compilation unit" — the same sense it already has on
  `struct`/`enum`/`type`/`const`/`macro`. It is *not* a new, entry-point-
  specific concept. `Label` currently has no visibility field at all
  (`src/ast.rs:82` — just `name` and `span`), so this is net-new, not an
  overload of something that already meant something else for labels.
- **The eventual "entry point" falls out of this for free** — once real
  linking exists, the entry point is just whichever `pub` symbol the final
  build step is told (or defaults) to start at, the same way a real
  linker's `ENTRY()`/`-e` names an ordinary global symbol. No separate
  entry-point mechanism needs building.
- **Cross-unit label references reuse `from file import label_name`
  unchanged** — no new keyword, no `@linked`-style marker. The compiler
  already reads an imported file's source to validate any import
  (`src/loader.rs`'s `collect_declarations`, per prior investigation,
  splices a module's full transitive declaration set in for
  macros/consts/etc.) — for a *label* specifically, instead of splicing,
  it records a deferred cross-unit reference. The importing file gets the
  same immediate, compile-time "this name doesn't exist over there" error
  it already gets for a bad macro/const import; only the label's numeric
  *value* waits for link time. This requires naming the specific file, the
  same as every other import in this language — deliberately not the same
  as classic C `extern int foo;`, where the linker hunts across whatever
  object files happen to be provided (see "Deferred, not rejected" below).
- **`.em` gains exactly two new capabilities, no new file format:**
  1. A new `Deferred` leaf meaning "the value of external symbol `X`, not
     yet known, resolve at link time" (alongside the existing `Here`,
     `Leaf`, `Node`/`BinOp` variants — see `bitter/src/pack.rs` around
     lines 220-230 and 533-628 for the existing `Deferred` shape).
  2. Each `.em` entry carries which section it belongs to.
- **`bitter encode`'s contract does not change.** It still means exactly
  what it means today: given a fully-resolvable `.em`, produce fully-
  resolved bytes, no exceptions, no partial output. `.bin` stays "fully
  resolved, zero symbolic content" — no relocation table gets bolted onto
  it. This was a deliberate choice over the alternative (see "Rejected
  designs") specifically because it keeps `encode`'s job exactly as crisp
  as it already is, and because it's the same resolve-everything-in-one-
  pass strategy `Deferred`/`Positioned<N>`/`here()`/`span()` already use
  within a single file today — cross-unit references are the same
  strategy, just widened in scope, not a second strategy bolted on.
- **No separate `bitter link` command.** `bitter build`/`bitter exec` gain
  the ability to accept *multiple* `.em` files as input and do the merge-
  and-resolve step internally before the OS-executable-wrapping work they
  already do — this matches the precedent they already set (their own
  doc comment already describes them as bundling `bitterasm compile` +
  `bitter encode` + `bitter exec` into one command; accepting N inputs
  instead of one and doing the linking step internally is the same move,
  not a new pattern).
- **`-f`/`--format` keeps working exactly as it does today** (explicit
  override, else `Format::native()` auto-detects the host OS —
  `bitter/src/formats.rs`) once the input side accepts multiple files.
- **Real ELF `ET_REL`-compatible object files are explicitly out of scope
  for this phase.** `.em`/`.bin`, extended as above, are bitterasm's own
  formats — nothing outside this project reads them, same as today. Real
  object-file compatibility (so `objdump`/GNU `ld`/etc. could read
  bitterasm's output directly) is a much larger, separate effort — ELF,
  PE, and Mach-O don't even share one relocatable-object convention with
  each other, so it would mean three different formats, not one. Revisit
  only with a real reason to interoperate with an external toolchain.

## Rejected designs (do not re-propose without a new argument)

These were seriously considered and specifically rejected during the
design conversation this plan came from. If you find yourself reaching for
one of these, re-read why it didn't survive first.

- **A `data { }` block keyword in the core language.** Rejected because it
  bakes a taxonomy ("this is non-executable") directly into language
  syntax that not every target shares — WASM has no such concept at all
  (its linear memory is uniformly read/write, and its bytecode isn't
  addressable as data in the first place), so every WASM package would
  carry a concept that means nothing to it.
- **R/W/X permission bits as `bitter`'s own universal, core-level
  contract.** Rejected as something the *shared* core promises, though
  it's fine as an internal convention *within* a specific backend (the ELF
  writer already expresses permissions this way, and PE/Mach-O do too —
  `bitter/src/formats.rs`: ELF's `p_flags = PF_R | PF_X`, PE's
  `MEM_EXECUTE | MEM_READ`, Mach-O's `VM_PROT_READ | WRITE | EXECUTE`).
  The reason it can't be a *universal* assumption: real architectures
  exist where R/W/X doesn't apply as a model at all — a true Harvard
  architecture (separate code/data buses; executability isn't an
  independent flag on a uniform address space, it's a consequence of which
  physical memory something lives in) and capability-based systems like
  CHERI (permission is a property of the *reference* you hold, not the
  memory region itself — "what are this region's permissions" isn't even
  a well-formed question there).
- **`Region<P>` (a generic, programmable "properties" struct attached to a
  section) and `Tagged<T, R>` (a `LittleEndian`-style value wrapper
  carrying a region).** Rejected as over-engineered once it became clear
  sections don't need to carry any interpreted properties at all — they're
  pure names, full stop. Nothing programmable, nothing `bitter` recognizes
  by struct name the way it recognizes `LittleEndian`/`Positioned`.
- **`@region(value) { ... }` as a new block-scoped keyword.** Rejected in
  favor of the much lighter NASM-style `section` point-declaration
  (auto-closes at the next one, no braces, no nesting) — once regions
  stopped needing programmable properties, there was no reason left for
  block syntax at all.
- **A separate `bitter link` command producing its own intermediate
  artifact.** Rejected — folded into `bitter build`/`bitter exec` instead
  (see "Decided scope" above).
- **Extending `bitter encode` to produce a partially-resolved `.bin` plus a
  relocation-record sidecar** (i.e. doing what a real assembler's `.o`
  does — pack what you can now, patch the rest later). Rejected in favor
  of keeping everything symbolic in `.em`'s JSON form until one final
  single-pass resolution over the *merged* multi-file result. Two reasons:
  it's the same strategy `Deferred` already uses within one file (defer
  until you have full information, resolve once), rather than a second,
  new resolution strategy; and it keeps `bitter encode`'s contract
  exactly as crisp as it is today instead of making it "usually fully
  resolved, except when it isn't."
- **`@linked`-style import syntax, or any new keyword for cross-unit label
  imports.** Rejected — plain `from file import label_name` already does
  the job once the compiler treats a label import as deferred instead of
  spliced (see "Decided scope").

## Deferred, not rejected (documented so it isn't lost, not built now)

- **File-agnostic symbol resolution** — classic C `extern int foo;` style,
  where the referencing file names only the *symbol*, not which file
  provides it, and the linker hunts across whatever files it's actually
  given. This is a real, legitimate feature (useful for swappable
  implementations, weak symbols) but trades away the early compile-time
  "this name doesn't exist" error the file-naming approach gets for free,
  and needs its own deliberate design pass if it's ever wanted. Do not
  build this as part of Phase 5 — Phase 5 is file-naming only.
- **An `.org`-equivalent (fixed load address for a section, or a mid-
  stream padding-gap primitive independent of any section).** This was
  originally sketched as a section *property* ("give this section a base
  address"), but properties were dropped entirely from the design (see
  "Rejected designs" — `Region<P>`). Whether/how a fixed load address gets
  expressed once sections carry no properties at all is **not decided**
  and needs a fresh pass, not an assumption that the old sketch still
  applies. The narrower "insert N padding bytes at a specific point" need
  is already achievable by hand today (an explicit `Bytes<N>` of zeros) and
  doesn't need special directive support unless something concrete demands
  it.
- **Duplicate `pub` symbol definitions across linked files.** The natural
  choice, by analogy with every real linker's "multiple definition" error,
  is to make this a hard error in Phase 6 — but this was never explicitly
  confirmed in the design conversation, only assumed by analogy. Confirm
  it (or pick something else) before or during Phase 6, don't silently
  build the assumption in.

## Established facts (verified against source, don't re-derive)

- **Labels are already a uniform, ISA-agnostic core feature, and this plan
  does not change them.** A label resolves to a plain `int` — its entry
  position (`src/resolver/values.rs:1330`, `resolve_label_value`) — and
  each ISA package already consumes that position completely differently:
  - RISC-V: relative, fixed-multiplier — `mul(sub(target, here()), 4)`
    (`std/riscv/impl.basm:629`, `beq`), correct only because every RV32I
    instruction is exactly 4 bytes.
  - x86-64: relative, byte-accurate — `sub(span(here(), target),
    own_length)` (`rel32_offset` in `std/x86_64/impl.basm`), needed
    because x86 instructions vary in length; this is *why* `span()` had to
    be generalized to be bidirectional and `Deferred`-endpoint-capable.
  - WASM: doesn't use label position at all — `br`/`br_if` take a plain
    caller-supplied nesting *depth* (`std/wasm/impl.basm:75-113`);
    structured control flow, no address-based branching, no import of
    `std.bitter.deferred` at all.
  - PDP-10: absolute, direct — a label's resolved position is written
    straight into the address field with no `here()`/`span()` arithmetic
    at all (`std/pdp10/impl.basm:83-94`, `jrst`/`jumpa`), since PDP-10
    jumps encode an absolute word address.
  
  This diversity is intentional and this plan must not reduce it — nothing
  here should force an ISA package to consume label positions any
  differently than it already does.
- **`bitter`'s packer already has a working, proven pattern for teaching it
  new semantics without touching the language core: recognizing specific
  struct *names*.** `bits`, `Positioned`, `LittleEndian`, and `Deferred`
  are all special-cased by name in `bitter/src/pack.rs` (e.g. line 76:
  `name == "bits"`, line 158: `name == "LittleEndian"`) while every other
  struct is packed generically. This is the same mechanism `LittleEndian<T,
  width>` uses today (`std/bitter/byte_order.basm:15` — "`bitter`
  recognizes this struct by name exactly the way it already recognizes
  `bits` and `Positioned`"). It was considered as the vehicle for section
  properties (see "Rejected designs") and rejected for that specific use,
  but remains the right precedent for *other* new by-name recognition this
  plan might still need (e.g. however the new `Deferred` symbol-reference
  leaf gets represented).
- **The existing facet system is the right precedent for the section-scope
  escape hatch (Phase 3).** `| emits <T>`, `| before ...`, and `| syntax
  {...}` (see `src/facets/`) already establish "safe default behavior,
  explicit and visible opt-in to the exception, declared right on the
  macro" as this language's standard way of handling this shape of
  problem — the new facet should follow the same shape, not invent a
  different mechanism.
- **`bitter build`/`bitter exec`'s entry point is currently hardcoded, not
  computed from any label.** `entry = LOAD_ADDR + EHDR_SIZE + PHDR_SIZE`
  (`bitter/src/formats.rs:180`) — literally "the first byte of the code
  stream," always. There is no existing "find the entry label" mechanism
  to preserve or migrate; Phase 6 (or a later phase) introduces this from
  scratch once `pub` labels exist.
- **Imports currently work by splicing, not deferring.** "Importing any
  name from a module recursively splices that module's entire transitive
  declaration set into the flattened program" — a named import list is
  only a typo check today, not a visibility filter (this was the root
  cause of `std/x86_64/impl.basm`'s `shl_cl`/`shr_cl`/`sar_cl` naming-
  collision workaround, since anything importing `std.x86_64.impl`
  unavoidably also gets everything `std.bitter.deferred` declares). Phase
  5 needs a *second* import behavior specifically for labels (defer
  instead of splice) without disturbing this existing behavior for every
  other declaration kind.

## Phases

### Phase 0 — This document
Done by virtue of existing. Read it fully before starting Phase 1.

### Phase 1 — `section` statement (parser/AST only) — DONE
**Deliverable:** `section <name>` parses as a new top-level statement
(`Statement::Section` in `src/ast.rs`, parallel to `Statement::Label`).
No resolver/emission behavior yet — this phase is purely "the syntax
exists and parses," verified with parser-level tests the way
`src/parser/tests.rs` already tests other statement kinds.
**Open questions, as settled:**
- Grammar: `section` is a hard keyword (`TokenKind::Section`, lexed
  alongside `struct`/`enum`/`const`/`macro` in `src/lexer.rs`) rather than
  a contextually-disambiguated soft keyword like `syntax`'s
  `at_syntax_override_start` — there's no legitimate existing use of
  `section` as an identifier to preserve, and every other top-level
  declaration keyword already works this way. `<name>` is a dotted
  identifier: an optional leading `.` (NASM/ELF convention, e.g. `.text`,
  `.rodata`) followed by one or more identifier segments joined by `.`
  (e.g. `.rela.text`), stored verbatim as a single `String` including the
  dots (`ast::Section::name`). A bare name with no leading dot (`section
  data`) is also legal — bitterasm doesn't require the dot, it just
  doesn't strip it if present, so a backend keying off literal ELF-style
  names (`.rodata`) still works.
- Character restrictions: none beyond what an identifier token already
  enforces (each dot-separated segment is a normal identifier); no
  arbitrary-string section names (no quoting).
**Files:** `src/token.rs`/`src/lexer.rs` (new `Section` keyword),
`src/ast.rs` (`Statement::Section`, `ast::Section`), `src/parser/mod.rs`
+ `src/parser/statements.rs` (`parse_section`), `src/parser/tests.rs`.
Every other exhaustive `match` over `Statement`/`TokenKind` in the crate
(`src/printer.rs`, `src/expander.rs`, `src/loader.rs`,
`src/diagnostics/lint.rs`, `src/resolver/generated.rs`) got a
no-behavior-change arm for the new variant so the crate keeps compiling;
`src/resolver/macro_body.rs`'s `walk_macro_body` rejects a `section`
statement inside a macro body with `ResolveError::UnsupportedMacroStatement`
for now (same treatment as `import`) since Phase 2 is what actually
defines push/pop scoping there.
**Verification:** parser unit tests confirming `section foo`/`section
.text`/`section .rela.text`/reopened sections all round-trip through the
AST (`src/parser/tests.rs`). Manually confirmed a `.basm` file containing
only `section` statements (no other statements) compiles to `[]` emitted
values — zero resolver/emission behavior change, as required.

### Phase 2 — Section tagging in resolution + `.em` output — DONE
**Deliverable:** the resolver tracks "current section" as call-site-scoped
state — starts as the implicit default/unnamed section, changes on
encountering a `Statement::Section`, and every `@emit`-produced value in
`.em` output records which section was active when it was emitted. A
macro invocation inherits the caller's current section as its own starting
state; on return, the caller's own current section is restored to exactly
what it was before the call (push/pop discipline — see "Decided scope").
**Open questions, as settled:**
- State lives directly on `AliasResolver` (`src/resolver/aliases.rs`):
  `current_section: Option<String>` (the live state, `None` = the
  implicit default section) plus `emitted_sections: Vec<Option<String>>`
  (a whole-program-persistent parallel log, pushed once per `@emit`, kept
  index-aligned with `values_emitted` — the exact same "shared counter
  advanced once per `@emit`" idiom `values_emitted` itself already
  established, just a `Vec` instead of a count). `walk_macro_body`
  (`src/resolver/macro_body.rs`) mutates `current_section` on a
  `Statement::Section`; `run_macro_body_inner` saves/restores it around
  every macro call, in the same spot and the same way it already
  saves/restores `generic_scope`/`current_module`. Top-level sections are
  driven the same way, one level up, by `main::walk_top_level` calling the
  new `AliasResolver::set_current_section` on a top-level
  `Statement::Section` (no push/pop needed there — no caller to restore
  to).
- `.em` shape: a flat array of per-entry objects, each the same tagged
  object `EmittedValue` already produced, with one new field
  (`emit::EmittedEntry`, `src/emit.rs`) — `#[serde(flatten)]` on the value
  merges its tagged fields into the same JSON object, and
  `#[serde(skip_serializing_if = "Option::is_none")]` on `section` omits
  that key entirely when there's no section, which is what makes the "no
  `section` statement → byte-for-byte identical `.em`" bar achievable at
  all (verified below) rather than merely "deserializes to the same
  values." Flat (not grouped-by-section) specifically because Phase 6
  needs to concatenate same-named sections across multiple files in
  argument order — a flat array just needs filter-by-name-then-append; a
  pre-grouped shape would need an extra merge step for no benefit now.
  `bitter`'s existing `.em` reader (`bitter/src/main.rs`/`pack.rs`) needs
  no change for this phase: it deserializes into `Vec<EmittedValue>`
  directly, and serde silently ignores the extra `section` key on a
  struct without `#[serde(deny_unknown_fields)]` — real section-aware
  consumption is Phase 6's job (`bitter build`/`bitter exec`), not this
  one's.
**Files:** `src/resolver/aliases.rs` (new state + `set_current_section`/
`take_emitted_sections`), `src/resolver/macro_body.rs` (`Statement::Section`
handling, `@emit` tagging, push/pop around `run_macro_body_inner`),
`src/main.rs` (`Expansion` gained a `sections` field; `walk_top_level`
handles top-level `Statement::Section`; `compile` zips `emitted`/`sections`
into `Vec<emit::EmittedEntry>` before serializing), `src/emit.rs`
(`EmittedEntry`). New fixtures under `tests/fixtures/emit/`:
`sections_none.basm`, `sections_one.basm`, `sections_reopened.basm`,
`sections_macro_scoped.basm` (the macro-inheritance-and-restore case),
exercised by the new `tests/sections.rs` (same real-CLI-binary standard as
`tests/label_passes.rs`).
**Verification:** `tests/sections.rs`'s
`no_section_statement_produces_em_with_no_section_key_at_all` asserts the
raw `.em` text contains no `"section"` key at all for a zero-`section`
fixture — not just equal-after-deserializing. Additionally hand-verified
by diffing actual compiled `.em` bytes for a pre-existing fixture
(`tests/fixtures/emit/backward_label_only.basm`) between the pre-Phase-2
and post-Phase-2 compiler: identical. The other three new tests cover
reopening (same section name reused later is one group, confirmed by the
macro `mark` landing its `@emit` in whichever of `.text`/`.data` was
active at each of its three call sites) and the macro-call push/pop
restore (a macro that changes section internally doesn't leak that change
into the caller's code after it returns).

### Phase 3 — Section-scope escape-hatch facet — DONE
**Deliverable:** a new macro facet, `| leaks_section`, that opts a specific
macro out of the automatic push/pop restore from Phase 2, for the
deliberate case of a macro meant to behave like a bare `section` statement
itself (e.g. a convenience wrapper that's *supposed* to change what
section subsequent caller code lands in).
**Open questions, as settled:**
- Final facet name: `leaks_section` (the working name from the design
  conversation survived unchanged).
- Parameters: none — a bare marker (`FacetPayload::Bare`,
  `PayloadShape::Bare`). This is the first facet to actually use
  `PayloadShape::Bare` via the plain-identifier parse path — the type
  existed already (and `printer.rs`/`expander.rs`/`loader.rs` already had
  correct no-op-ish arms for it) but `src/parser/facets.rs`'s identifier
  path previously hit `unreachable!()` for it, having been written on the
  (incorrect, per this module's own doc) assumption that `Bare` was
  reserved for dedicated-token facets like a hypothetical `pub`/`return`
  facet, which don't actually exist as facets at all (`pub` and `-> Type`
  are declaration-signature fields, not facets — see `src/parser/
  facets.rs`'s module doc). That `unreachable!()` arm now does the real
  work: `PayloadShape::Bare => FacetPayload::Bare`.
**Files:** `src/facets/leaks_section.rs` (new facet module, following
`syntax.rs`'s macro-only/at-most-once cardinality shape rather than
`emits`'s repeatable one — leaking is a yes/no property of a macro, not a
set), `src/facets/mod.rs` (registration in `payload_shape`/`check`, plus a
new `facets::has` helper for a presence-only check — the other `extract_*`
helpers all assume a payload worth collecting, which `Bare` doesn't have),
`src/parser/facets.rs` (the `PayloadShape::Bare` arm), `src/resolver/
macro_body.rs` (`run_macro_body_inner` now checks `facets::has(&declaration
.facets, "leaks_section")` before restoring `current_section` on return),
new fixture `tests/fixtures/emit/sections_leaks_facet.basm` demonstrating
the opted-out (leaking) case side by side with the existing
`sections_macro_scoped.basm` (default, restored case).
**Verification:** `tests/sections.rs`'s existing
`a_macros_section_change_is_scoped_to_its_own_call_not_leaked_to_the_caller`
covers the default case (no facet: not leaked); the new
`a_macro_declared_leaks_section_leaves_its_section_change_active_after_return`
covers the opted-out case (facet present: leaked) — both assert final
section membership of code after the call, via the real `bitterasm
compile` CLI same as Phase 2. Also added a parser-level unit test
(`parser::tests::parses_bare_leaks_section_facet_on_a_macro`) confirming
`| leaks_section` round-trips to `FacetPayload::Bare` with no payload.
Full `cargo test` suite (290+ tests across `src/` and `tests/`) passes
with no regressions; `cargo clippy --all-targets` shows no new warnings
attributable to this phase's files.

### Phase 4 — `pub` on labels — DONE
**Deliverable:** `Label` (`src/ast.rs:83`) gains an `is_pub: bool` field,
parsed the same way `pub` already parses on `struct`/`macro`/`const`/etc.
(the five existing `is_pub` fields already in `src/ast.rs` are the
precedent followed exactly). This phase is self-contained — it does not
yet unlock cross-file references (that's Phase 5) — it only needed the
flag to exist and parse correctly, and (per "Decided scope") to mean the
*same* thing `pub` already means everywhere else, not a new concept.
**Open questions, as settled:**
- Grammar: `pub` precedes a label the same way it precedes every other
  declaration keyword (`pub start:`), even though a label itself has no
  leading keyword — `src/parser/statements.rs`'s `TokenKind::Pub` arm
  gained one more case (`TokenKind::Identifier(_) if self.check_next(&
  TokenKind::Colon)`) alongside its existing `Struct`/`Enum`/`Type`/
  `Const`/`Macro` cases, calling the same `parse_label` the non-`pub`
  path uses, now parameterized by `is_pub: bool`.
- Whether the symbol table itself needed a new field: **no** — every
  other kind's `is_pub` already lives only on the found AST declaration,
  never as a field on `resolver::symbols::Symbol` (`find_struct_
  declaration`/`find_alias_declaration`/etc. in `src/resolver/aliases.rs`
  fetch the full declaration by `SymbolId` and read `is_pub` off *that*).
  Labels follow the identical pattern once they need it — no
  `find_label_declaration` was added yet, since nothing in Phase 4 (or
  Phase 5, until it's actually built) calls it; adding it now would have
  been unused `pub(super)` API sitting idle, which this codebase doesn't
  otherwise carry. It's a same-shaped few-line addition to `src/resolver/
  aliases.rs` whenever Phase 5 needs it for real.
**Files:** `src/ast.rs` (`Label::is_pub`), `src/parser/statements.rs`
(`parse_label` takes `is_pub`, `TokenKind::Pub`'s new label case),
`src/printer.rs` (prints `pub ` before a `pub` label, mirroring every
other kind's `pub_kw` handling). `src/resolver/mod.rs`/`generated.rs`
needed no change at all — both already register every `Statement::Label`
into the symbol table by cloning/reading through the whole `Label` value,
so the new field rides along automatically. New fixture `tests/fixtures/
emit/pub_label.basm` (a `pub` label and a non-`pub` label side by side,
otherwise identical to the existing `backward_label_only.basm`).
**Verification:** `src/parser/tests.rs`'s `parses_pub_label` (new, next to
the existing `parses_label`) confirms `pub start:` parses with `is_pub ==
true`, `parses_label` now also asserts the non-`pub` case is `false`.
`src/resolver/aliases.rs`'s `pub_label_is_distinguishable_from_a_non_pub_
one_via_the_symbol_table` confirms both `loop_start`/`private_marker`
register into `collect_symbols`'s table and their `is_pub` values differ,
read off the found AST node the same way every other kind's tests would.
`tests/label_passes.rs`'s `a_pub_label_resolves_identically_to_a_non_pub_
one` confirms the hard "no behavior change yet" bar through the real CLI
— `pub_label.basm` emits the exact same sequence as `backward_label_only.
basm`. Full `cargo test` (292+ lib tests, all integration suites) passes;
`cargo clippy --all-targets` shows no new warnings from this phase's
files.

### Phase 5 — Cross-unit label references — DONE
**Deliverable:** `from file import label_name` resolves correctly when
`label_name` is a `pub` label in `file` — instead of splicing (today's
behavior for every other declaration kind), the resolver records a
deferred cross-unit reference. The importing file still gets an immediate
compile-time error if `label_name` isn't declared `pub` in `file` — the
compiler reads `file`'s source to check this the same way it already does
for a normal import, it just doesn't need `file`'s labels to have their
final numeric positions yet, only to know they exist and are `pub`.
**Open questions, as settled:**
- **Exact shape of the new "Deferred" leaf:** *not* a new variant of the
  user-space `std.bitter.deferred.Deferred` enum (that enum, and the
  `Here`/`Leaf`/`Node` vocabulary the original phase text pointed at, is
  entirely library code `bitter`'s packer recognizes by name — the
  resolver itself has zero built-in knowledge of it, and coupling a
  cross-unit label's representation to whether a program happens to
  import `std.bitter.deferred` would break the "no ISA package needs its
  own changes" bar (an architecture like WASM, which per "Established
  facts" never imports `std.bitter.deferred` at all, still needs to be
  able to import a label from another file). Instead: a genuinely new,
  core-level `EmittedValue::Deferred { file: String, symbol: String }`
  variant (`src/emit.rs`), sibling to `Int`/`Struct`/`Enum`, with a
  matching resolver-level `Value::ExternLabel { file, name }`
  (`src/resolver/values.rs`) that reifies to it. `value_type()` reports
  `Value::ExternLabel` as plain `ResolvedType::Builtin(BuiltinType::Int)`
  — the *same* type an ordinary, locally-resolvable label reference
  already has — which is what actually delivers "no ISA package needs its
  own changes": every existing `target: int`-shaped branch macro (RISC-V,
  x86-64, ...) accepts a cross-unit label with no changes of its own,
  the same way it already accepts a same-file one. It still composes with
  `std.bitter.deferred`'s own arithmetic where a program chooses to use
  it: `sub(imported_label, here())` picks `sub`'s `(int, Deferred) ->
  Deferred` overload and constructs `Deferred.Leaf(imported_label)`
  exactly as it would for a same-file `int`; `imported_label`'s own
  `Value::ExternLabel` just rides along as that variant's payload, reified
  through `Deferred.Leaf` to `EmittedValue::Deferred` at emission —
  `bitter/src/pack.rs` was taught to recognize that shape in the one place
  a real `Deferred.Leaf` payload is parsed (see "Files" below), alongside
  the two other places an `int`-typed value could receive one directly
  (bare `@emit`, and a `bits<N>{value: ...}` field) — all three give the
  same clear "link required" error rather than a wrong byte or a panic.
  `bitter encode`'s contract is unchanged by construction: a `.em`
  containing one is not fully resolvable, so refusing it *is* honoring
  the contract, not an exception to it.
- **How the resolver distinguishes "this import should splice" from "this
  import should defer":** at the *loader* level, before symbol collection
  ever runs — not a `SymbolKind` branch inside symbol collection itself.
  A new `ast::ExternLabel { name, file, span }` / `Statement::ExternLabel`
  (parallel to `ast::Label`/`Statement::Label`, but never real parseable
  source syntax — only ever synthesized by `loader::splice_import`) is
  what a requested import name that turns out to be a `pub` label
  produces, in place of the ordinary whole-declaration splice every other
  kind gets. `collect_symbols` registers it under its own new
  `SymbolKind::ExternLabel` (distinct from `SymbolKind::Label`), so
  identifier lookup (`resolver::values`) can tell a deferred reference
  apart from a locally-resolvable one purely by symbol kind, matching the
  open question's own guess — it just didn't need touching
  `collect_declarations`'s existing whole-module-splice loop at all (which
  already ignored `Statement::Label` outright and still does): the
  distinction is made once, in `splice_import`'s own per-requested-name
  validation loop, not woven through the transitive-splice machinery.
  Scoped deliberately narrow: only a plain `from file import label_name`
  triggers this — `import *` and package-style imports don't currently
  expose labels at all (same as before this phase; no regression, just
  not built, since Phase 5's own text only ever describes the named-import
  form).
**Files:** `src/ast.rs` (`ExternLabel`, `Statement::ExternLabel`),
`src/loader.rs` (`splice_import`'s new pub-label branch in its per-name
validation loop; every other exhaustive `Statement` match in the crate —
`printer.rs`, `expander.rs`, `diagnostics/lint.rs`, `resolver/generated.rs`,
`resolver/macro_body.rs` — got a no-behavior-change or clear-rejection arm
so the crate keeps compiling, the same treatment Phase 1's `section`
statement got), `src/resolver/symbols.rs` (`SymbolKind::ExternLabel`),
`src/resolver/mod.rs` (`collect_symbols`), `src/resolver/values.rs`
(`Value::ExternLabel`, `resolve_extern_label_value`, `value_type`),
`src/resolver/aliases.rs` (`find_extern_label_declaration`, following
`find_struct_declaration`'s exact shape), `src/emit.rs`
(`EmittedValue::Deferred`, `reify_value`), `bitter/src/pack.rs`
(`EmittedValue::Deferred` handled in `structural_width_bits`, `pack_value`'s
bare/`bits<N>`-field cases, and `resolve_deferred`'s `Leaf` case — all via
one shared `unresolved_deferred_error` message). New fixtures:
`tests/fixtures/emit/extern_label_dep.basm` (a `pub` label and a private
one), `extern_label_importer.basm` (imports and uses the `pub` one),
`extern_label_private_import.basm`/`extern_label_typo_import.basm` (the
two rejection cases).
**Verification:** `tests/extern_labels.rs`'s
`importing_a_pub_label_compiles_to_an_em_with_a_visibly_unresolved_deferred_entry`
compiles the importer fixture through the real `bitterasm compile` CLI and
asserts the resulting `.em`'s one entry is `EmittedValue::Deferred { file,
symbol: "target" }` — confirmed by hand too:
`bitter encode` on that same `.em` fails with "can't encode standalone:
`target` from `.../extern_label_dep.basm` is an unresolved cross-file
reference ... link it with `bitter build`/`bitter exec` instead" (exit 1,
no panic), not a wrong byte. `importing_a_non_pub_label_fails_immediately_
with_a_clear_error`/`importing_a_name_that_doesnt_exist_at_all_fails_the_
same_way_a_typod_macro_import_would` cover the two rejection cases, both
`UnknownImportedName` — the same error path (and message shape) an
unimported private/nonexistent macro or const already gets. Three new
`bitter/src/pack.rs` unit tests (`rejects_a_bare_unresolved_extern_label`,
`rejects_an_unresolved_extern_label_as_a_bits_n_value`,
`rejects_an_unresolved_extern_label_wrapped_in_deferred_leaf`) cover all
three shapes an unresolved reference can reach `bitter encode` in. Full
workspace `cargo test` (27 `bitter` + 292 `bitterasm` lib tests + every
integration suite) passes; `cargo clippy --workspace --all-targets`
introduces no new warning shapes beyond one `collapsible_if` that matches
its own immediate neighbors' (`find_struct_declaration`/
`find_alias_declaration`) pre-existing style exactly.

### Phase 6 — `bitter build`/`bitter exec` multi-file merge + link + wrap
**Deliverable:** `bitter build`/`bitter exec` accept multiple `.em` files
as input. They: concatenate same-named sections across all inputs (order
= command-line argument order — confirmed in "Decided scope"), build one
combined symbol table from every `pub` label across every input, run a
single resolution pass over the merged, still-symbolic result (reusing
`bitter encode`'s existing resolve-everything logic — its contract does
not change, see "Decided scope"), and then perform the exact same OS-
executable-wrapping work (`bitter/src/formats.rs`) they already do today.
`-f`/`--format` continues to work unchanged on this multi-input path.
**Open questions to settle here:**
- Duplicate `pub` symbol across two input files — confirm this is a hard
  error before building it (see "Deferred, not rejected").
- Exact CLI shape for accepting multiple input files (repeated positional
  args, a flag, etc. — not decided).
- How the entry point gets chosen now that `pub` labels exist — a
  specific well-known name by convention (`_start`?), a required CLI flag,
  or something else. "Decided scope" establishes *that* it should be "just
  a `pub` symbol, same as a real linker's `ENTRY()`" but not which symbol
  or how it's specified.
**Files:** `bitter/src/main.rs`, `bitter/src/pack.rs` (section merging,
symbol table construction, resolution over merged input), new integration
tests with multiple `.basm` files compiled separately and linked together,
run through the same real-execution verification `examples/x86_64/
hello.basm` already gets (actually build and run the resulting
executable, not just check its bytes).
**Verification:** two-file program (e.g. one file with `_start` calling a
`pub` function defined in a second file) compiles as two separate `.em`
files, links via `bitter build` with both as input, produces a working
native executable, actually runs and produces correct output — mirroring
how `examples/x86_64/hello.basm` was verified by actually executing it,
not just inspecting bytes.
