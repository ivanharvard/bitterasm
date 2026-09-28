# `@return`

`@return` ends a macro and, optionally, gives a value back to the caller.

## Syntax

```basm,ignore
@return value
@return
```

## Example

```basm
macro clamp(x: int, lo: int, hi: int) -> int {
    @if x < lo {
        @return lo
    }
    @if x > hi {
        @return hi
    }
    @return x
}

macro show(value: int) {
    @emit value
}

show clamp(-5, 0, 10)
show clamp(50, 0, 10)
show clamp(7, 0, 10)
```

```emits
0 10 7
```

## Details

- `@return` stops the macro at once, even from inside `@if`, `@match`,
  `@for` or `@fold`.
- A bare `@return` ends the macro without a value.
- Values the macro emitted before returning stay emitted.
- `@return f(...)`, where `f` is the macro itself, is a *tail call*: it
  doesn't count toward the recursion limit. See
  [Recursion](../macros/recursion.md).
- `@return` can return the value of a call that emits, and its emitted values
  are kept. See [Returning values](../macros/returning.md).

## Returning early

```basm
macro first_multiple_of(k: int, limit: int) -> int {
    @for i in 1..limit {
        @if i % k == 0 {
            @return i
        }
    }
    @return -1
}

macro show(value: int) {
    @emit value
}

show first_multiple_of(7, 100)
show first_multiple_of(700, 100)
```

```emits
7 -1
```
