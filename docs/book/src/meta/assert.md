# `@assert`

`@assert` stops compilation with an error if a condition is false.

## Syntax

```basm,ignore
@assert condition
@assert condition, "message"
```

The message is optional, and must be a string literal.

## Example

```basm
macro shift_amount(n: int) {
    @assert n >= 0 && n < 32, "shift amount must be 0 to 31"
    @emit n
}

shift_amount 5
```

```emits
5
```

```basm,fail
macro shift_amount(n: int) {
    @assert n >= 0 && n < 32, "shift amount must be 0 to 31"
    @emit n
}

shift_amount 40
```

```error
shift amount must be 0 to 31
```

## Details

- The condition is true if it's not `0`.
- A passing `@assert` emits nothing and has no other effect.
- The error points at the `@assert` that failed.

## `@error`

`@error "message"` always fails, with `message`. It's the same as
`@assert 0, "message"`, for the branch of an `@if` that's never allowed:

```basm,fail
macro width(bits: int) {
    @if bits == 8 || bits == 16 {
        @emit bits
    } @else {
        @error "only 8 and 16 bits are supported"
    }
}

width 12
```

```error
only 8 and 16 bits are supported
```

## Asserts, hooks and invariants

An `@assert` checks something at one point in one macro. For checks that
apply more broadly:

- To check the arguments of several macros the same way, put the asserts in
  one macro and call it from a [`before` hook](../macros/hooks.md).
- To check every value of a type, wherever it's made, use an
  [invariant](../types/invariants.md).
