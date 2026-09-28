# `bitfield`

[`std`](index.md) › `bitfield`

Bit ranges, written the way ISA manuals write them. `slice(x, 10, 5)` is
the manual's `x[10:5]`: `hi` and `lo` are inclusive bit numbers, so a
one-bit field is `(n, n)`. The shift, the mask and the result's width all
come from those two numbers, so they can't disagree.

```basm
from std.bitfield import *

macro show(value: int) {
    @emit value
}

show slice(0b1101_0110, 7, 4)
show place(3, 7, 6) | place(2, 5, 3) | place(1, 2, 0)
```

```emits
13 209
```

`slice` and `field` also take a `Deferred` value, such as a branch offset
`bitter` works out later, and give back a `Deferred` or `Positioned<N>`.

## Macros

### `mask`

`width` one-bits: `mask(5)` is `0b11111`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mask(width)` | `width: int` | returns `int` |  |

### `bit`

Bit `n` of `value`, as 0 or 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bit(value, n)` | `value: int`, `n: int` | returns `int` |  |

### `slice`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `slice(value, hi, lo)` | `value: int`, `hi: int`, `lo: int` | returns `int` | Bits `hi` down to `lo` of `value`, shifted down to bit 0: the manuals' `value[hi:lo]`. |
| `slice(value, hi, lo)` | `value: Deferred`, `hi: int`, `lo: int` | returns `Deferred` | Bits `hi` down to `lo` of a value `bitter` works out later. |

### `field`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `field(value, hi, lo)` | `value: int`, `hi: int`, `lo: int` | returns `bits<...>` | `slice`, as the `bits<hi - lo + 1>` a format struct's field holds: `imm10_5: field(imm, 10, 5)`. |
| `field(value, hi, lo)` | `value: Deferred`, `hi: int`, `lo: int` | returns `Positioned<...>` | `slice` of a value `bitter` works out later, as a `Positioned<hi - lo + 1>`: `field(offset, 10, 5)` in place of `Positioned<6> { value: band(shr(offset, 5), 0b111111) }`. |

### `truncate`

The low `width` bits of `value`, as a `bits<width>`. It truncates rather
than rejecting, so a negative immediate becomes its two's-complement
encoding: `truncate(-1, 12)` is `0xFFF`, where `bits<12> { value: -1 }`
would fail `bits`'s range check.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `truncate(value, width)` | `value: int`, `width: int` | returns `bits<...>` |  |

### `place`

`value`'s low `hi - lo + 1` bits, moved up to bits `hi` down to `lo`:
`slice`'s inverse, for building a word out of fields with `|`.
`place(mod, 7, 6) | place(reg, 5, 3) | place(rm, 2, 0)` is a ModRM byte.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `place(value, hi, lo)` | `value: int`, `hi: int`, `lo: int` | returns `int` |  |
