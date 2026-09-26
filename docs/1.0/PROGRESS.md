# Road to 1.0: design and implementation plan

**Read this file before working on spliced-name reads, `@fold`, the
`std/string.basm` length fix, the `.em` v1 format, or moving executable
containers (ELF/PE/Mach-O) out of `bitter`.** It is the source of truth for
what's decided and what's next, not any prior chat conversation. It follows
the same shape as `docs/sections-and-linking/PROGRESS.md`: decided scope,
rejected designs, established facts, then phases.

These five pieces of work came out of one design conversation (2026-09-26)
about what bitterasm needs before a 1.0 release. They're in one file because
they share a sequencing: `@fold` is what fixes long strings, and the `.em`
v1 module-path identity is what in-language executable headers build on.

## Status

**Part A — spliced-name reads**
- [x] Phase A1 — `` acc_`i` `` in expression position

**Part B — `@fold`**
- [x] Phase B1 — Syntax: parser, AST, printer, formatter
- [x] Phase B2 — Macro bodies
- [ ] Phase B3 — Construct literals and struct bodies
- [ ] Phase B4 — Top level
- [ ] Phase B5 — Reference docs + `@next` lint

**Part C — long strings**
- [ ] Phase C1 — Rewrite `std/string.basm`'s walkers with `@fold`

**Part D — `.em` v1**
- [ ] Phase D1 — Module identity (`id` = module path + name)
- [ ] Phase D2 — `Deferred { module, symbol }`
- [ ] Phase D3 — `exports` replaces `--labels`
- [ ] Phase D4 — Versioned header; `bitter` matches on `id`

**Part E — executable containers in bitterasm**
- [ ] Phase E1 — `Deferred` `add`
- [ ] Phase E2 — Linker-provided image symbols
- [ ] Phase E3 — `Align<N>` packer primitive
- [ ] Phase E4 — `std/formats/elf.basm`
- [ ] Phase E5 — `std/formats/pe.basm` and `std/formats/macho.basm`
- [ ] Phase E6 — Remove `formats.rs`, `bitter exec`, `--format`, `--entry`

Work through parts in order (A → E). Within a part, phases are in order.
Parts A–C are language work and don't depend on D–E. D must precede E.

## Decided scope (do not revisit without a real reason)

### Spliced-name reads
- **`` name`expr` `` works in expression position**, reading whatever the
  spliced name resolves to (a scope binding, then a top-level symbol),
  exactly as a plain identifier would. Declarations (`` const r`i` ``),
  fields (`` __el`i`: ``) and member access (`` x.__el`i` ``) already splice;
  this closes the one missing position.
- **Adjacency is required.** A splice only continues a name when the
  backtick touches the identifier (`` acc_`i` ``). With whitespace between
  (`` acc `i` ``) it stays two separate tokens, as today.
- **This does not make a `@for` running total work**, and isn't meant to:
  a `const` declared inside a `@for` body is scoped to that one iteration
  (`walk_macro_body`'s per-iteration `iter_scope`), so
  `` const acc_`i + 1` = acc_`i` + i `` can't see the previous iteration's
  constant. Accumulation is `@fold`'s job.

### `@fold`
- **Syntax:** `@fold` is a prefix on an ordinary `@for`:
  ```
  const table_size = @fold offset = 0 @for s in strings {
      @emit offset
      @next offset + s.len
  }
  ```
  `@fold <acc> = <init> [, <acc> = <init>]* @for <var> in <source> { ... }`.
  The `@for` part is unchanged: same sources (ranges, struct/array values,
  string literals), same iteration order.
- **Multiple accumulators** are allowed: `@fold offset = 0, count = 0 @for ...`.
- **`@next` supplies the next iteration's accumulator values and ends the
  current iteration** (like `continue`, carrying values). Two forms:
  - `@next <expr>`: positional, only when there is exactly one accumulator.
  - `@next offset = <expr>, count = <expr>`: named, any number. An
    accumulator not named keeps its current value.
- **No `@next` on a path means every accumulator stays unchanged** for that
  iteration. So filtering needs no `@else`:
  `@if s.len > 0 { @next offset + s.len }`. The risk (a forgotten `@next`
  silently doing nothing) is covered by a lint (Phase B5), not by making
  `@next` mandatory.
- **Inside the body, accumulators and the loop variable are ordinary
  immutable bindings** holding this iteration's values. Nothing is mutated:
  each iteration binds fresh values, so this stays inside const-only
  semantics.
- **Result value:** one accumulator gives its final value. Several give a
  synthesized struct with one `pub` field per accumulator, in declaration
  order (`const r = @fold a = 0, b = 0 @for ... { ... }`, then `r.a`,
  `r.b`), the same way `start..end` already synthesizes a `__range#N`
  struct (`resolver::generated::eval_range_value`). No tuple type is added.
- **`@fold` is both an expression and a statement.** As a statement the
  result is discarded, which is the point when the body only `@emit`s.
- **Allowed everywhere `@for` is:** macro bodies, construct literals
  (`Array<...> { @fold ... }`), struct declaration bodies, and top level.
  In construct literals and struct bodies the body produces fields/items
  instead of `@emit`s, and the result is always discarded.
- **A body that emits must have somewhere for its emits to go.** A fold
  whose body `@emit`s (directly or through an invoked macro) is allowed as
  a statement, as a `const`'s whole value, or as `@return`'s whole value.
  Anywhere else in an expression (e.g. a call argument) it's an error.
  Expression positions can't carry emits today (see "Established facts" on
  expression-position macro calls), and `@fold` shouldn't add a second
  silent-discard path.
