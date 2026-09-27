# Const parameters

A const parameter puts a compile-time value into a type. `bits<8>` and
`bits<16>` are different types, because their `width` differs.

## Syntax

```basm,ignore
struct Name<const N: int> { ... }
macro name<const N: int>(x: Type<N>) { ... }
```

A const parameter's type can be `int` or an [enum](../types/enums.md):

```basm
from std.binary import Endian

struct Word<const width: int, const order: Endian> {
    pub value: int,
}

macro emit_word(w: Word<16, Endian.Big>) {
    @emit w
}

emit_word Word<16, Endian.Big> { value: 1 }
```

```emits
Word<16, 1> { value: 1 }
```

In the output, an enum argument is recorded as its variant's position in the
enum: `Endian.Big` is `1`, because it's `Endian`'s second variant.

## Using the value

Inside the declaration, a const parameter is an ordinary value. A struct
can use it in its fields, defaults and invariants, and a macro can use it in
its body:

```basm
from std.binary import bits

macro width_of<const W: int>(_b: bits<W>) -> int {
    @return W
}

macro show(value: int) {
    @emit value
}

show width_of(3 as bits<12>)
```

```emits
12
```

## Computing types

A type argument can be any expression, including one built from other
parameters. `std.array`'s `appended` returns an array one element longer
than its argument:

```basm,ignore
pub macro appended<T, const N: int>(
    arr: Array<T, N>,
    value: T
) -> Array<T, N + 1>
```

```basm
from std.binary import Endian

struct Word<const width: int, const order: Endian> {
    pub value: int,
}

macro widen<const N: int>(w: Word<N, Endian.Big>) -> Word<N * 2, Endian.Big> {
    @return Word<N * 2, Endian.Big> { value: w.value }
}

macro emit_word(w: Word<32, Endian.Big>) {
    @emit w
}

emit_word widen(Word<16, Endian.Big> { value: 3 })
```

```emits
Word<32, 1> { value: 3 }
```

## Generating fields from a parameter

A struct's fields can depend on its const parameters through
[`@for` and `@if`](../types/structs.md#generating-fields). That's how
`Array<T, N>` has exactly `N` elements.
