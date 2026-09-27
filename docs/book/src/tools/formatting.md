# Formatting

`bitterasm format` (or `fmt`) rewrites `.basm` files in a consistent style:

```sh
bitterasm format program.basm     # one file
bitterasm fmt std/                # every .basm file below a directory
bitterasm format --check .        # change nothing; fail if anything would change
```

`--check` is meant for CI: it exits with a non-zero status if any file isn't
formatted.

## What it changes

- Indentation, following `()`, `[]` and `{}`, plus one level for the lines
  under a top-level label.
- Facets, each moved to its own indented line.
- Trailing whitespace, runs of blank lines, and the final newline.
- Comments longer than `comment_width`, which are wrapped.
- Long lines, which are wrapped at commas inside `()`, `[]` and `{}`. A line
  with no such comma stays long, since a newline would end the statement.

It keeps every comment and never changes what the code means.

## Configuration

The formatter reads `bitterasm.toml` (or `.bitterasm.toml`), searching the
file's directory and then its parents, like `rustfmt`. Pass
`--config path/to/bitterasm.toml` to choose one. Every setting is optional:

| Setting | Default | Meaning |
|---|---|---|
| `indent_width` | `4` | Spaces per indentation level. |
| `hard_tabs` | `false` | Indent with tabs instead of spaces. |
| `indent_facets` | `true` | Indent `\| facet` lines one level under their declaration. |
| `facets_on_new_line` | `true` | Move facets written on the declaration's line onto their own lines. |
| `indent_label_bodies` | `true` | Indent the lines under a top-level label, up to the next label, `section` or declaration. |
| `pub_on_declaration` | `true` | Rewrite an old-style `\| pub` line as `pub` on the declaration. |
| `return_type_on_declaration` | `true` | Rewrite an old-style `\| -> T` line as `-> T` on the declaration. |
| `collapse_short_multiline_generics` | `true` | Join a generic argument list that was split across lines back onto one, when it fits. |
| `max_blank_lines` | `1` | The most consecutive blank lines kept. |
| `max_width` | `100` | The line width code is wrapped at. |
| `comment_width` | `80` | The line width comments are wrapped at. |
| `newline_style` | `"Auto"` | `"Auto"`, `"Unix"` or `"Windows"` line endings. |

```toml
indent_width = 2
max_width = 90
```

The same file holds [lint settings](diagnostics.md#setting-levels), under
`[lints]`.
