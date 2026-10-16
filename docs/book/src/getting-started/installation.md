# Installation

BitterASM is built from source and needs a Rust toolchain (`cargo`). From a
clone of the repository:

```sh
make install    # or: ./install.sh
```

The script asks before installing each part:

- `bitterasm` (the compiler) and `bitter` (the binary evaluator), into
  `~/.bitterasm/bin`.
- The standard library, copied from `std/` to `~/.bitterasm/std`.
- Optionally, the `bitterasm-lsp` language server.

Pass `-y` to answer yes to every prompt. Add `~/.bitterasm/bin` to your
`PATH` if the script tells you to.

## Where imports are found

An absolute import such as `from std.riscv.native import *` is looked up in
these directories, in order:

1. The current directory.
2. Each directory in `BITTERASM_PATH` (separated like `PATH`).
3. On non-Apple Unix, `$XDG_DATA_HOME/bitterasm` (defaulting to
   `~/.local/share/bitterasm`).
4. On non-Apple Unix, each `$XDG_DATA_DIRS/bitterasm` (defaulting to
   `/usr/local/share/bitterasm` and `/usr/share/bitterasm`).
5. The legacy install root `~/.bitterasm`.

So a program anywhere on disk finds the installed `std`, and a project's own
`std/` directory takes priority over it. See [Imports](../modules/imports.md).

## Checking it works

```sh
bitterasm --version
bitter --version
```

Next: [Your first program](first-program.md).
