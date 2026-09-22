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
- [ ] Phase 3 — Section-scope escape-hatch facet
- [ ] Phase 4 — `pub` on labels
- [ ] Phase 5 — Cross-unit label references (`from file import label`)
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

### Phase 3 — Section-scope escape-hatch facet
**Deliverable:** a new macro facet (working name `| leaks_section`, not
finalized — see `src/facets/` for the existing pattern to follow) that
opts a specific macro out of the automatic push/pop restore from Phase 2,
for the deliberate case of a macro meant to behave like a bare `section`
statement itself (e.g. a convenience wrapper that's *supposed* to change
what section subsequent caller code lands in).
**Open questions to settle here:**
- Final facet name.
- Does the facet need any parameters, or is it a bare marker?
**Files:** `src/facets/` (new facet module, following `emits.rs`'s shape),
`src/parser/facets.rs`, a fixture demonstrating both default (restored)
and opted-out (leaking) behavior side by side.
**Verification:** a macro without the facet cannot change its caller's
subsequent section; a macro with the facet can, on purpose, confirmed via
a fixture with assertions on final section membership of code after the
call in both cases.

### Phase 4 — `pub` on labels
**Deliverable:** `Label` (`src/ast.rs:82`) gains an `is_pub: bool` field,
parsed the same way `pub` already parses on `struct`/`macro`/`const`/etc.
(the five existing `is_pub` fields already in `src/ast.rs`, e.g. lines
368/380/396/406/416, are the precedent to follow exactly). This phase is
self-contained — it does not yet unlock cross-file references (that's
Phase 5) — it only needs the flag to exist and parse correctly, and (per
"Decided scope") to mean the *same* thing `pub` already means everywhere
else, not a new concept.
**Files:** `src/ast.rs`, parser, `src/resolver/mod.rs`/`generated.rs`
(wherever `SymbolKind::Label` gets registered) to carry the flag through
to the symbol table.
**Verification:** parser/resolver tests confirming a `pub` label parses
and is distinguishable from a non-`pub` one in the symbol table; no
behavior change yet for anything that consumes labels within one file.

### Phase 5 — Cross-unit label references
**Deliverable:** `from file import label_name` resolves correctly when
`label_name` is a `pub` label in `file` — instead of splicing (today's
behavior for every other declaration kind), the resolver records a
deferred cross-unit reference: a new `Deferred` leaf (see "Established
facts" — same by-name-recognition precedent as `LittleEndian`/
`Positioned`) meaning "value of symbol `label_name` from `file`, not yet
known." The importing file still gets an immediate compile-time error if
`label_name` isn't declared `pub` in `file` — the compiler reads `file`'s
source to check this the same way it already does for a normal import,
it just doesn't need `file`'s labels to have their final numeric
positions yet, only to know they exist and are `pub`.
**Open questions to settle here:**
- Exact shape of the new `Deferred` leaf and how it's represented in
  `.em`'s JSON (needs to carry both the symbol name and which file it
  came from, for Phase 6 to resolve against).
- How the resolver distinguishes "this import should splice" from "this
  import should defer" — presumably branching on whether the imported
  symbol's `SymbolKind` is `Label` vs. everything else, but confirm this
  doesn't interact badly with the existing whole-module-splice behavior
  described in "Established facts."
**Files:** `src/loader.rs` (`collect_declarations`), `src/resolver/`
(wherever imports currently resolve to spliced declarations), `bitter/src/
pack.rs` (new `Deferred` variant), new fixtures with two files, one
importing a `pub` label from the other.
**Verification:** compiling the importing file alone (without its
dependency's final byte layout known) succeeds and produces an `.em` with
a visibly-unresolved symbol reference; compiling with a typo'd or non-
`pub` label name fails immediately with a clear error, the same quality of
error an unimported macro/const reference gets today.

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
