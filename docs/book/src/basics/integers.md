# Integers

`int` is BitterASM's only built-in type. An `int` is a mathematical integer:
it has **arbitrary precision** and no width, so it never overflows or wraps.

```basm
macro show(value: int) {
    @emit value
}

show 1_000_000 * 1_000_000 * 1_000_000 * 1_000_000
```

```emits
1000000000000000000000000
```

`3` is just the number three, with no bits attached. Bits, bytes, signed and
unsigned values are all library types built on `int`: see `bits<N>` in
[Packing bytes with `bitter`](../output/bitter.md), and
[Types](../types/index.md) for how to build types of your own.

## Literals

| Form | Example | Value |
|---|---|---|
| Decimal | `42` | 42 |
| Hexadecimal | `0xff` | 255 |
| Binary | `0b1010` | 10 |
| Octal | `0o17` | 15 |
| Character | `'A'` | 65 |

Any literal may use `_` between digits for readability: `1_000_000`,
`0xffff_0000`, `0b1010_0101`.

```basm
macro show(value: int) {
    @emit value
}

show 42
show 0xff
show 0b1010
show 0o17
show 0xff_ff
```

```emits
42 255 10 15 65535
```

## Negative numbers

There are no negative literals. `-5` is the negation operator applied to `5`,
which gives the same result:

```basm
macro show(value: int) {
    @emit value
}

show -5
show -(2 + 3)
```

```emits
-5 -5
```

## True and false

There's no built-in boolean. Conditions are integers: `0` is false and
anything else is true. Comparisons and logical operators produce `0` or `1`.
See [Operators](operators.md).

`std.binary` does define a `bool` type (a one-bit `bits<1>`) with `true` and
`false` constants, for code that needs a boolean it can emit.

## No floating point

There are no fractional numbers in the language. `std.decimal` provides
`Decimal` and `Fraction` types, and `std.math` provides fixed-point
arithmetic, square roots, logarithms and trigonometry over them.
