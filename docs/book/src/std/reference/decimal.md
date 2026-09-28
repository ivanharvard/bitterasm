# `decimal`

[`std`](index.md) › `decimal`

Exact non-integer numbers, for compile-time math: `Decimal`, a value
scaled by a power of ten, and `Fraction`, a numerator over a denominator.
Both convert to and from `int` with `as`.

```basm
from std.decimal import *

macro show(value: int) {
    @emit value
}

const half = Fraction(n = 1, d = 2)
const d = fraction_to_decimal(half, 3)
show d.value
show d.scale
show Decimal(value = 42, scale = 0) as int
```

```emits
500 3 42
```

## Macros

### `decimal_to_int`

`x` as an `int`. `x.scale` must be 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decimal_to_int(x)` | `x: Decimal` | returns `int` |  |

### `decimal_from_int`

`x` as a `Decimal` with scale 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decimal_from_int(x)` | `x: int` | returns `Decimal` |  |

### `fraction_to_int`

`x` as an `int`. `x.d` must be 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fraction_to_int(x)` | `x: Fraction` | returns `int` |  |

### `fraction_from_int`

`x` over 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fraction_from_int(x)` | `x: int` | returns `Fraction` |  |

### `decimal_to_fraction`

`x` as `x.value` over `10^x.scale`, not reduced.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decimal_to_fraction(x)` | `x: Decimal` | returns `Fraction` |  |

### `fraction_to_decimal`

`x` as a `Decimal` with `precision` decimal places, rounded toward
zero.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fraction_to_decimal(x, precision)` | `x: Fraction`, `precision: int` | returns `Decimal` |  |

## Types

### `Infinity`

`enum Infinity`

A signed infinity.

| Variant | Payload | Description |
|---|---|---|
| `Positive` |  | Positive infinity. |
| `Negative` |  | Negative infinity. |

### `Decimal`

`struct Decimal`

`value / 10^scale`: `Decimal(value = 314, scale = 2)` is 3.14. `scale`
can't be negative. `as int` works when `scale` is 0.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The digits, as an integer. |
| `scale` | `int` | How many of those digits come after the decimal point. |

### `Fraction`

`struct Fraction`

`n / d`, where `d` isn't 0. `as int` works when `d` is 1, and `as
Decimal` keeps 10 decimal places.

| Field | Type | Description |
|---|---|---|
| `n` | `int` | The numerator. |
| `d` | `int` | The denominator. |
