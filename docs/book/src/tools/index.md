# Tools

Besides compiling, `bitterasm` has commands that help while writing code:

| Command | Page |
|---|---|
| `bitterasm format` | [Formatting](formatting.md) |
| `bitterasm check` | [Diagnostics and lints](diagnostics.md) |
| `bitterasm doc` | [Generating docs](docs.md) |
| `bitterasm expand` | [Generating declarations](../macros/generating.md#seeing-what-was-generated) |

Both `format` and the lint settings read an optional `bitterasm.toml`,
found by searching the file's directory and then its parents.

An editor language server, `bitterasm-lsp`, can be installed alongside the
compiler. See [Installation](../getting-started/installation.md).

For the full list of commands, see [The toolchain](../getting-started/toolchain.md).
