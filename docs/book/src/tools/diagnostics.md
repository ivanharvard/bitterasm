# Diagnostics and lints

## Errors

Every error points at the source that caused it, with a file, line and
column, the line itself, and a label:

```text
error: shift amount must be 0 to 31
  --> prog.basm:2:5
  |
2 |     @assert n >= 0 && n < 32, "shift amount must be 0 to 31"
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ error occurs here
```

`--diagnostic-format` picks the output: `terminal` (the default), `plain`, or
`json` for editors and build tools. `--color auto|always|never` controls
color; `auto` also respects the `NO_COLOR` environment variable.

## Lints

Warnings are **lints**. Each has a name, and a level that decides what
happens when it fires.

| Lint | Fires when |
|---|---|
| `unused_import` | An imported name is never used. |
| `unused_parameter` | A macro parameter is never used. Start its name with `_` to mark it unused on purpose. |
| `unreachable_code` | A statement comes after a `@return` or `@next` that always runs. |
| `fold_without_next` | A `@fold` body has no `@next`, so its accumulators never change. |
| `generated_declarations` | Macros generated declarations. See [Generating declarations](../macros/generating.md). |
| `unfulfilled_lint_expectation` | An `expect` facet's lint didn't fire. |

Two groups name several at once: `unused` (`unused_import` and
`unused_parameter`) and `all`.

## Levels

| Level | Effect |
|---|---|
| `allow` | Silent. |
| `expect` | Silent, but `unfulfilled_lint_expectation` fires if the lint *doesn't* occur. |
| `warn` | A warning. The default for every lint. |
| `deny` | An error. |
| `forbid` | An error, and no declaration can lower it. |

## Setting levels

**On a declaration**, with facets named after the level. Give one lint, or
several in parentheses:

```basm
macro compatibility(value: int)
    | allow unused_parameter
    | deny(unreachable_code, generated_declarations)
{
}
```

`expect` documents a lint you know about, and tells you once it's gone:

```basm,fail
macro uses_it(x: int)
    | expect unused_parameter
{
    @emit x
}

uses_it 3
```

```error
expected `unused_parameter` warning was not produced
```

(Warnings count as errors in this book's examples.)

**For a project**, in `bitterasm.toml`:

```toml
[lints]
unused = "warn"
unreachable_code = "deny"
```

**On the command line**, which overrides the project file:

```sh
bitterasm compile program.basm -A unused_parameter
bitterasm compile program.basm -D unreachable_code
```

`-A`, `-W`, `-D` and `-F` set `allow`, `warn`, `deny` and `forbid`. They
are applied in that order, whatever order you type them in, so
`-D all -A unused` still denies `unused`.

## Checking without compiling

`bitterasm check` runs every check `compile` does, including lints, but
writes no `.em` file. It takes the same options.
