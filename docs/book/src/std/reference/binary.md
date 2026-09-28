# `binary`

[`std`](index.md) › `binary`

Fixed-width binary values: `bits<N>`, `signed<N>`, `bool`, and byte
order.

`bits<N>` is how std gives an `int` a width. Its value must fit in `N`
bits, which is checked when the value is built, so an encoder can't
silently emit a field that overflows.

```basm
from std.binary import *

macro show<T>(value: T) {
    @emit value
}

show bits<8> { value: 65 }
show bit_width(255)
```

```emits
bits<8> { value: 65 }
8
```

## Macros

### `signed_from_int`

`x` as a `signed<width>`. It must fit: from `-2^(width - 1)` up to
`2^(width - 1) - 1`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `signed_from_int(x, width)` | `x: int`, `width: int` | returns `signed<...>` |  |

### `signed_to_int`

The `int` a `signed<width>` holds, sign-extended from its top bit.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `signed_to_int(x)` | `<const width: int>`, `x: signed<width>` | returns `int` |  |

### `fits_inside_width`

1 if `value` fits in `width` bits (`0 <= value < 2^width`), else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fits_inside_width(width, value)` | `width: int`, `value: int` | returns `int` |  |

### `bit_width`

The number of bits needed to write `n`, which must not be negative. Zero
still takes one bit.

```basm
from std.binary import bit_width

macro show(value: int) {
    @emit value
}

show bit_width(0)
show bit_width(255)
show bit_width(256)
```

```emits
1 8 9
```

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bit_width(n)` | `n: int` | returns `int` |  |

### `byte_width`

The number of whole bytes needed to write `n`, which must not be
negative: `byte_width(255)` is 1 and `byte_width(256)` is 2.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `byte_width(n)` | `n: int` | returns `int` |  |

## Types

### `bits`

`struct bits<const width: int>`

An unsigned value `width` bits wide: `0 <= value < 2^width`.

```basm,fail
from std.binary import *

const too_big = bits<4> { value: 16 }
```

```error
invariant `fits_inside_width(width, value)` was violated for `bits`
```

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The value itself. `skip`, so `@for` over a `bits` visits nothing. |

### `signed`

`struct signed<const width: int>`

A signed value `width` bits wide, stored in two's complement:
`-2^(width - 1) <= value < 2^(width - 1)`. Convert an `int` to one with
`as`, and back the same way.

```basm
from std.binary import *

macro db(value: signed<8>) {
    @emit value
}

db (-1) as signed<8>
db 100 as signed<8>
```

```bytes
ff 64
```

```basm,fail
from std.binary import *

const too_small = (-129) as signed<8>
```

```error
value doesn't fit in the signed width
```

| Field | Type | Description |
|---|---|---|
| `bits` | `bits<width>` | The value's two's-complement bits: -1 is all ones. |

### `bool`

`type bool = bits<1>`

A single bit: `true` or `false`.

### `Endian`

`enum Endian`

Byte order, for code that can lay out values either way.

| Variant | Payload | Description |
|---|---|---|
| `Little` |  | Least significant byte first, as on x86 and RISC-V. |
| `Big` |  | Most significant byte first. |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `BITS_PER_BYTE` | `int` | `8` | The number of bits in a byte. |
| `true` | `bool` | `1` | `1` as a `bool`. |
| `false` | `bool` | `0` | `0` as a `bool`. |
