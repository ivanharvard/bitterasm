# `packed`

[`std`](../index.md) › [`strings`](index.md) › `packed`

The shared base of `std.strings`: strings packed into one integer. A
string literal like `"hi"` is a struct of code points. Each encoding
packs it as a run of fixed-width code units in a single `int`:

- [`std.strings.ascii`](ascii.md): `AsciiString`, one byte per character.
- [`std.strings.utf8`](utf8.md): `Utf8String`, and `string_from_struct`,
  which encodes a literal as UTF-8.
- [`std.strings.utf16`](utf16.md): `Utf16String`, 16-bit code units.
- [`std.strings.cstring`](cstring.md): `nul_terminated`, for C.

Every string type has a `len`, counted in code units, and an `endian`:
whether unit 0 is the most (`Big`) or least (`Little`) significant unit
of `value`. The `endian` is only how the units are packed. It's not the
byte order the units are written out in, which an architecture's data
directives decide.

```basm
from std.strings.packed import unit_at
from std.binary import Endian

macro show(value: int) {
    @emit value
}

show unit_at(0x4869, 2, 0, Endian.Big, 8)
show unit_at(0x4869, 2, 0, Endian.Little, 8)
```

```emits
72 105
```

## Macros

### `validate_units`

1 if `value` fits in `len` code units of `width` bits each. Otherwise
it's a compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_units(value, len, width)` | `value: int`, `len: int`, `width: int` | returns `int` |  |

### `unit_shift`

How far `value` is shifted right to bring code unit `index` of `len` to
the bottom, for units of `width` bits.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `unit_shift(index, len, endian, width)` | `index: int`, `len: int`, `endian: Endian`, `width: int` | returns `int` |  |

### `unit_at`

Code unit `index` of `len` units of `width` bits packed into `value`,
counting from the `endian` end. `index` must be within the string.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `unit_at(value, len, index, endian, width)` | `value: int`, `len: int`, `index: int`, `endian: Endian`, `width: int` | returns `int` |  |

### `byte_at`

Byte `index` of `len` bytes packed into `value`, counting from the
`endian` end. `index` must be within the string.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `byte_at(value, len, index, endian)` | `value: int`, `len: int`, `index: int`, `endian: Endian` | returns `int` |  |

### `validate_codepoint`

1 if `codepoint` is a Unicode scalar value: 0 to U+10FFFF, and not a
surrogate (U+D800 to U+DFFF). Otherwise it's a compile error. Every
Unicode encoding can store exactly these.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_codepoint(codepoint)` | `codepoint: int` | returns `int` |  |

## Types

### `String`

`struct String<const len: int>`

`len` bytes packed into `value`, first byte most significant, not yet
checked to be any encoding. `std.strings.utf8`'s `string_from_struct`
builds one. Convert it with `as` to `AsciiString` or `Utf8String` to
check its bytes and choose their order.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The bytes, packed. |
| `len` | `int` | The number of bytes. |
