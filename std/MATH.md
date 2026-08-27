# `std.math`

`std.math` supplies arbitrary-precision integer operations and real fixed-point
operations over `Decimal`. Transcendental functions accept a trailing
`precision: int = 16`, compute with four guard digits, and truncate to the
requested number of decimal places. They are deterministic approximations, not
correctly-rounded floating-point operations.

`PI`, `TAU`, `E`, `SQRT2`, `LN2`, and `LN10` are macros rather than fixed values:
`PI()` uses 16 places and `PI(40)` requests 40. Embedded constant data supports
0 through 60 places.

## Domains and conventions

- Integer `pow` requires a nonnegative exponent and defines `0 ** 0` as `1`.
- Decimal-to-Decimal `pow` requires a positive base. The Decimal/integer
  overload permits negative bases and negative exponents except `0` to a
  negative exponent.
- `root` requires a positive degree. Negative radicands require an odd degree.
- `isqrt` rejects negative inputs.
- `gcd(0, 0) = 0`; `lcm` is nonnegative and is zero if either input is zero.
- `pow_mod` requires a nonnegative exponent and positive modulus.
- `next_pow_of_two(0) = 1`; `is_pow_of_two` is false for nonpositive inputs.
- `popcount` and `ctz` require nonnegative integers; `ctz(0) = 0`.
- `clz` and rotations take an explicit width, and the value must fit it.
- `ln`, `log2`, and `log10` require positive inputs; `log1p(x)` requires
  `1 + x > 0` through the same check.
- `asin` and `acos` accept `[-1, 1]`; `atan2(0, 0)` is undefined.
- `acosh` accepts `[1, infinity)` and `atanh` accepts `(-1, 1)`.

All domain violations use `@assert`. Integer division helpers use mathematical
floor/ceiling or Euclidean semantics independently of BitterASM's truncating
`/` and `%` operators.
