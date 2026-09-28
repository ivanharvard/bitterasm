# `macho`

[`std`](../index.md) › [`formats`](index.md) › `macho`

Mach-O 64 executables, for macOS.

Invoke the header macro first thing in the program's entry file, before
any `section` statement, so its bytes start the image:

```basm
from std.formats.macho import *
from std.x86_64.nasm import *

macho64_executable CPU_TYPE_X86_64, CPU_SUBTYPE_X86_64_ALL, _start

_start:
    ret
```

The whole image is one `__TEXT` segment at `vm_address`, readable and
executable, and `entry` (a label in this file or imported from another
one) is where execution starts. A modern macOS kernel refuses an
executable without a dynamic linker even if it never calls a shared
library, so the header names `/usr/lib/dyld`; current macOS may still
refuse an unsigned binary depending on Gatekeeper policy.

Not verified on macOS: this reproduces the Rust writer it replaced byte
for byte, and `llvm-readobj` accepts its output.

## Macros

### `macho64_executable`

The Mach-O header and its three load commands (the `__TEXT` segment, the
dynamic linker and the entry point), 160 bytes. `entry` is the label
execution starts at, and the segment loads at `vm_address`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `macho64_executable cpu_type, cpu_subtype, entry, vm_address` | `cpu_type: int`, `cpu_subtype: int`, `entry: int`, `vm_address: int = 0x100000000` |  |  |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `CPU_TYPE_X86_64` |  | `0x01000007` | `cpu_type` for x86-64. |
| `CPU_SUBTYPE_X86_64_ALL` |  | `3` | `cpu_subtype` for any x86-64 processor. |
