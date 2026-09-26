# CLI and language reference

Details on macro-default semantics, struct-field visibility, spliced names,
`@fold`, executables, the `.em` format, the formatter, and diagnostics/lints — split out of the README so that stays a high-level
overview.

## Struct fields: `pub` and `skip`

A struct field's `pub` controls two things, the same way top-level `pub`
(on a `struct`/`macro`/`const`/`type`/`enum`) already controls whether
other files can reach a declaration by name:

- **Visibility.** A non-`pub` field can only be read (`.field`) or supplied
  by name in a brace/paren construction from code in the *module that
  declared the struct* — reading or naming it from anywhere else is a
  `PrivateFieldAccess` error. Positional construction (`Point(1, 2)`, no
  field names spelled out) is unaffected regardless of module, since it
  never depends on knowing a field's name. A struct's own `invariant`,
  field defaults, and `to`/`from` conversion templates always run as that
  struct's own module, so they can freely use their own non-`pub` fields no
  matter who triggers the construction/conversion.
- **`@for` inclusion.** `@for i in x` (`x` resolving to this struct's type)
  only visits `pub` fields in the first place — a non-`pub` field is
  internal bookkeeping (e.g. whatever an `invariant` needs) and is skipped
  regardless of the rest of the struct.

`pub skip name: T` marks a field that's fully visible (readable,
nameable, part of construction, from any module) but deliberately excluded
from `@for`'s walk anyway — for a field that isn't one of the struct's
"elements" even though it's genuinely public. `std.array`'s `Array<T, N>`
is the motivating case:

```text
pub struct Array<T, const N: int>
    | invariant N >= 0
{
    @for i in 0..N {
        pub __el`i`: T,
    }

    pub skip len: int = N,
}
```

Without `skip`, `@for x in someArray` would visit every `__el{i}` *and*
`len`, handing the loop body a trailing iteration where `x` is `len`'s
`int` value instead of a `T` — `skip` excludes `len` from that walk while
leaving it exactly as constructible and readable (`arr.len`) as any other
`pub` field. `skip` on a non-`pub` field is legal but has no effect: such a
field is already excluded from `@for`.

## Character and string literals

A character literal, such as `'a'`, evaluates to the Unicode scalar value
for that character as an `int`. This is architecture-independent: `'€'`
is `0x20AC`, not a target-machine byte sequence.

A string literal, such as `"abc"`, evaluates to a generated struct with one
public `int` element per character (`__el0`, `__el1`, ...), plus a public
`skip len` field. Iterating it with `@for` visits only the character
elements, while indexed helper code can still read `.len`.

`std.array.array_from_struct("abc")` converts that generated struct into
`Array<int, 3>`. `std.string.string_from_struct("abc")` UTF-8 encodes it
into the packed `String<N>` representation used by `std.string`.

## `in`: membership as a boolean

`value in source` tests the same relation `@for var in source` walks
generatively — true when `source` produces some element equal to `value` —
just checked instead of bound to `var`. `source` is either shape `@for`
already accepts:

- **A range.** `idx in 0..len` / `idx in 0..=len` is a plain bound check
  (`0 <= idx < len`, or `<= len` for the inclusive form) — no iteration
  happens, so it's cheap even for a range far larger than `@for` would ever
  actually unroll.
- **A struct/array value.** `x in someArray` is true when `x` equals one of
  `someArray`'s iterable elements — exactly the `pub`, non-`skip` fields
  `@for i in someArray` would visit, in the same order. A field excluded
  from `@for` (non-`pub`, or `pub skip`) is equally excluded from `in`.

```text
macro assert_in_bounds(idx: int, len: int) {
    | invariant idx in 0..len
}
```

`in` doesn't extend to a type name (`x in SomeEnum`, testing whether `x` is
one of `SomeEnum`'s valid discriminants) — that's membership against a
*type* rather than a value already in hand, a different relation than the
one `@for`/`in` share, so it's deliberately out of scope rather than
overloading the same keyword with a second meaning.

## Spliced names

