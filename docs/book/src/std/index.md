# The standard library

`std` is written entirely in BitterASM. Nothing in it is built into the
compiler: bits, strings and whole instruction sets are ordinary modules you
could have written yourself, and can read in the repository's `std/`
directory.

## Core

| Module | Provides |
|---|---|
| `std.binary` | `bits<N>`, two's-complement `signed<N>`, the `bool` type with `true` and `false`, `Endian`, `bit_width`, `byte_width` |
| `std.bitfield` | Bit-range helpers for encoders: `mask`, `bit`, `slice`, `field`, `truncate`, `place` |
| `std.ctypes` | C-style integer types: signed `int8_t` ... `int64_t` and unsigned `uint8_t` ... `uint64_t` |
| `std.unsigned` | `uint`, a non-negative `int` |
| `std.array` | `Array<T, N>` and `get`, `updated`, `reversed`, `popped`, `appended`, `concatenated`, `filled`, `first`, `last`, `mapped`, `enumerate`, `find`, `contains`, `remove`, `sum`, `any`, `all`, `sorted`, `array_from_struct` |
| `std.strings.packed` | The base for packed strings: raw `String`, `unit_at`, `byte_at`, `validate_codepoint` |
| `std.strings.ascii` | `AsciiString`, with validation and case conversion |
| `std.strings.utf8` | `Utf8String`, and `string_from_struct` to encode a literal as UTF-8 |
| `std.strings.utf16` | `Utf16String`, and `utf16_from_struct` to encode a literal as UTF-16 |
| `std.strings.cstring` | `nul_terminated`, for C strings in any of those encodings |
| `std.option` | `Option<T>`, and `is_some`, `is_none`, `unwrap`, `unwrap_or`, `mapped` |
| `std.list` | `List<T>`, a linked list, and `list`, `nil`, `length`, `get`, `reversed`, `concatenated`, `mapped`, `struct_as_list` |
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
| x86-64 | `std.x86_64.intel`, `std.x86_64.nasm` (Intel plus NASM's `[rel label]`), `std.x86_64.att` (GNU's AT&T syntax), and `std.x86_64.data` (`db` and friends, which both re-export) |
| RISC-V (RV32I) | `std.riscv.native`, `std.riscv.c_like` (`a0 = a1 + a2`) |
| WebAssembly | `std.wasm.module` (modules and sections), over `std.wasm.impl` |
| PDP-10 | `std.pdp10.impl` (36-bit words, no byte order) |
| AVR | `std.avr.native` (`ld r16, X+`), over `std.avr.impl`; `std.avr.atmega328p` names that chip's registers |

```basm
from std.riscv.c_like import *

a0 = a1 + a2
```

```bytes
33 85 c5 00
```

The same `add` instruction as in [Your first program](../getting-started/first-program.md),
through the C-like dialect.

### Coverage

How much of each architecture `std` supports, counted in **operand forms**:
an instruction with one kind of operand in each position. `add reg, reg`
and `add reg, mem` are two forms. Operand sizes don't count separately, so
`add reg, reg` stands for 8- to 64-bit registers alike. A form is covered
when it compiles through the architecture's main dialect.

<!-- coverage:start -->
| Architecture | Reference set | Operand forms covered |
|---|---|---|
| AVR | ATmega328P (avr5) | 131 of 131 (100%) |
| PDP-10 | KA10 | 11 of 366 (3%) |
| RISC-V | RV32I | 40 of 40 (100%) |
| WebAssembly | WebAssembly 1.0 | 23 of 172 (13%) |
| x86-64 | baseline general-purpose, user mode | 299 of 301 (99%) |
<!-- coverage:end -->

Each architecture's list of forms, with what's covered marked, is in
[`tests/coverage/`](https://github.com/ivanharvard/bitterasm/tree/main/tests/coverage)
in the repository.

## Reference

The [std reference](reference/index.md) lists every public declaration of
every module, generated from std's doc comments by `bitterasm doc`. See
[Generating docs](../tools/docs.md) to do the same for your own modules.
