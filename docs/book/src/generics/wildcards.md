# Wildcards: `...`

In a parameter's type, `...` accepts any argument in that position without
naming it.

```basm
from std.array import Array

macro count(arr: Array<int, ...>) -> int {
    @return arr.len
}

macro show(value: int) {
    @emit value
}

show count(Array<int, 2> { __el0: 1, __el1: 2 })
show count(Array<int, 3> { __el0: 1, __el1: 2, __el2: 3 })
```

```emits
2 3
```

## `...` versus a named parameter

Both of these accept an array of any length:

```basm,ignore
macro count(arr: Array<int, ...>) -> int
macro count<const N: int>(arr: Array<int, N>) -> int
```

Use a named parameter when the macro needs the value *as a type argument*,
for example to declare its return type as `Array<int, N + 1>`. Use `...` when
it doesn't. The value is usually still available from the argument itself,
like `arr.len` above.

## In return types

`...` in a return type says the macro returns *some* instance of a generic
type, decided by its body. `std.bitfield`'s `field` returns a `bits<N>` whose
width depends on its arguments:

```basm,ignore
pub macro field(value: int, hi: int, lo: int) -> bits<...> {
    @return bits<hi - lo + 1> { value: slice(value, hi, lo) }
}
```

## Mixing

Wildcards and named parameters can be mixed: `Array<T, ...>` names the
element type and accepts any length. `std.array`'s `get`, `updated` and
`reversed` all take an `Array<T, ...>`.
