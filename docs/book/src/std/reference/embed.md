# `embed`

[`std`](index.md) › `embed`

Reading what `@embed` gives. `@embed "file"` is the file's contents as
numbered units, in a struct shaped like a string's: `__el0`, `__el1`,
..., and `len`. It says nothing about what the units are. These macros
are for a program that knows its file holds bytes.

```text
from std.embed import emit_bytes

# Pastes `logo.bin`, beside this file, into the output here, untouched.
emit_bytes @embed "logo.bin"
```

A unit that doesn't fit in a byte, 0 to 255, is a compile error.

## Macros

### `as_bytes`

The units of `raw`, from `@embed`, as bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `as_bytes(raw)` | `<S>`, `raw: S` | returns `Array<bits<8>, ...>` |  |

### `emit_bytes`

Emits each unit of `raw`, from `@embed`, as a byte: the file, pasted
into the output.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `emit_bytes raw` | `<S>`, `raw: S` |  |  |