`` r`id` `` builds a name from literal text and evaluated pieces: `r`
followed by `id`'s value, so `` r`3` `` is `r3`. It works in declarations
(`` pub const r`i` = ... ``), in field names (`` __el`i`: ... ``), after a
`.` (`` arr.__el`i` ``), and in expressions, where it reads whatever that
name refers to:

```text
@for i in 0..4 {
    pub const k`i` = i * i
}

macro show(n: int) {
    @emit k`n`
}
```

In an expression the backtick must touch the name: `` k`n` `` is a spliced
name, while `` k `n` `` is `k` followed by a separate splice. A spliced read
finds a module's own non-`pub` constants as well as `pub` ones.

A `const` declared inside a `@for` body only exists for that iteration, so
a spliced name can't carry a value from one iteration to the next. Use
`@fold` for that.

## `@fold`: loops that carry values

`@fold` puts accumulators in front of an ordinary `@for`. Each iteration
sees the accumulators' current values, and `@next` gives the values for the
next iteration:

```text
from std.array import Array

macro offsets<const N: int>(lengths: Array<int, N>) {
    const table_size = @fold offset = 0 @for len in lengths {
        @emit offset
        @next offset + len
    }
    @emit table_size
}
```

`offsets Array<int, 3> { __el0: 4, __el1: 2, __el2: 5 }` emits `0`, `4`,
`6` (each entry's offset), then `11` (the table's size).

- **`@next` ends the iteration**, like `continue`, carrying new values.
  With one accumulator, write `@next value`. With several, name the ones
  that change (`@next offset = offset + 1, count = count + 1`).
- **Anything `@next` doesn't name keeps its value**, and an iteration that
  reaches no `@next` at all keeps every value. So a filter needs no `@else`:
  `@if len > 0 { @next offset + len }`. The `fold_without_next` lint
  warns about a fold whose body has no `@next` anywhere.
- **Nothing is mutated.** Each iteration binds fresh values, the same way
  `@for` binds its loop variable.
- **The value** is the final accumulator with one accumulator, or a struct
  with one `pub` field per accumulator with several:
  `const r = @fold a = 0, b = 0 @for ... { ... }`, then `r.a` and `r.b`.
- **It's a statement or an expression.** As a statement its value is
  unused, which is what you want when the body only `@emit`s.
- **`@return` inside the body** returns from the enclosing macro.
- **`@next` belongs to the innermost `@fold`** in the same macro body. It's
  an error inside a plain `@for` nested in the fold, and in a macro called
  from the fold's body.
