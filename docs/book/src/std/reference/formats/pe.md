# `pe`

[`std`](../index.md) › [`formats`](index.md) › `pe`

PE32+ (64-bit Windows) console executables.

Invoke the header macro first thing in the program's entry file, before
any `section` statement, so its bytes start the image:

```basm
from std.formats.pe import *
from std.x86_64.nasm import *

pe64_executable IMAGE_FILE_MACHINE_AMD64, _start

_start:
    ret
```

The image is the headers (padded to 512 bytes), then everything after
them as one read+execute `.text` section at RVA 0x1000, padded to 512
bytes at the end. `entry` is a label in this file or imported from
another one. No imports, exports or relocations: a program that calls
into Windows DLLs needs more than this provides.

Not verified on Windows: this reproduces the Rust writer it replaced byte
for byte, and `llvm-readobj` accepts its output.

## Macros

### `pe64_executable`

The DOS, PE and optional headers plus the `.text` section header, 368
bytes, padded to 512. `entry` is the label execution starts at, and the
image loads at `image_base`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pe64_executable machine, entry, image_base` | `machine: int`, `entry: int`, `image_base: int = 0x140000000` |  |  |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `IMAGE_FILE_MACHINE_AMD64` |  | `0x8664` | `machine` for x86-64. |