- **`@return` inside a fold body returns from the enclosing macro**, the
  same as inside a plain `@for`.
- **`@next` belongs to the innermost enclosing `@fold`,** and must not sit
  inside a plain `@for` nested in that fold's body (error). That rules out
  "`@next` also breaks out of an inner loop", which would amount to
  labelled `continue`. It can be relaxed later if something needs it.
- **Bounded by the source.** `@fold` iterates a finite range/struct, so it
  needs no iteration cap beyond `@for`'s existing `MAX_FOR_ITERATIONS`
  (1,000,000). That's the advantage it has over tail recursion, which is
  capped at 4,096 iterations (`macro_body.rs`, `MAX_TAIL_CALLS`).
- **Top level** runs in the pre-resolution unroll pass
  (`resolver::toplevel`), so it has the same limits top-level `@for`
  already has: the source must be a literal `start..end` range, and every
  `init`/`@next` expression must evaluate with `eval::eval` from earlier
  top-level consts, the loop variable and the accumulators (ints only).
  `const x = @fold ...` at top level unrolls to `const x = <literal>`.

### `.em` v1
- **Top-level shape:**
  ```json
  {
    "version": 1,
    "requires": ["sections", "extern-labels"],
    "module": "examples.x86_64.hello",
    "exports": { "_start": 3 },
    "entries": [ ... ]
  }
  ```
- **`version`** (not `em`) is a whole number that goes up only on an
  incompatible change to the file's structure. v1 is the first version.
- **`requires`** lists the *language* features this program actually uses.
  Initially `sections` (some entry has a section) and `extern-labels` (some
  value is a `Deferred`). A reader must reject any feature it doesn't know,
  naming it. A program that uses neither writes `[]`.
- **Plain-list (pre-v1) `.em` files are rejected**, not read as "version 0".
- **Every struct, enum and type in `.em` carries an `id` = module path +
  name**, e.g. `std.binary.bits`, replacing `name`, including inside
  generic type arguments. No hash suffix.
- **Module path rule:** the file's path relative to the first search root
  that contains it (`loader::search_roots`), dots for separators, no
  extension. If no root contains it, relative to the entry file's
  directory, with leading dots for each step above it (`..shared.util`),
  the same syntax relative imports use. A declaration generated by a macro
  belongs to the module where the macro was *called*, since that's where
  generated declarations already land.
- **Private declarations use the same rule.** The loader's `Foo#3`
  load-order mangling stays internal and never reaches `.em`: a private
  `Foo` in `lib.util` is `lib.util.Foo`. A module can't declare the same
  name twice, so this is still unique within a program.
- **`Deferred` stores `{ "module": ..., "symbol": ... }`** instead of an
  absolute file path. Output no longer depends on where the checkout lives.
- **`exports`** records each top-level `pub` label's position, replacing
  the `bitterasm compile --labels` side file and flag.
- **`module`** records the entry file's own module path, so `bitter build`
  can match `Deferred.module` against its inputs without recomputing paths.
