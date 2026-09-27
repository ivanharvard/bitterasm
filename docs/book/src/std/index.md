# The standard library

`std` is written entirely in BitterASM. Nothing in it is built into the
compiler: bits, strings and whole instruction sets are ordinary modules you
could have written yourself, and can read in the repository's `std/`
directory.

## Core

| Module | Provides |
|---|---|
| `std.binary` | `bits<N>`, the `bool` type with `true` and `false`, `Endian`, `bit_width`, `byte_width` |
| `std.bitfield` | Bit-range helpers for encoders: `mask`, `bit`, `slice`, `field`, `truncate`, `place` |
| `std.ctypes` | C-style integer types: `int8_t`, `uint8_t`, ... `uint64_t` |
| `std.unsigned` | `uint`, a non-negative `int` |
| `std.array` | `Array<T, N>` and `get`, `updated`, `reversed`, `popped`, `appended`, `first`, `last`, `mapped`, `enumerate`, `array_from_struct` |
| `std.string` | Packed `String`, `AsciiString` and `Utf8String`, with validation and case conversion |
| `std.option` | `Option<T>` |
| `std.enumerated` | `Enumerated<T>`, an index with a value |
| `std.iter` | `Range` and `range(start, stop, step)` for stepped ranges |
| `std.decimal` | `Decimal` and `Fraction`, with conversions between them and `int` |
| `std.math` | Integer math (`pow`, `gcd`, `isqrt`, `popcount`, ...) and fixed-point math over `Decimal` (`sqrt`, `ln`, `sin`, `atan2`, ...) |

## For `bitter`

These define the types the `bitter` evaluator understands. See
[Packing bytes with `bitter`](../output/bitter.md).

| Module | Provides |
|---|---|
| `std.bitter.deferred` | `Deferred` values, `here()`, `span`, arithmetic on them, and `Positioned<N>` |
| `std.bitter.byte_order` | `LittleEndian<T, width>` |
| `std.bitter.layout` | `align` and `pad_image` |
| `std.bitter.link` | The `image_start` and `image_end` labels |

## Executable formats

See [Executables](../programs/executables.md).

| Module | Provides |
|---|---|
| `std.formats.elf` | `elf64_executable`, `elf32_executable` |
| `std.formats.pe` | `pe64_executable` |
| `std.formats.macho` | `macho64_executable` |

## Architectures

Each architecture has an `impl` module with its registers, instruction
formats and instructions, plus one or more **dialects** that give the
instructions a syntax. Import a dialect. See
[Custom syntax](../macros/syntax.md#changing-another-macros-syntax).

| Architecture | Dialects |
|---|---|
| x86-64 | `std.x86_64.intel`, `std.x86_64.nasm` (Intel plus NASM's `[rel label]`, `db` and friends), `std.x86_64.att` |
| RISC-V (RV32I) | `std.riscv.native`, `std.riscv.c_like` (`a0 = a1 + a2`) |
| WebAssembly | `std.wasm.module` (modules and sections), over `std.wasm.impl` |
| PDP-10 | `std.pdp10.impl` (36-bit words, no byte order) |

```basm
from std.riscv.c_like import *

a0 = a1 + a2
```

```bytes
33 85 c5 00
```

The same `add` instruction as in [Your first program](../getting-started/first-program.md),
through the C-like dialect.

## Reference

A generated reference for every public declaration is planned. Until then,
the source files are the reference: each starts with comments explaining
it.
