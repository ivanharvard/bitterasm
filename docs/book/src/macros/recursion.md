# Recursion

A macro can call itself, directly or through other macros:

```basm
macro factorial(n: int) -> int {
    @if n <= 1 {
        @return 1
    }
    @return n * factorial(n - 1)
}

macro show(value: int) {
    @emit value
}

show factorial(20)
```

```emits
2432902008176640000
```

## Limits

Since macros run inside the compiler, a runaway recursion must not crash it,
so calls are limited:

- **At most 32 nested calls.** Deeper than that fails with
  `MacroCallDepthExceeded`. The limit counts every nested call, whether it
  was made as a statement or as an expression.
- **Tail calls don't count.** A macro whose `@return` is exactly a call to
  itself, like `@return gcd(b, a % b)`, reuses the current call instead of
  nesting a new one. A tail-call loop can run up to 4,096 times before it
  fails with `MacroTailCallLimitExceeded`.

```basm
macro gcd(a: int, b: int) -> int {
    @if b == 0 {
        @return a
    }
    @return gcd(b, a % b)
}

macro show(value: int) {
    @emit value
}

show gcd(1071, 462)
```

```emits
21
```

A call is only a tail call if it's the *whole* `@return` value.
`@return 1 + f(x)` isn't one. A macro with an [`after`](hooks.md) hook never
makes tail calls, because its hooks have to run after each call finishes.

## Prefer loops for long runs

For anything that repeats more than a few thousand times, use
[`@for`](../meta/for.md) or [`@fold`](../meta/fold.md) instead. They have no
depth limit, and can run up to 1,000,000 iterations:

```basm
macro sum_to(n: int) -> int {
    @return @fold total = 0 @for i in 0..=n {
        @next total + i
    }
}

macro show(value: int) {
    @emit value
}

show sum_to(100_000)
```

```emits
5000050000
```
