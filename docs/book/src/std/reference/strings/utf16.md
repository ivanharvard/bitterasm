# `utf16`

[`std`](../index.md) › [`strings`](index.md) › `utf16`

UTF-16 strings: code points as 16-bit code units, the encoding Windows
APIs and Java use. A character past U+FFFF takes two units, a surrogate
pair.

```basm
from std.strings.utf16 import *

macro show(value: int) {
    @emit value
}

macro demo() {
    const s = utf16_from_struct("h€𝄞")
    show s.len
    show utf16_codepoint_count(s)
    show utf16_unit_at(s, 2)
}

demo
```

```emits
4 3 55348
```

`𝄞` is U+1D11E, past U+FFFF, so it takes the surrogate pair `0xD834`
(`55348`) and `0xDD1E`. `len` counts code units, not bytes, and `endian`
is which end of `value` holds unit 0. The byte order each unit is
written in is up to the directive that emits it.

## Macros

### `validate_utf16`

1 if `len` 16-bit code units packed into `value` are valid UTF-16: every
high surrogate is followed by a low one, and every low surrogate follows
a high one. Otherwise it's a compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_utf16(value, len, endian)` | `value: int`, `len: int`, `endian: Endian` | returns `int` |  |

### `utf16_struct_unit_len`

How many code units a string literal (or any struct of code points)
takes as UTF-16.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf16_struct_unit_len(source)` | `<S>`, `source: S` | returns `int` |  |

### `utf16_from_struct`

A string literal (or any struct of code points) as a `Utf16String`,
first unit most significant.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf16_from_struct source` | `<S>`, `source: S` |  |  |

### `utf16_unit_at`

Code unit `index` of `s`. This is a unit, not a character: a character
past U+FFFF takes two.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf16_unit_at(s, index)` | `s: Utf16String`, `index: int` | returns `int` |  |

### `utf16_codepoint_count`

The number of characters (code points) in `s`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf16_codepoint_count(s)` | `s: Utf16String` | returns `int` |  |

## Types

### `Utf16String`

`struct Utf16String<const len: int, const endian: Endian>`

`len` UTF-16 code units packed into `value`, with unit 0 at the `endian`
end. Every surrogate is part of a pair.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The code units, packed. |
| `len` | `int` | The number of code units. |
| `endian` | `Endian` | Which end of `value` holds unit 0. |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `UTF16_UNIT_BITS` |  | `16` | Bits in one UTF-16 code unit. |
