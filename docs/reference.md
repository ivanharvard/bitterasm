# CLI and language reference

Details on macro-default semantics, struct-field visibility, the formatter,
and diagnostics/lints — split out of the README so that stays a high-level
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

Compiler warnings are structured lints. The initial lint set is
`unused_import`, `unused_parameter`, `unreachable_code`, `generated_declarations`, and
`unfulfilled_lint_expectation`; `unused` and `all` are lint groups.

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
