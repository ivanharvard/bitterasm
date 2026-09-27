# Returning values

A macro gives a value back to its caller with [`@return`](../meta/return.md):

```basm
macro max(a: int, b: int) -> int {
    @if a > b {
        @return a
    }
    @return b
}

macro show(value: int) {
    @emit value
}

show max(3, 9)
```

```emits
9
```

`-> int` declares the return type. It's optional, but it documents the macro,
and it's needed when the macro is passed as an argument (see
[Macros as parameters](../generics/macro-parameters.md)) or used as a
[conversion](../types/conversions.md).

> **Note:** The compiler doesn't yet check `@return` values against the
> declared return type.

## Returning versus emitting

Every macro has two separate outputs:

| | `@return` | `@emit` |
|---|---|---|
| Goes to | The caller, as the call's value | The program's output |
| How many | One value, or none | Any number |
| Ends the macro | Yes | No |

A macro may do both. A macro that only emits is like an instruction; a macro
that only returns is like a function.

## Where emitted values can go

A statement call puts the macro's emitted values in the output where the call
appears. A call used as an expression has nowhere to put emitted values
unless it's the *whole* of one of these:

- a `const`'s value: `const x = f()`
- `@return`'s value: `@return f()`

In both cases, the emitted values are kept, in order, where that statement
appears. The same goes for [`@fold`](../meta/fold.md) in expression position.

```basm
macro emit_and_return(x: int) -> int {
    @emit x
    @return x * 10
}

macro caller() {
    const y = emit_and_return(1)
    @emit y
}

caller
```

```emits
1 10
```

Anywhere else, such as inside a larger expression, calling a macro that
emits is an error, because its values would be lost:

```basm,fail
macro emit_and_return(x: int) -> int {
    @emit x
    @return x * 10
}

macro caller() {
    @emit 1 + emit_and_return(1)
}

caller
```

```error
`emit_and_return` emits values, so it can only be used as a statement
```

## Macros that return nothing

Using a macro that returns nothing as a value is an error. A bare `@return`
ends a macro early without a value.
