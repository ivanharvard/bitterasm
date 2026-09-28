# `math`

[`std`](index.md) › `math`

Integer math, and fixed-point math on `Decimal` and `Fraction`, all
evaluated at compile time.

```basm
from std.math import *

macro show(value: int) {
    @emit value
}

macro demo() {
    show gcd(12, 18)
    show div_floor(-7, 2)
    show sqrt(2, 4).value
    show sin(PI(10), 6).value
}

demo
```

```emits
6 -4 14142 0
```

Functions that give a `Decimal` take a `precision`: the number of decimal
places to keep, `DEFAULT_PRECISION` unless given. Results are truncated
to that many places, so `sqrt(2, 4)` is 1.4142. Angles are in radians.

Re-exports [`std.decimal`](decimal.md).

## Macros

### `abs`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `abs(x)` | `x: int` | returns `int` | The absolute value of `x`. |
| `abs(x)` | `x: Decimal` | returns `Decimal` | The absolute value of `x`. |
| `abs(x)` | `x: Fraction` | returns `Fraction` | The absolute value of `x`. |

### `min`

The smaller of `a` and `b`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `min(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `max`

The larger of `a` and `b`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `max(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `sign`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sign(x)` | `x: int` | returns `int` | -1, 0 or 1, as `x` is negative, zero or positive. |
| `sign(x)` | `x: Decimal` | returns `int` | -1, 0 or 1, as `x` is negative, zero or positive. |
| `sign(x)` | `x: Fraction` | returns `int` | -1, 0 or 1, as `x` is negative, zero or positive. |

### `copysign`

`abs(x)` with the sign of `y`: `abs(x) * sign(y)`, so 0 when `y` is 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `copysign(x, y)` | `x: int`, `y: int` | returns `int` |  |

### `clamp`

`x`, moved into `lo..=hi` if it's outside. `lo` can't be more than `hi`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clamp(x, lo, hi)` | `x: int`, `lo: int`, `hi: int` | returns `int` |  |

### `pow`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pow(base, exponent)` | `base: int`, `exponent: int` | returns `int` | `base` to the power `exponent`, which can't be negative. |
| `pow(base, exponent, precision)` | `base: Decimal`, `exponent: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` | `base` to the power `exponent`. `base` must be positive. |
| `pow(base, exponent, precision)` | `base: Decimal`, `exponent: int`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` | `base` to an integer power. A negative `exponent` needs a nonzero `base`. |

### `gcd`

The greatest common divisor of `a` and `b`, never negative. `gcd(0, 0)` is 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `gcd(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `lcm`

The least common multiple of `a` and `b`, never negative.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lcm(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `div_floor`

`a / b` rounded down: `div_floor(-7, 2)` is -4, where `/` gives -3.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `div_floor(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `div_ceil`

`a / b` rounded up: `div_ceil(7, 2)` is 4.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `div_ceil(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `rem_euclid`

The remainder of `a / b` that's never negative: `rem_euclid(-7, 3)` is 2,
where `%` gives -1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rem_euclid(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `div_euclid`

The quotient that goes with `rem_euclid`, so
`div_euclid(a, b) * b + rem_euclid(a, b) == a`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `div_euclid(a, b)` | `a: int`, `b: int` | returns `int` |  |

### `pow_mod`

`base^exponent % modulus`, without computing `base^exponent` in full.
`exponent` can't be negative, and `modulus` must be positive.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pow_mod(base, exponent, modulus)` | `base: int`, `exponent: int`, `modulus: int` | returns `int` |  |

### `isqrt`

The square root of `x`, rounded down. `x` can't be negative.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `isqrt(x)` | `x: int` | returns `int` |  |

### `popcount`

How many bits of `x` are 1. `x` can't be negative.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `popcount(x)` | `x: int` | returns `int` |  |

### `ctz`

How many 0 bits come below `x`'s lowest 1 bit. `ctz(0)` is 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ctz(x)` | `x: int` | returns `int` |  |

### `clz`

How many 0 bits come above `x`'s highest 1 bit, in a `width`-bit value.
`x` must fit in `width` bits.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clz(x, width)` | `x: int`, `width: int` | returns `int` |  |

### `rotate_left`

`x` rotated left by `amount` bits within a `width`-bit value: bits
shifted out at the top come back in at the bottom.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rotate_left(x, amount, width)` | `x: int`, `amount: int`, `width: int` | returns `int` |  |

### `rotate_right`

`x` rotated right by `amount` bits within a `width`-bit value: bits
shifted out at the bottom come back in at the top.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rotate_right(x, amount, width)` | `x: int`, `amount: int`, `width: int` | returns `int` |  |

### `is_pow_of_two`

1 if `x` is a power of two, else 0. 0 isn't one.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `is_pow_of_two(x)` | `x: int` | returns `int` |  |

### `next_pow_of_two`

The smallest power of two that's at least `x`: `next_pow_of_two(5)` is 8,
and `next_pow_of_two(0)` is 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `next_pow_of_two(x)` | `x: int` | returns `int` |  |

### `floor`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `floor(x)` | `x: int` | returns `int` | The largest integer not above `x`. |
| `floor(x)` | `x: Decimal` | returns `int` |  |
| `floor(x)` | `x: Fraction` | returns `int` |  |

### `ceil`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ceil(x)` | `x: int` | returns `int` | The smallest integer not below `x`. |
| `ceil(x)` | `x: Decimal` | returns `int` |  |
| `ceil(x)` | `x: Fraction` | returns `int` |  |

### `trunc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `trunc(x)` | `x: int` | returns `int` | `x` with its fractional part dropped, rounding toward zero. |
| `trunc(x)` | `x: Decimal` | returns `int` |  |
| `trunc(x)` | `x: Fraction` | returns `int` |  |

### `round`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `round(x)` | `x: int` | returns `int` | The nearest integer to `x`. Halves round away from zero: 2.5 becomes 3, and -2.5 becomes -3. |
| `round(x)` | `x: Decimal` | returns `int` |  |
| `round(x)` | `x: Fraction` | returns `int` |  |

### `fract`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fract(x)` | `x: Decimal` | returns `Decimal` | `x - floor(x)`, always between 0 and 1: `fract(-2.5)` is 0.5. |
| `fract(x)` | `x: Fraction` | returns `Fraction` |  |

### `PI`

π to `precision` decimal places, at most 60.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `PI(precision)` | `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `TAU`

τ = 2π to `precision` decimal places, at most 60.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `TAU(precision)` | `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `E`

e to `precision` decimal places, at most 60.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `E(precision)` | `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `SQRT2`

√2 to `precision` decimal places, at most 60.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `SQRT2(precision)` | `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `LN2`

ln 2 to `precision` decimal places, at most 60.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `LN2(precision)` | `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `LN10`

ln 10 to `precision` decimal places, at most 60.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `LN10(precision)` | `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `root`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `root(x, degree, precision)` | `x: Decimal`, `degree: int`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` | The `degree`th root of `x`. `degree` must be positive, and a negative `x` needs an odd `degree`: `root(-8, 3)` is -2. |
| `root(x, degree, precision)` | `x: int`, `degree: int`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |
| `root(x, degree, precision)` | `x: Fraction`, `degree: int`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `sqrt`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sqrt(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` | The square root of `x`, which can't be negative. |
| `sqrt(x, precision)` | `x: int`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `hypot`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `hypot(x, y, precision)` | `x: Decimal`, `y: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` | `sqrt(x^2 + y^2)`, the length of the hypotenuse. |
| `hypot(x, y, precision)` | `x: int`, `y: int`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `exp`

e to the power `x`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `exp(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `exp2`

2 to the power `x`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `exp2(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `expm1`

`exp(x) - 1`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `expm1(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `ln`

The natural logarithm of `x`, which must be positive.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ln(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `log2`

The base-2 logarithm of `x`, which must be positive.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `log2(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `log10`

The base-10 logarithm of `x`, which must be positive.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `log10(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `log1p`

`ln(1 + x)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `log1p(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `sin`

The sine of `x` radians.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sin(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `cos`

The cosine of `x` radians.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cos(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `tan`

The tangent of `x` radians. It's an error where the cosine is 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `tan(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `atan`

The arctangent of `x`, in radians from -π/2 to π/2.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `atan(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `atan2`

The angle of the point (`x`, `y`) from the positive x axis, in radians
from -π to π, using both signs to pick the quadrant. `atan2(0, 0)` is an
error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `atan2(y, x, precision)` | `y: Decimal`, `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `asin`

The arcsine of `x`, in radians. `x` must be between -1 and 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `asin(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `acos`

The arccosine of `x`, in radians. `x` must be between -1 and 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `acos(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `sinh`

The hyperbolic sine of `x`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sinh(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `cosh`

The hyperbolic cosine of `x`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cosh(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `tanh`

The hyperbolic tangent of `x`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `tanh(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `asinh`

The inverse hyperbolic sine of `x`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `asinh(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `acosh`

The inverse hyperbolic cosine of `x`, which must be at least 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `acosh(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `atanh`

The inverse hyperbolic tangent of `x`, which must be between -1 and 1,
exclusive.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `atanh(x, precision)` | `x: Decimal`, `precision: int = DEFAULT_PRECISION` | returns `Decimal` |  |

### `simplify`

`x` in lowest terms, with a positive denominator: 6/-4 becomes -3/2.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `simplify(x)` | `x: Fraction` | returns `Fraction` |  |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `DEFAULT_PRECISION` | `int` | `16` | The number of decimal places a `Decimal` result keeps when no `precision` is given. |
| `GUARD_DIGITS` | `int` | `4` | Extra decimal places used inside a calculation, and dropped from its result. |
| `CONSTANT_DIGITS` | `int` | `60` | The most decimal places `PI`, `E` and the other constants can give. |

## Re-exported

From [`std.decimal`](decimal.md): `Decimal`, `Fraction`.