- **No depth limit.** A fold runs as many iterations as its source has
  elements (up to `@for`'s limit of 1,000,000), unlike recursion, which is
  limited to 32 nested calls or 4,096 tail calls.

`@fold` works everywhere `@for` does:

- **Construction literals**, producing fields:
  `Array<int, 4> { @fold offset = 0 @for i in 0..4 { __el`i`: offset, @next offset + lengths.__el`i` } }`.
- **Struct declarations**, where accumulators are integers (like a
  struct-body `@for`'s variable), e.g. to compute field names.
- **Top level**, where it unrolls before anything else is resolved, like
  top-level `@for`: the source must be a literal range, and accumulators are
  integer constants. `const x = @fold ...` binds the final value, so `x`
  can bound a later top-level `@for`.

### Where emits can go

A `@fold`, or a macro call, whose body `@emit`s may be used as a statement,
as a `const`'s whole value, or as `@return`'s whole value: its emitted
values are kept, in order, where it appears. Inside a larger expression
(`@emit 1 + side()`), emitting is an error, because the values would have
nowhere to go.

## Macro defaults

Trailing macro parameters may provide default value expressions:

```text
macro encode(value: int, width: int = 8, mask: int = (1 << width) - 1) {
    @emit value & mask
}
```

Calls may omit any trailing defaulted parameters. Defaults are evaluated from left to
right at the call site, so a default may refer to an earlier parameter. A required
parameter may not follow a defaulted parameter. Overload selection considers every
arity between a macro's required parameter count and its total parameter count; a call
is ambiguous if more than one overload accepts the supplied argument types and arity.

Macros may call themselves, directly or mutually, including from value expressions.
Statement and expression calls share one call stack. To keep malformed compile-time
programs from overflowing the compiler's host stack, expansion fails with
`MacroCallDepthExceeded` when a call chain would exceed 32 nested macro calls.
An exact direct self-tail-call such as `@return gcd(b, a % b)` reuses the current
frame, rebinding and rechecking its arguments instead of consuming call depth. Calls
wrapped in another expression are not tail calls. Macros with `after` hooks are also
excluded because eliminating their frames would change unwind-time hook behavior.
Tail-call loops are capped at 4,096 restarts and fail with
`MacroTailCallLimitExceeded` if they do not reach a base case.

## Executables

`bitter build a.basm [b.basm ...] -o program` compiles every input, links
them (see `docs/sections-and-linking/PROGRESS.md`), packs the result, and
writes it marked executable. It adds nothing of its own: an executable
format's header is BitterASM code the program writes, first thing in its
first input, before any `section` statement, so it starts the image:

| Module | Header macro | Notes |
|---|---|---|
| `std.formats.elf` | `elf64_executable EM_X86_64, _start` | also `elf32_executable` (e.g. `EM_RISCV`); optional `load_address`, `segment_flags` (`PF_R`/`PF_W`/`PF_X`), `flags` |
| `std.formats.pe` | `pe64_executable IMAGE_FILE_MACHINE_AMD64, _start` | console PE32+; optional `image_base` |
| `std.formats.macho` | `macho64_executable CPU_TYPE_X86_64, CPU_SUBTYPE_X86_64_ALL, _start` | optional `vm_address` |

The entry label may be in any input. Each format maps the whole image as one
segment (readable and executable by default) and has no dynamic linking,
imports or relocations. Without a header, `bitter build` writes a flat binary,
like `nasm -f bin`.

The headers are built from pieces any other format can use:

- `std.bitter.link`'s `image_start` and `image_end`, imported like `pub`
  labels, which `bitter` resolves to the start and end of the linked image.
  `span(image_start, image_end)` is the image's size in bytes.
- `std.bitter.deferred`'s `add`, `sub`, `band`, ... over those positions.
- `std.bitter.layout`'s `align n` (zero bytes up to the next multiple of `n`)
  and `pad_image n` (pad the finished image to a multiple of `n`, wherever
  it's written; it takes no space where it appears).

## The `.em` format

`bitterasm compile` writes a program's emitted values to a `.em` file, and
an evaluator such as `bitter` reads it. This is the contract between them;
anything that reads `.em` should follow it.

A `.em` file is one JSON object:

```json
{
  "version": 1,
  "requires": ["sections", "extern-labels"],
  "module": "spec",
  "exports": { "start": 0 },
  "entries": [
    { "kind": "Struct", "id": "std.binary.bits",
      "args": [{ "kind": "Const", "value": "8" }],
      "fields": [["value", { "kind": "Int", "value": "7" }]],
      "section": ".text" },
    { "kind": "Enum", "id": "spec.Mode", "args": [], "variant": "Slow",
      "payload": { "kind": "Int", "value": "3" }, "section": ".text" },
    { "kind": "Deferred", "module": "spec_dep", "symbol": "far", "section": ".text" }
  ]
}
```

- **`version`** is `1`. It changes only when the file's structure changes
  incompatibly. A reader must refuse any version it doesn't know, and must
  refuse the unversioned plain-list files older compilers wrote.
- **`requires`** lists the language features the program actually uses. A
  reader must refuse a file that requires a feature it doesn't know,
  because ignoring one produces wrong output with no error:
  - `sections`: some entry has a `section`. Lay entries out grouped by
    section name, sections in order of first appearance, entries within a
    section in file order.
  - `extern-labels`: some value is a `Deferred` (see below), which only a
    linker with the other file's `.em` can resolve.
- **`module`** is the compiled file's module path: its path relative to the
  deepest search root containing it, with dots (`examples.x86_64.hello`).
  A file under no search root is named relative to the working directory,
  with one leading `.` per level up plus one (`..shared.util`).
- **`exports`** maps each top-level `pub` label to its position: how many
  entries precede it.
- **`entries`** is the emitted values, in emission order. Each is one of
  these, tagged by `kind`, plus an optional `section`:
  - `Int`: `value` is a decimal string, since integers are unbounded.
  - `Struct`: `id`, generic `args`, and `fields` as `[name, value]` pairs
    in declaration order.
  - `Enum`: `id`, generic `args`, `variant`, and an optional `payload`.
  - `Deferred`: the value of `pub` label `symbol` in the file whose
    `module` is given, not known until link time.

A generic argument is `{"kind": "Const", "value": "8"}` or
`{"kind": "Type", "type_kind": ..., ...}`, where the type is
`{"type_kind": "Builtin", "name": "int"}`, or `Struct`/`Enum` with an `id`
and its own `args`.

**Ids.** A struct or enum's `id` is its declaring module's path plus its
name: `std.binary.bits`, `spec.Mode`. Ids are unique within one program, and
they're what an evaluator matches on. Which ids an evaluator gives meaning
to is up to that evaluator. `bitter` understands `std.binary.bits`,
`std.bitter.byte_order.LittleEndian`, and `std.bitter.deferred`'s
`Positioned`, `Deferred`, `BinOp` and `Op`, and packs any other struct as
the concatenation of its fields.

A later version-1 file may add top-level fields that a reader can safely
ignore. Anything a reader must understand to produce correct output is
either a new `requires` feature or a new `version`.

## Formatting

Format one file, several files, or every `.basm` file below a directory:

```sh
bitterasm format program.basm
bitterasm fmt std/
bitterasm format --check .
```

Like `rustfmt`, the formatter searches the file's directory and then its parents for
`bitterasm.toml` (or `.bitterasm.toml`). Pass `--config path/to/bitterasm.toml` to
select one explicitly. All settings are optional:

```toml
indent_width = 4
hard_tabs = false
indent_facets = true
facets_on_new_line = true
pub_on_declaration = true
return_type_on_declaration = true
collapse_short_multiline_generics = true
max_blank_lines = 1
max_width = 100
comment_width = 80
newline_style = "Auto" # Auto, Unix, or Windows
```

Formatting preserves comments and source tokens while normalizing delimiter-aware
indentation, trailing whitespace, blank lines, comment wrapping, and the final newline.
Long code is wrapped at safe commas inside `()`, `[]`, and `{}`; lines with no safe
split may exceed `max_width` because newlines terminate BitterASM statements. `--check`
makes no changes and returns a non-zero exit status if any input would be reformatted.

## Diagnostics and lints

Lexer, parser, loader, and resolver failures use the same structured
diagnostic model and renderer as warnings. Source-backed failures include a
path, line and column, excerpt, and primary label; JSON output exposes the
same information for editor and build-tool integrations.

Compiler warnings are structured lints: `unused_import`, `unused_parameter`,
`unreachable_code`, `generated_declarations`, `fold_without_next`, and
`unfulfilled_lint_expectation`. `unused` and `all` are lint groups.

Configure a declaration with facets:

```text
macro compatibility(value: int)
    | allow unused_parameter
    | deny(unreachable_code, generated_declarations)
{
}
```

The supported levels are `allow`, `expect`, `warn`, `deny`, and `forbid`.
`expect` suppresses the selected warning but emits
`unfulfilled_lint_expectation` if that warning does not occur. `deny` and
`forbid` promote warnings to compilation errors; a `forbid` level cannot be
lowered by a declaration facet.

Project-wide levels live in `bitterasm.toml`:

```toml
[lints]
unused = "warn"
unreachable_code = "deny"
```

Command-line settings override project settings:

```sh
bitterasm compile program.basm -A unused_parameter
bitterasm compile program.basm -D unreachable_code
bitterasm check program.basm
```

`bitterasm check` runs the same lints, loading, resolution, and expansion
validation as `compile`, but does not serialize emitted values or create a
`.em` file. It accepts the same diagnostic and lint-level options.

Use `--diagnostic-format terminal`, `plain`, or `json`. Terminal color is
controlled with `--color auto|always|never`; automatic color also respects
the `NO_COLOR` environment variable. The compiler library represents
diagnostics as structured values and leaves rendering to callers.
