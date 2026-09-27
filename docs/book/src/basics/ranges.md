# Ranges and `in`

## Ranges

A range is a run of consecutive integers:

| Syntax | Contains |
|---|---|
| `a..b` | `a`, `a + 1`, ..., `b - 1` (excludes `b`) |
| `a..=b` | `a`, `a + 1`, ..., `b` (includes `b`) |

If `b` is not greater than `a` (or, for `..=`, less than `a`), the range is
empty. Ranges only count upward. For other steps, use `std.iter`'s `range`.

A range is mostly used as the source of a [`@for`](../meta/for.md) loop:

```basm
macro show(value: int) {
    @emit value
}

@for i in 0..3 {
    show i
}

@for i in 0..=3 {
    show i * 10
}
```

```emits
0 1 2 0 10 20 30
```

Inside a macro, a range can be stored in a constant and used later. A
top-level `@for` needs its range written in place, because it's unrolled
before anything else in the file is evaluated.

## `in`

`value in source` is `1` if `source` contains `value`, and `0` otherwise.
The source can be:

- **A range.** `i in 0..len` checks `0 <= i && i < len`. Nothing is
  iterated, so it's cheap even for a range with a trillion elements.
- **A struct value**, such as an array or a string. `x in arr` is true if `x`
  equals one of the elements [`@for`](../meta/for.md) would visit: the `pub`
  fields that aren't `skip`, as described in
  [Struct fields](../types/fields.md).

```basm
macro show(value: int) {
    @emit value
}

show 5 in 0..5
show 5 in 0..=5
show 1000 in 0..1_000_000_000_000
show 'b' in "abc"
show 'z' in "abc"
```

```emits
0 1 1 1 0
```

`in` is often used in an [`@assert`](../meta/assert.md):

```basm
macro checked_index(idx: int, len: int) {
    @assert idx in 0..len, "index out of bounds"
    @emit idx
}

checked_index 3, 4
```

```emits
3
```

```basm,fail
macro checked_index(idx: int, len: int) {
    @assert idx in 0..len, "index out of bounds"
    @emit idx
}

checked_index 4, 4
```

```error
index out of bounds
```

`in` binds as tightly as `<`, and a range binds more loosely than anything
else, so `i + 1 in 0..n + 1` means `(i + 1) in 0..(n + 1)`.

## `in` isn't for types

`x in SomeEnum` doesn't test whether `x` is a valid variant. `in` always asks
whether a value *you have* contains something, the same question `@for`
walks through. Asking whether a value belongs to a *type* is a different
question, so it doesn't share the keyword.
