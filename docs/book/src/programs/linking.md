# Linking multiple files

A program can be split across files, and `bitter build` links them into one
image:

```sh
bitter build main.basm util.basm -o program
```

## What linking does

1. Each input is compiled on its own.
2. Same-named [sections](sections.md) are joined across all inputs, in
   command-line order: every file's `.text`, then every file's `.data`, and
   so on.
3. Every [`pub` label](labels.md#labels-across-files) that one file imports
   and another declares is resolved to its final position.
4. The result is packed into bytes, and written marked as executable.

## Example

`main.basm` calls a routine in `util.basm`:

```basm,ignore
# main.basm
from std.riscv.native import *
from .util import double

section .text
pub _start:
    addi a0, zero, 21
    jal ra, double
```

```basm,ignore
# util.basm
from std.riscv.native import *

section .text
pub double:
    add a0, a0, a0
    jalr zero, ra, 0
```

```sh
bitter build main.basm util.basm -o prog.bin
```

`prog.bin` holds four instructions. `jal ra, double` is encoded as a jump of
4 bytes forward, to where `double` landed.

## Things to know

- **The first input goes first.** An [executable header](executables.md)
  must be emitted by the first file on the command line, before any
  `section` statement.
- **Positions inside a file are kept correct.** A branch to a label in the
  same file still lands on it after sections from other files are merged in
  around it.
- **`bitter encode` doesn't link.** It packs a single `.em` file, and fails
  if the file refers to a label in another file.

## Positions `bitter` provides

`std.bitter.link` declares two labels that `bitter` resolves against the
whole linked image, the way a linker defines symbols like `_end`:

```basm,ignore
from std.bitter.link import image_start, image_end
```

| Label | Position |
|---|---|
| `image_start` | The first value of the image |
| `image_end` | Just past its last value |

`span(image_start, image_end)` is the image's size in bytes, and
`span(image_start, label)` is a label's offset into the image. That's what
an executable header needs.
