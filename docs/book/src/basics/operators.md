# Operators

## Arithmetic

| Operator | Meaning | Example | Result |
|---|---|---|---|
| `+` | Addition | `7 + 2` | `9` |
| `-` | Subtraction | `7 - 2` | `5` |
| `*` | Multiplication | `7 * 2` | `14` |
| `/` | Division, rounding toward zero | `-7 / 2` | `-3` |
| `%` | Remainder, with the sign of the left side | `-7 % 2` | `-1` |
| `-x` | Negation | `-7` | `-7` |

```basm
macro show(value: int) {
    @emit value
}

show 7 / 2
show -7 / 2
show -7 % 2
```

```emits
3 -3 -1
```

For floor, ceiling or Euclidean division, use `std.math`.

## Bitwise

Integers have no width, so bitwise operators act as if every number had
infinitely many bits, in two's complement. Negative numbers have infinitely
many leading ones.

| Operator | Meaning | Example | Result |
|---|---|---|---|
| `&` | And | `0b1100 & 0b1010` | `8` |
| `\|` | Or | `0b1100 \| 0b1010` | `14` |
| `^` | Exclusive or | `0b1100 ^ 0b1010` | `6` |
| `~x` | Not | `~0` | `-1` |
| `<<` | Shift left | `1 << 4` | `16` |
| `>>` | Shift right, keeping the sign | `-16 >> 2` | `-4` |

```basm
macro show(value: int) {
    @emit value
}

show 0b1100 & 0b1010
show 0b1100 | 0b1010
show 0b1100 ^ 0b1010
show ~0
show 1 << 4
show -16 >> 2
```

```emits
8 14 6 -1 16 -4
```

## Comparison

`==`, `!=`, `<`, `<=`, `>` and `>=` produce `1` for true and `0` for false.
`==` and `!=` also compare structs and enums, field by field.

```basm
macro show(value: int) {
    @emit value
}

show 2 < 3
show 2 == 3
```

```emits
1 0
```

## Logic

| Operator | Meaning |
|---|---|
| `&&` | `1` if both sides are true, else `0` |
| `\|\|` | `1` if either side is true, else `0` |
| `!x` | `1` if `x` is `0`, else `0` |

Any nonzero value counts as true, and the result is always `0` or `1`:

```basm
macro show(value: int) {
    @emit value
}

show 3 && 4
show 0 || 7
show !5
```

```emits
1 1 0
```

## Precedence

From tightest to loosest binding:

| Level | Operators |
|---|---|
| Postfix | `.field`, calls `f(x)`, `as` |
| Prefix | `-x`, `!x`, `~x` |
| Multiplicative | `*`, `/`, `%` |
| Additive | `+`, `-` |
| Shift | `<<`, `>>` |
| Comparison | `<`, `<=`, `>`, `>=`, `in` |
| Equality | `==`, `!=` |
| Bitwise and | `&` |
| Bitwise xor | `^` |
| Bitwise or | `\|` |
| Logical and | `&&` |
| Logical or | `\|\|` |
| Range | `..`, `..=` |

Operators on the same level group left to right. Use parentheses whenever
the grouping isn't obvious, especially when mixing bitwise and comparison
operators:

```basm
macro show(value: int) {
    @emit value
}

show 1 << 4 | 1
show (6 & 3) == 2
```

```emits
17 1
```

Since `as` binds tightly, `a + b as T` means `a + (b as T)`, and
`-1 as T` means `-(1 as T)`. Write `(a + b) as T` and `(-1) as T`. See
[Conversions](../types/conversions.md).

```basm,fail
from std.ctypes import int8_t

macro emit_byte(b: int8_t) {
    @emit b
}

emit_byte -1 as int8_t
```

```error
`as` binds more tightly than `-`, so `-x as T` means `-(x as T)`; write `(-x) as T`
```
