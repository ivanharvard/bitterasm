# Macros as parameters

A macro can take another macro as an argument, and call it.

```basm
macro double(x: int) -> int {
    @return x * 2
}

macro apply_twice<F: Fn(int) -> int>(f: F, x: int) -> int {
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

## Syntax

```basm,ignore
macro name<F: Fn(ParamType, ...) -> ReturnType>(f: F, ...) { ... }
```

- `F` is an ordinary type parameter, with a **bound**: `Fn(int) -> int`
  describes the signature a macro must have to be passed as `F`.
- The parameter `f: F` receives the macro. Call it like any macro: `f(x)`.
- Pass a macro by its name, with no parentheses: `apply_twice(double, 5)`.
- `-> ReturnType` can be left off: `Fn(int)`.

## The bound is checked at the call

Passing a macro whose signature doesn't match is an error at the call site,
before the body runs:

```basm,fail
macro add(x: int, y: int) -> int {
    @return x + y
}

macro apply<F: Fn(int) -> int>(f: F, x: int) -> int {
    @return f(x)
}

macro show(value: int) {
    @emit value
}

show apply(add, 5)
```

```error
expected `Fn(int) -> int`, found `Fn(int, int) -> int`
```

## Example: mapping an array

`std.array`'s `mapped` applies a macro to every element:

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

show_all mapped(Array<int, 3> { __el0: 1, __el1: 2, __el2: 3 }, double)
```

```emits
2 4 6
```

Its declaration reads:

```basm,ignore
pub macro mapped<T, U, F: Fn(T) -> U>(arr: Array<T, ...>, f: F) -> Array<U, ...>
```

## Limits

- Only a non-generic macro can be passed.
- `Fn(...)` is the only kind of bound. There are no traits or interfaces.
- A macro is a compile-time value only. It can be passed around and called,
  but never emitted.
