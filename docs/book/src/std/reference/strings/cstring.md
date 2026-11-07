# `cstring`

[`std`](../index.md) › [`strings`](index.md) › `cstring`

NUL-terminated strings, the way C (and every API built on it) expects
them: the characters, then one zero code unit. `nul_terminated` works on
any encoding and returns the same string type, one unit longer. A zero
unit is one byte for ASCII and UTF-8, and two bytes for UTF-16, as in
Windows' wide strings.

```basm
from std.strings.cstring import nul_terminated
from std.strings.ascii import AsciiString
from std.strings.utf8 import string_from_struct
from std.strings.utf16 import utf16_from_struct
from std.binary import Endian

macro show(value: int) {
    @emit value
}

macro demo() {
    const s = nul_terminated(string_from_struct("hi") as AsciiString<2, Endian.Big>)
    show s.value
    show s.len
    show nul_terminated(utf16_from_struct("hi")).len
}

demo
```

```emits
6842624 3 3
```

`6842624` is `0x686900`: `h`, `i`, then the NUL. A string that already
holds a NUL is a compile error, since C would stop reading at it.

## Macros

### `nul_terminated`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `nul_terminated(s)` | `s: AsciiString` | returns `AsciiString` | `s` followed by a NUL byte. |
| `nul_terminated(s)` | `s: Utf8String` | returns `Utf8String` | `s` followed by a NUL byte. |
| `nul_terminated(s)` | `s: Utf16String` | returns `Utf16String` | `s` followed by a NUL code unit (two zero bytes). |
