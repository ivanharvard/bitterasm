# `std`

| Name | Summary |
|---|---|
| [`array`](array.md) | `Array<T, N>`: `N` values of type `T`, and ways to build new arrays from old ones. Every value is immutable, so each operation returns a new array. |
| [`binary`](binary.md) | Fixed-width binary values: `bits<N>`, `signed<N>`, `bool`, and byte order. |
| [`bitfield`](bitfield.md) | Bit ranges, written the way ISA manuals write them. `slice(x, 10, 5)` is the manual's `x[10:5]`: `hi` and `lo` are inclusive bit numbers, so a one-bit field is `(n, n)`. The shift, the mask and the result's width all come from those two numbers, so they can't disagree. |
| [`bitter/`](bitter/index.md) | `byte_order`, `deferred`, `layout`, `link` |
| [`ctypes`](ctypes.md) | C's fixed-width integer names. The signed ones are `signed<N>`, stored in two's complement; the unsigned ones are `bits<N>` that can't be negative. |
| [`decimal`](decimal.md) | Exact non-integer numbers, for compile-time math: `Decimal`, a value scaled by a power of ten, and `Fraction`, a numerator over a denominator. Both convert to and from `int` with `as`. |
| [`enumerated`](enumerated.md) | `Enumerated<T>`: a value paired with its position. |
| [`formats/`](formats/index.md) | `elf`, `macho`, `pe` |
| [`iter`](iter.md) | Stepped ranges of integers, for `@for`. |
| [`math`](math.md) | Integer math, and fixed-point math on `Decimal` and `Fraction`, all evaluated at compile time. |
| [`option`](option.md) | `Option<T>`: a value that may be missing. |
| [`pdp10/`](pdp10/index.md) | `impl`, `tops10` |
| [`riscv/`](riscv/index.md) | `c_like`, `impl`, `native` |
| [`string`](string.md) | Strings packed into one integer. A string literal like `"hi"` is a struct of code points; `string_from_struct` encodes it as UTF-8 bytes in a single `int`, with its length in bytes. |
| [`unsigned`](unsigned.md) | `uint`, an `int` that can't be negative. |
| [`wasm/`](wasm/index.md) | `impl`, `leb128`, `module` |
| [`x86_64/`](x86_64/index.md) | `att`, `impl`, `intel`, `nasm` |
