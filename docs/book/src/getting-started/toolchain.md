# The toolchain

## `bitterasm`

| Command | What it does |
|---|---|
| `bitterasm compile prog.basm [-o prog.em]` | Expands the program and writes its emitted values to a `.em` file. |
| `bitterasm check prog.basm` | Runs every check `compile` does, but writes nothing. |
| `bitterasm expand prog.basm` | Prints the program with every macro call replaced by the macro's body. Nothing is evaluated. |
| `bitterasm doc <paths> [-o dir]` | Writes reference pages from doc comments; `--test` compiles their examples. See [Generating docs](../tools/docs.md). |
| `bitterasm format <paths>` | Formats `.basm` files in place (`fmt` for short). See [Formatting](../tools/formatting.md). |

`compile` and `check` also take lint options (`-A`, `-W`, `-D`, `-F`) and
output options (`--diagnostic-format`, `--color`). See
[Diagnostics and lints](../tools/diagnostics.md). `compile --verbose` shows
progress and timing for each top-level call.

`compile` writes compact JSON. `--pretty` indents it for reading, in one
extra pass as it's written. `--timings` (also on `check` and `bitter build`)
prints how long each phase took, the peak memory, and the output's size.

## `bitter`

| Command | What it does |
|---|---|
| `bitter encode prog.em [-o prog.bin]` | Packs one `.em` file's values into bytes. |
| `bitter build a.basm [b.basm ...] [-o prog]` | Compiles every input, links them, packs the result and marks it executable. |

`bitter build` runs the whole pipeline in one process, so no `.em` file is
written or read. With several inputs, it joins their same-named
[sections](../programs/sections.md) and resolves `pub`
[labels](../programs/labels.md) across files. See
[Linking multiple files](../programs/linking.md) and
[Executables](../programs/executables.md).

## A typical session

```sh
bitterasm check hello.basm            # fast feedback while editing
bitter build hello.basm -o hello      # compile, link and pack
./hello
```
