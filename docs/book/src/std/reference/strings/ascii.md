# `ascii`

[`std`](../index.md) › [`strings`](index.md) › `ascii`

ASCII strings: one byte per character, each below `0x80`.

```basm
from std.strings.ascii import *
from std.strings.utf8 import string_from_struct
from std.binary import Endian

macro show(value: int) {
    @emit value
}

macro demo() {
    const s = string_from_struct("hi") as AsciiString<2, Endian.Big>
    show ascii_upper(s).value
    show ascii_byte_at(s, 1)
}

demo
```

```emits
18505 105
```

`18505` is `0x4849`, the bytes of `HI`. Converting a `String` to
`AsciiString` with `as` checks every byte is ASCII.

## Macros

### `validate_ascii`

1 if `len` bytes packed into `value` are all ASCII. Otherwise it's a
compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `validate_ascii(value, len, endian)` | `value: int`, `len: int`, `endian: Endian` | returns `int` |  |

### `ascii_byte_at`

Byte `index` of `s`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ascii_byte_at(s, index)` | `s: AsciiString`, `index: int` | returns `int` |  |

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

## Types

### `AsciiString`

`struct AsciiString<const len: int, const endian: Endian>`

`len` ASCII bytes (each below `0x80`) packed into `value`, with byte 0 at
the `endian` end.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The bytes, packed. |
| `len` | `int` | The number of bytes. |
| `endian` | `Endian` | Which end of `value` holds byte 0. |
