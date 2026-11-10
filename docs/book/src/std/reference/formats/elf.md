# `elf`

[`std`](../index.md) › [`formats`](index.md) › `elf`

ELF static executables, for Linux and the BSDs.

Invoke one header macro first thing in the program's entry file, before
any `section` statement, so its bytes start the image. This one is a
complete Linux x86-64 program that exits with status 0:

```basm
from std.formats.elf import *
from std.x86_64.nasm import *

elf64_executable EM_X86_64, _start

_start:
    mov eax, 60     # exit
    xor edi, edi    # with status 0
    syscall
```

The header maps the whole image as one segment at `load_address` and
starts execution at `entry`, a label in this file or imported from another
one. Its sizes and entry point come from `span` over the linked image
(`std.bitter.link`), so they stay correct however the program's sections
and files are laid out.

What this doesn't produce: section headers, dynamic linking, or separate
segments per section. The single segment is readable and executable by
default; pass `segment_flags` (a combination of `PF_R`, `PF_W`, `PF_X`) to
change that. Every multi-byte field is little-endian, as x86-64 and
RISC-V are.

## Macros

### `progbits`

The section's contents are stored in the file: code and data. Sets
`elf_type` to `SHT_PROGBITS`.

```basm
from std.formats.elf import progbits, alloc, exec

const CODE = section .text
    | progbits
    | alloc
    | exec

@into CODE
```

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `progbits(sec)` | `sec: section` | returns `section` |  |

### `nobits`

The section takes no room in the file and is zeroed when the program
loads, like `.bss`. Sets `elf_type` to `SHT_NOBITS`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `nobits(sec)` | `sec: section` | returns `section` |  |

### `alloc`

The section is in memory while the program runs. Sets `alloc`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `alloc(sec)` | `sec: section` | returns `section` |  |

### `write`

The program can write to the section. Sets `write`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `write(sec)` | `sec: section` | returns `section` |  |

### `exec`

The section holds code. Sets `exec`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `exec(sec)` | `sec: section` | returns `section` |  |

### `elf64_executable`

A 64-bit ELF header and its one program header, 120 bytes. `machine` is
`EM_X86_64` or `EM_RISCV`, `entry` is the label execution starts at, and
`flags` is the header's `e_flags`, which some architectures give meaning.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `elf64_executable machine, entry, load_address, segment_flags, flags` | `machine: int`, `entry: label`, `load_address: int = 0x400000`, `segment_flags: int = 5`, `flags: int = 0` |  |  |

### `elf32_executable`

A 32-bit ELF header and its one program header, 84 bytes, e.g. for
RV32. The parameters mean what they do for `elf64_executable`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `elf32_executable machine, entry, load_address, segment_flags, flags` | `machine: int`, `entry: label`, `load_address: int = 0x10000`, `segment_flags: int = 5`, `flags: int = 0` |  |  |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `EM_X86_64` |  | `0x3E` | `machine` for x86-64. |
| `EM_RISCV` |  | `0xF3` | `machine` for RISC-V. |
| `PF_X` |  | `1` | `segment_flags`: the segment is executable. |
| `PF_W` |  | `2` | `segment_flags`: the segment is writable. |
| `PF_R` |  | `4` | `segment_flags`: the segment is readable. |
| `SHT_PROGBITS` |  | `1` | A section's `elf_type`: its contents are stored in the file. |
| `SHT_NOBITS` |  | `8` | A section's `elf_type`: it takes no room in the file, and is zeroed when the program loads. |

## Sections

| Constant | Section | Space | Facets | Description |
|---|---|---|---|---|
| `TEXT` | `.text` | `stream` | `progbits`, `alloc`, `exec` | Code: stored in the file, in memory while running, and executable. |
| `RODATA` | `.rodata` | `stream` | `progbits`, `alloc` | Constants: stored in the file and in memory while running. |
| `DATA` | `.data` | `stream` | `progbits`, `alloc`, `write` | Variables with starting values: stored in the file, and writable. |
| `BSS` | `.bss` | `stream` | `nobits`, `alloc`, `write` | Variables that start at zero: no room in the file, and writable. |