- **Which `id`s an evaluator understands is that evaluator's business**,
  not part of `.em`'s version. `bitter` implements every type `std/bitter/`
  and `std/binary.basm` define because it's the only reference evaluator,
  but another evaluator may support a subset. Evaluator-specific types
  (e.g. `std.bitter.layout.Align`) therefore don't appear in `requires`.

### Executable containers in bitterasm
- **ELF, PE and Mach-O headers become ordinary bitterasm code** in
  `std/formats/`, and `bitter` stops knowing about any executable format.
  This supersedes two points in `docs/sections-and-linking/PROGRESS.md`'s
  "Decided scope": `--format` keeping host auto-detection, and permissions
  living in `bitter`'s ELF writer.
- **A program chooses its container explicitly**, e.g.
  `from std.formats.elf import *` and one header invocation. No host
  auto-detection: the program's system calls already tie it to one OS.
- **The machine type comes from the program or the ISA package**, never a
  hardcoded constant. (`wrap_elf` hardcodes `EM_X86_64` today, so a RISC-V
  program would be labelled as x86-64.)
- **The header must land at byte 0.** That works by the existing documented
  rule: sections in first-appearance order, inputs in command-line order.
  The file that invokes the header goes first. No new ordering mechanism.
- **With no container imported, `bitter build` writes a flat binary** (like
  `nasm -f bin`), marked executable. `bitter build` always sets `+x`.
- **Scope is static executable images only.** Relocatable objects (`.o`),
  dynamic linking, symbol and relocation tables are out of scope for 1.0
  (see "Deferred, not rejected").

## Rejected designs (do not re-propose without a new argument)

- **Spliced names as the accumulation mechanism** (`` acc_`i` `` chains).
  It would need `@for` body constants to outlive their iteration, and
  leaves N numbered constants lying around. `@fold` is the mechanism;
  spliced reads exist for their other uses (generated register/constant
  names).
- **Raising `MAX_MACRO_CALL_DEPTH` (32) to fix long strings.** It only moves
  the failure point, and deep resolver recursion risks overflowing the
  compiler's own native stack.
- **Mutable compile-time variables** (Zig `comptime var`, C++ `constexpr`
  mutation). Too large a departure from const-only semantics when `@fold`
  covers the need.
- **A mandatory `@next` on every path**, requiring `@else { @next acc }`
  for filters. Rejected for noise; replaced by the unchanged-by-default rule
  plus a lint.
- **A single-accumulator-only `@fold` with structs for more.** Rejected in
  favor of real multiple accumulators with named `@next`.
- **A hash (content or build) in type ids** (Rust crate-hash, Unison
  style). It would stop evaluators from matching a fixed string like
  `std.binary.bits`. Module path + name is unique within one build, which is
  the scope `.em` files are merged in.
- **Tying `.em`'s version to the compiler's release version.** It would
  force every evaluator to track bitterasm's releases.
- **Keeping ELF/PE/Mach-O in `bitter` behind a `--machine` flag** to fix the
  hardcoded machine type. Fixes the symptom; the container would still be
  privileged, built-in knowledge.

## Deferred, not rejected

- **Relocatable object output and dynamic linking.** Needs a much richer
  way for bitterasm code to query linker state (iterate sections and
  symbols, emit relocation records). Revisit only with a real reason to
  interoperate with an external linker.
