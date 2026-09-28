# `string`

[`std`](index.md) › `string`

Strings packed into one integer. A string literal like `"hi"` is a
struct of code points; `string_from_struct` encodes it as UTF-8 bytes in a
single `int`, with its length in bytes.

```basm
from std.string import *
from std.binary import Endian

macro show(value: int) {
    @emit value
}

macro demo() {
    const s = string_from_struct("héllo")
    show s.len
    show utf8_codepoint_count(s as Utf8String<6, Endian.Big>)
    show ascii_upper(string_from_struct("hi") as AsciiString<2, Endian.Big>).value
}

demo
```

```emits
6 5 18505
```

`18505` is `0x4849`, the bytes of `HI`. Converting to `AsciiString` or
`Utf8String` with `as` checks the bytes are valid, and picks their
`Endian`: whether byte 0 is the most (`Big`) or least (`Little`)
significant byte of `value`. The byte order doesn't change the UTF-8.

## Macros

### `byte_at`

Byte `index` of `len` bytes packed into `value`, counting from the
`endian` end. `index` must be within the string.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `byte_at(value, len, index, endian)` | `value: int`, `len: int`, `index: int`, `endian: Endian` | returns `int` |  |

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

### `validate_ascii`

1 if `len` bytes packed into `value` are all ASCII. Otherwise it's a
compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_ascii(value, len, endian)` | `value: int`, `len: int`, `endian: Endian` | returns `int` |  |

### `validate_utf8`

1 if `len` bytes packed into `value` are valid UTF-8: no stray or
missing continuation bytes, overlong forms, surrogates, or code points
past U+10FFFF. Otherwise it's a compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_utf8(value, len, endian)` | `value: int`, `len: int`, `endian: Endian` | returns `int` |  |

### `ascii_byte_at`

Byte `index` of `s`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ascii_byte_at(s, index)` | `s: AsciiString`, `index: int` | returns `int` |  |

### `utf8_byte_at`

Byte `index` of `s`. This is a byte, not a character.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `utf8_byte_at(s, index)` | `s: Utf8String`, `index: int` | returns `int` |  |

### `ascii_upper`

`s` with `a`-`z` made uppercase.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ascii_upper(s)` | `s: AsciiString` | returns `AsciiString` |  |

### `ascii_lower`

`s` with `A`-`Z` made lowercase.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ascii_lower(s)` | `s: AsciiString` | returns `AsciiString` |  |

### `ascii_title`

`s` with its first byte made uppercase.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ascii_title(s)` | `s: AsciiString` | returns `AsciiString` |  |

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

### `String`

`struct String<const len: int>`

`len` bytes packed into `value`, first byte most significant.
`string_from_struct` builds one. Convert it with `as` to `AsciiString` or
`Utf8String` to check its bytes and choose their order.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The bytes, packed. |
| `len` | `int` | The number of bytes. |

### `AsciiString`

`struct AsciiString<const len: int, const endian: Endian>`

`len` ASCII bytes (each below `0x80`) packed into `value`, with byte 0 at
the `endian` end.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The bytes, packed. |
| `len` | `int` | The number of bytes. |
| `endian` | `Endian` | Which end of `value` holds byte 0. |

### `Utf8String`

`struct Utf8String<const len: int, const endian: Endian>`

`len` bytes of valid UTF-8 packed into `value`, with byte 0 at the
`endian` end.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The bytes, packed. |
| `len` | `int` | The number of bytes. |
| `endian` | `Endian` | Which end of `value` holds byte 0. |
