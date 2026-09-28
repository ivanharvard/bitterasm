# `@for`

`@for` runs its body once for each element of a source, binding the element
to a name.

## Syntax

```basm,ignore
@for name in source {
    ...
}
```

## Looping over a range

```basm
macro squares(n: int) {
    @for i in 0..n {
        @emit i * i
    }
}

squares 5
```

```emits
0 1 4 9 16
```

See [Ranges](../basics/ranges.md) for `..` and `..=`.

## Looping over a struct

A struct value works as a source too. `@for` visits its `pub` fields in
declaration order, skipping fields marked `skip`. That's how you loop over an
array or a string:

```basm
from std.array import Array

macro sum<const N: int>(values: Array<int, N>) -> int {
    const total = @fold acc = 0 @for v in values {
        @next acc + v
    }
    @return total
}

macro each_char() {
    @for c in "hi" {
        @emit c
    }
}

macro show(value: int) {
    @emit value
}

show sum(Array<int, 3> { __el0: 1, __el1: 2, __el2: 3 })
each_char
```

```emits
6 104 105
```

See [Struct fields: `pub` and `skip`](../types/fields.md).

## Details

- **Each iteration is fresh.** Nothing carries over from one iteration to the
  next: a `const` in the body only exists for that iteration. To carry a
  value along, such as a running total, use [`@fold`](fold.md).
- **`@return` ends the whole macro**, not just the loop.
- **At most 1,000,000 iterations.**
- **At the top level**, the source must be a range written in place, such as
  `0..32`. The loop is unrolled before anything else in the file is
  evaluated, so it can generate declarations:

```basm
@for i in 0..4 {
    pub const r`i` = i * 10
}

macro show(value: int) {
    @emit value
}

show r3
```

```emits
30
```

That's how architecture packages declare their registers. The backticks in
`` r`i` `` build each name; see [Spliced names](../splicing/names.md).

## In struct declarations and constructions

`@for` can also generate a struct's fields, or the values of a construction.
See [Structs](../types/structs.md#generating-fields).