- **Per-section start/end symbols from the linker**, needed for separate
  ELF segments with different permissions (today everything is one
  read+execute segment, so `.data` isn't writable). Image start/end
  (Phase E2) is enough to replace `formats.rs`.
- **`@next` from inside a nested plain `@for`** (labelled-continue
  semantics). See "Decided scope".
- **Raising or configuring `MAX_TAIL_CALLS` (4,096).** Less pressing once
  `@fold` exists.

## Established facts (verified against source, don't re-derive)

- **Direct self tail calls are already eliminated.** `@return f(...)` where
  `f` is the current macro reuses the frame (`macro_body.rs`,
  `eval_direct_self_tail_call`, `pending_tail_call`), capped at 4,096
  iterations. Mutual tail recursion is not eliminated and still counts
  toward the 32-level `MAX_MACRO_CALL_DEPTH`. Verified: a tail-recursive sum
  runs at depth 1,000 and fails at 100,000 with "tail call ... exceeded
  4096 iterations".
- **Why long strings fail:** `std/string.basm`'s `utf8_struct_byte_len_at`
  returns `head + tail` (not a tail call), so it uses one depth level per
  character and hits the 32 limit at about 30 characters.
  `utf8_struct_value_at` has the same shape and also recomputes the rest of
  the string's length at every step (quadratic). `validate_ascii_at` is
  already a tail call.
- **Constants in a `@for` body are per-iteration** (`walk_macro_body`,
  `iter_scope`); `pub const`s in a macro body are instead registered
  globally as generated declarations.
- **Expression-position macro calls used to keep the return value and
  drop the `@emit`s** (`values.rs`, `eval_macro_call` → `.returned`), while
  the dropped `@emit`s still advanced `values_emitted` and pushed onto
  `emitted_sections`. Verified in Phase B2: after `const v = side()` where
  `side` emits two values, a following label resolved to 3 with only one
  value before it. **Fixed in B2** by giving macro calls the same rule as
  `@fold` (see Phase B2's "As built").
- **`@for` has four implementations**: macro bodies (`macro_body.rs`,
  `"for"`), construct literals (`values.rs`, `ConstructItem::For`), struct
  bodies (`structs.rs`, `StructBodyItem::For`) and top level
  (`toplevel.rs`, syntactic unroll before resolution). `@fold` needs all
  four.
- **An in-language ELF header already works with no compiler or `bitter`
  changes** for a single file: `span(image_start, _start)` for `e_entry`
  (plus the load address via `sub(x, -LOAD_ADDR)`), and
  `span(image_start, image_end)` for `p_filesz`, each wrapped in
  `LittleEndian<Positioned<64>, 64>`. Built with `bitterasm compile` +
  `bitter encode` + `chmod +x`, it ran on Linux (2026-09-26).
- **`bitter` recognizes types by bare name** in `bitter/src/pack.rs`
  (`bits`, `Positioned`, `LittleEndian`, `Deferred`, `BinOp`, `Op`), so a
  user struct named `bits` in any module is packed as binary today.
- **Only PE needs layout-dependent padding.** It pads the code section to
  `FileAlignment` (0x200). ELF's padding is inside fixed-size headers, and
  Mach-O's `vmsize` rounding is a header value computable with `add` and a
  mask.
- **`bitterasm-lsp` matches no `Expr`/`Statement`/`EmittedValue` variants**,
  so AST changes here don't break it; it only needs to keep building.

## Phases

### Part A — spliced-name reads

#### Phase A1 — `` acc_`i` `` in expression position
**Deliverable:** an identifier immediately followed by a backtick splice
parses in expression position as a spliced identifier and evaluates like a
plain identifier after its name is resolved.
**Plan:** add `Expr::SplicedIdentifier { name: SplicedName, span }` rather
than changing `Expr::Identifier`'s field type (which every pass matches on).
Parse it where the expression parser reads an identifier, reusing
`parse_spliced_name`'s loop, gated on the backtick's span starting exactly
at the identifier's end. Evaluate via `resolve_spliced_name` and then the
same lookup `Expr::Identifier` uses. Top-level unroll and the expander must
substitute inside the splice. Printer and formatter print it back verbatim.
**Verification:** fixtures for a scope binding (`` x`i` `` inside a
`@for`), a generated `pub const r`i`` read back by spliced name, whitespace
(`` x `i` `` still an error), and an unknown spliced name giving the normal
unknown-name error.
**As built (DONE):**
- `Expr::SplicedIdentifier` (`src/ast.rs`). The parser forms it only when
  the backtick starts exactly where the identifier ends, and never while
  already inside a splice's contents (`Parser::in_splice`): there, an
  identifier followed by a backtick is the splice *closing* (`` r`i` ``).
- Evaluation (`values.rs`) resolves the name, then tries the loader's
  private spelling `name#<current module>` before the plain name. The
  loader renames private top-level names before resolution and can't
  rewrite a name it doesn't know yet, so without this a spliced read
  couldn't see its own module's private constants.
- Inside a generated declaration (`macro_body.rs`, `splice_expr`) the name
  is settled at generation time and becomes a plain identifier, since the
  macro's bindings won't exist where the declaration is resolved later.
- `eval::eval` (top-level consts, top-level `@for` bounds) resolves it
  against its int scope. `resolver::consts::referenced_identifiers` and the
  lint's name collector record the name when every splice is an integer
  literal (`ast::literal_spliced_name`), which is what top-level unrolling
  leaves, plus whatever the splices themselves reference.
- Tests: `tests/spliced_reads.rs` through the real CLI, fixtures in
  `tests/fixtures/spliced/`. New shared helper `tests/common/mod.rs`
  (compile a fixture, read its `.em` back) for tests added from here on.

### Part B — `@fold`

#### Phase B1 — Syntax: parser, AST, printer, formatter
**Deliverable:** `@fold ... @for ...` and `@next` parse in all four `@for`
positions and in expression position, and round-trip through the printer
and formatter. No semantics yet: resolving one is an "unsupported" error.
**Verification:** parser tests for one and several accumulators, both
`@next` forms, statement vs `const`/`@return` value positions, and each of
the four body kinds. `bitterasm format` is idempotent on them.
**As built (DONE):**
- Statement position: `MetaStatement { name: "fold", args: [var, source],
  body, bindings }`, i.e. `@for`'s shape plus a new `bindings` field
  (`ast::FoldBinding { name, value, span }`) holding the accumulators.
  `@next` is `MetaStatement { name: "next" }` with its positional value in
  `args` (zero or one) and named updates in `bindings`. Every other meta
  has empty `bindings`.
- Expression position: `Expr::Fold { fold: Box<MetaStatement> }`, parsed
  in `parse_prefix_expr` on `@` + `fold`.
- Construct and struct bodies: new `ConstructItem::Fold`/`Next` and
  `StructBodyItem::Fold`/`Next` variants rather than an accumulators field
  on `For`. That way every pass that handles `For` was forced by the
  compiler to decide what a fold means, instead of silently treating one
  as a plain `@for`.
- Syntax-only passes handle folds fully: loader renaming, expander
  substitution (the loop variable and accumulators shadow substitutions,
  like a `@for` variable), printer, lint name collection, const
  dependencies. The unreachable-code lint treats `@next` like `@return`.
  Resolver sites return `ResolveError::Fold` ("isn't supported ... yet")
  until B2–B4.
- Fixed while here: the printer printed a statement `@for` as
  `@for i, 0..3 { ... }` (its two args joined with a comma), which
  `bitterasm expand` output and couldn't be re-parsed. It now prints
  `@for i in 0..3`.
- The formatter needed no change (it only handles indentation and
  wrapping) and leaves fold source unchanged.
- Tests: `src/parser/tests.rs` (every form, plus a print → parse → print
  stability check).

#### Phase B2 — Macro bodies
**Deliverable:** full semantics in macro bodies: iteration, `@next` (both
forms, unchanged-by-default), result value (single and synthesized struct),
`@return` from inside, the emit-placement rule, and the `@next`-inside-
nested-`@for` error. Settle the expression-position emit question from
"Established facts".
**Verification:** emit fixtures for each rule, including a 100,000-element
fold (proves no depth or tail-call cap applies) and label positions after a
fold that emits.
**As built (DONE):**
- `src/resolver/fold.rs`: `run_fold` walks the source like `@for`, binding
  the loop variable and every accumulator in each iteration's scope.
  `@next` evaluates its values and parks them in
  `AliasResolver::pending_next`, then returns early from `walk_macro_body`
  like `@return` does; `@if`/`@match` propagate it upward, and `run_fold`
  applies it. `AliasResolver::next_target` (`None` / `Fold` /
  `ForInsideFold`) is saved and restored around every fold body, plain
  `@for` body and macro call, which is what makes `@next` belong to the
  innermost fold of its own macro body and gives the two `@next` errors.
- Several accumulators produce a synthesized `__fold#N<T0, T1, ...>` struct
  with one `pub` field per accumulator, each typed by its own generic
  parameter, so accumulators of any type (structs included) fit.
- **The emit rule is shared with macro calls** (a decision made during
  B2; the old behavior was the label-shifting bug in "Established facts").
  `AliasResolver::eval_value_keeping_emits` handles a `@fold` or a macro
  call that is a statement, a `const`'s whole value, or `@return`'s whole
  value: its `@emit`s and generated declarations are kept. Anywhere else
  in an expression, one that emits is an error
  (`reject_expression_emits`). So `const v = side()` now emits `side`'s
  values where it used to drop them; nothing in `std` or the tests relied
  on the old dropping.
- Generated declarations from a call or fold inside a larger expression
  are still dropped, as before; only emits are rejected there.
- Tests: `tests/fold.rs` + `tests/fixtures/fold/` (every rule and error,
  a 100,000-iteration fold, label positions after an emitting fold, a
  struct-valued accumulator, and the `const v = side()` regression).

#### Phase B3 — Construct literals and struct bodies
**Deliverable:** `@fold` inside `Type { ... }` literals and struct
declaration bodies, producing fields.
**Verification:** an offsets table built as an `Array` literal with
`@fold`; a struct declaration whose field names/count come from a fold.

#### Phase B4 — Top level
**Deliverable:** `@fold` at top level in the unroll pass, int-only, plus
`const x = @fold ...` unrolling to a literal.
**Verification:** a top-level fold generating `pub const`s and labels, and a
clear error for a non-range source or a non-int accumulator.

#### Phase B5 — Reference docs + `@next` lint
**Deliverable:** a `@fold` section in `docs/reference.md`; a lint warning
when a fold body contains no `@next` at all.

### Part C — long strings

#### Phase C1 — Rewrite `std/string.basm`'s walkers with `@fold`
**Deliverable:** `utf8_struct_byte_len`, `utf8_struct_value_at`'s
equivalent and `validate_ascii` use `@fold` (linear, no recursion).
**Verification:** a `db` of a 1,000-character string (and a multi-byte
UTF-8 one) compiles and encodes to the expected bytes; existing string
fixtures unchanged.

### Part D — `.em` v1

#### Phase D1 — Module identity
**Deliverable:** every loaded file has a module path (rule above); every
emitted struct/enum/type carries `id` instead of `name`.
**Verification:** fixture compiled from two different working directories
produces identical `.em`; a private struct's `id` has no `#N`.

#### Phase D2 — `Deferred { module, symbol }`
**Deliverable:** `ast::ExternLabel` and `EmittedValue::Deferred` store the
module path; `bitter`'s linker matches on it.
**Verification:** existing multi-file link tests pass; `.em` contains no
absolute paths.

#### Phase D3 — `exports` replaces `--labels`
**Deliverable:** `pub` label positions written into `.em`; `--labels`
removed; `bitter build` reads `exports`.

#### Phase D4 — Versioned header; `bitter` matches on `id`
**Deliverable:** the header object; `bitter` rejects plain-list files and
unknown `version`/`requires`, and recognizes types by `id`
(`std.binary.bits`, `std.bitter.deferred.Positioned`, ...).
**Verification:** a user struct named `bits` in another module is packed as
an ordinary struct; each rejection has a clear message; `requires` lists
exactly the features used.

### Part E — executable containers in bitterasm

#### Phase E1 — `Deferred` `add`
**Deliverable:** `add` overloads in `std/bitter/deferred.basm` and `Op.Add`
in the packer.

#### Phase E2 — Linker-provided image symbols
**Deliverable:** `std/bitter/link.basm` declaring `pub image_start:` and
`pub image_end:`; `bitter build` resolves `Deferred`s into that module to
the start and end of the linked image. No compiler change: they're ordinary
extern labels.
**Verification:** a two-file build whose header measures the whole image.

#### Phase E3 — `Align<N>` packer primitive
**Deliverable:** an entry whose byte width pads up to the next multiple of
`N`, computed in the packer's width pass (widths become a forward pass that
knows the running offset).
**Verification:** `span` across an `Align` is correct; alignment after
variable-length x86 instructions.

#### Phase E4 — `std/formats/elf.basm`
**Deliverable:** ELF64 and ELF32 static executables (RV32 needs ELF32);
machine constants for x86-64 and RISC-V.
**Verification:** byte-for-byte match with the old `wrap_elf` output for
the x86 hello; `readelf -h -l` on x86 and RISC-V output; the x86 hello
runs.

#### Phase E5 — `std/formats/pe.basm` and `std/formats/macho.basm`
**Deliverable:** ports of `wrap_pe` and `wrap_macho`.
**Verification:** byte-for-byte match with the old Rust output for the same
code; `objdump`/`file` accept them. (No Windows/macOS host to run them,
same as today.)

#### Phase E6 — Remove `formats.rs`, `bitter exec`, `--format`, `--entry`
**Deliverable:** `bitter build` writes bytes and sets `+x`; examples import
a format; README, `docs/reference.md` and the sections-and-linking doc's
superseded points updated.
