# Macros as parameters

A macro can take another macro as an argument, and call it.

```basm
macro double(x: int) -> int {
    @return x * 2
}

macro apply_twice(f: macro(int) -> int, x: int) -> int {
    @return f(f(x))
}

macro show(value: int) {
    @emit value
}

show apply_twice(double, 5)
```

```emits
20
```

## Macro types

`macro(int) -> int` is a **macro type**: the type of any macro that takes one
`int` and returns an `int`.

```basm,ignore
f: macro(ParamType, ...) -> ReturnType
```

- List the parameter types in parentheses. `macro()` takes none.
- `macro(int)`, with no `->`, is a macro that returns nothing. Only a macro
  declared without `->` matches it.
- Pass a macro by its name, with no parentheses: `apply_twice(double, 5)`.
- Call the parameter like any macro: `f(x)`.

A macro type is compile-time machinery, not data. See
[Data types and compile-time types](../types/index.md#data-types-and-compile-time-types).

## The type is checked at the call

Passing a macro whose signature doesn't match is an error at the call site,
before the body runs:

```basm,fail
macro add(x: int, y: int) -> int {
    @return x + y
}

macro apply(f: macro(int) -> int, x: int) -> int {
    @return f(x)
}

macro show(value: int) {
    @emit value
}

show apply(add, 5)
```

```error
expected `macro(int) -> int`, found `macro(int, int) -> int`
```

## Macros that return nothing

A macro that only emits is a useful argument too. Call it as a statement:

```basm
macro show(value: int) {
    @emit value
}

macro each_of_three(f: macro(int)) {
    f(1)
    f(2)
    f(3)
}

each_of_three show
```

```emits
1 2 3
```

## Generic macro types

A macro type can mention the macro's own generic parameters. They're
inferred from the macro that's passed. `std.array`'s `mapped` applies a macro
to every element:

```basm
from std.array import Array, mapped

macro double(x: int) -> int {
    @return x * 2
}

macro show_all<const N: int>(values: Array<int, N>) {
    @for v in values {
        @emit v
    }
}

show_all mapped(Array<int, 3>(1, 2, 3), double)
```

```emits
2 4 6
```

Its declaration reads:

```basm,ignore
pub macro mapped<T, U>(arr: Array<T, ...>, f: macro(T) -> U) -> Array<U, ...>
```

Passing `double` sets `T` and `U` to `int`.

## Limits

- Only a macro that is neither generic nor overloaded can be passed.
- A macro can't be emitted.
