# `data`

[`std`](../index.md) › [`x86_64`](index.md) › `data`

Data for x86-64 programs: NASM's `db`, `dw`, `dd` and `dq`, shared by
`std.x86_64.nasm` and `std.x86_64.att`, since data is spelled the same
whichever syntax the instructions use.

```basm
from std.x86_64.data import *

db "Hi", 10
dw 0x1234
```

```bytes
48 69 0a
34 12
```

## Macros

### `data_int`

`value` as `width` little-endian bytes, truncated the way NASM does.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `data_int(value, width)` | `value: int`, `width: int` | returns `Bytes<...>` |  |

### `data_string`

A string literal's UTF-8 bytes, with no terminator.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `data_string(source)` | `<S>`, `source: S` | returns `Bytes<...>` |  |

### `resb`

`count` zero bytes, as one value: room for variables, like NASM's
`resb`. In a section that takes no room in the file, like ELF's `.bss`,
a link stage leaves them out of the file.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `resb count` | `count: int` |  |  |

### `db`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `db a` | `a: int` |  | Bytes. Each operand is an integer, stored in one byte, or a string literal, stored as its UTF-8 bytes with no terminator (add `, 0` yourself). Up to four operands; `db "Hello", 10` is the common case. |
| `db a` | `<S>`, `a: S` |  |  |
| `db a, b` | `<S>`, `a: S`, `b: int` |  |  |
| `db a, b, c` | `<S>`, `a: S`, `b: int`, `c: int` |  |  |
| `db a, b` | `a: int`, `b: int` |  |  |
| `db a, b, c` | `a: int`, `b: int`, `c: int` |  |  |
| `db a, b, c, d` | `a: int`, `b: int`, `c: int`, `d: int` |  |  |

### `dw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dw a` | `a: int` |  | 16-bit little-endian words, one or two per line. |
| `dw a, b` | `a: int`, `b: int` |  |  |

### `dd`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dd a` | `a: int` |  | 32-bit little-endian doublewords, one or two per line. |
| `dd a, b` | `a: int`, `b: int` |  |  |

### `dq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dq a` | `a: int` |  | 64-bit little-endian quadwords, one or two per line. |
| `dq a, b` | `a: int`, `b: int` |  |  |
