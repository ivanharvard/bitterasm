# `utf8`

[`std`](../index.md) › [`strings`](index.md) › `utf8`

UTF-8 strings, and `string_from_struct`, which encodes a string literal
as UTF-8 bytes packed into one `int`.

```basm
from std.strings.utf8 import *
from std.binary import Endian

macro show(value: int) {
    @emit value
}

macro demo() {
    const s = string_from_struct("héllo")
    show s.len
    show utf8_codepoint_count(s as Utf8String<6, Endian.Big>)
}

demo
```

```emits
6 5
```

`é` takes two bytes, so `"héllo"` is 6 bytes but 5 characters.
Converting to `Utf8String` with `as` checks the bytes are valid UTF-8,
and picks their `Endian`. The byte order doesn't change the UTF-8.

## Macros

### `utf8_struct_byte_len`

How many bytes a string literal (or any struct of code points) takes
as UTF-8.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf8_struct_byte_len(source)` | `<S>`, `source: S` | returns `int` |  |

### `string_from_struct`

A string literal (or any struct of code points) as a `String` of its
UTF-8 bytes, first character most significant.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `string_from_struct source` | `<S>`, `source: S` |  |  |

### `validate_utf8`

1 if `len` bytes packed into `value` are valid UTF-8: no stray or
missing continuation bytes, overlong forms, surrogates, or code points
past U+10FFFF. Otherwise it's a compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_utf8(value, len, endian)` | `value: int`, `len: int`, `endian: Endian` | returns `int` |  |

### `utf8_byte_at`

Byte `index` of `s`. This is a byte, not a character.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf8_byte_at(s, index)` | `s: Utf8String`, `index: int` | returns `int` |  |

### `utf8_codepoint_count`

The number of characters (code points) in `s`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf8_codepoint_count(s)` | `s: Utf8String` | returns `int` |  |

### `utf8_is_ascii`

1 if every byte of `s` is ASCII, else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf8_is_ascii(s)` | `s: Utf8String` | returns `int` |  |

## Types

### `Utf8String`

`struct Utf8String<const len: int, const endian: Endian>`

`len` bytes of valid UTF-8 packed into `value`, with byte 0 at the
`endian` end.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The bytes, packed. |
| `len` | `int` | The number of bytes. |
| `endian` | `Endian` | Which end of `value` holds byte 0. |
