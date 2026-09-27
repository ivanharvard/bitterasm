# `@fold` and `@next`

`@fold` is a [`@for`](for.md) that carries values from one iteration to the
next. These values are called **accumulators**. `@next` ends an iteration
and gives the accumulators their values for the next one.

## Syntax

```basm,ignore
@fold acc = initial @for x in source {
    ...
    @next new_value
}

@fold a = 0, b = 0 @for x in source {
    ...
    @next a = new_a, b = new_b
}
```

## Example: a running total

```basm
macro sum_to(n: int) -> int {
    @return @fold total = 0 @for i in 0..=n {
        @next total + i
    }
}

macro show(value: int) {
    @emit value
}

show sum_to(10)
```

```emits
55
```

The fold starts with `total = 0`. Each iteration sees the current `total`,
and `@next total + i` gives the next iteration its new `total`. The fold's
value is the final `total`.

## Example: offsets into a table

A fold can emit, too. This one emits each entry's offset, then the table's
total size:

```basm
from std.array import Array

macro offsets<const N: int>(lengths: Array<int, N>) {
    const table_size = @fold offset = 0 @for len in lengths {
        @emit offset
        @next offset + len
    }
    @emit table_size
}

offsets Array<int, 3> { __el0: 4, __el1: 2, __el2: 5 }
```

```emits
0 4 6 11
```

## Several accumulators

Separate accumulators with commas. `@next` then names each one it changes.
The fold's value is a struct with one field per accumulator:

```basm
macro stats(n: int) {
    const r = @fold count = 0, evens = 0 @for i in 0..n {
        @if i % 2 == 0 {
            @next count = count + 1, evens = evens + 1
        }
        @next count = count + 1
    }
    @emit r.count
    @emit r.evens
}

stats 5
```

```emits
5 3
```

## How `@next` works

- **`@next` ends the iteration**, like `continue` in other languages. Code
  after it in the same iteration doesn't run.
- **Accumulators `@next` doesn't name keep their values**, and an iteration
  that reaches no `@next` at all keeps every value. So filtering needs no
  `@else`: `@if keep { @next total + x }`.
- **Nothing is mutated.** Each iteration binds fresh values, the same way
  `@for` binds its loop variable.
- **`@next` belongs to the innermost `@fold`** in the same macro body. Using it
  inside a plain `@for` nested in the fold, or in a macro called from the
  fold, is an error.
- A fold whose body has no `@next` at all gets the `fold_without_next`
  warning, since its accumulators could never change.

## Statement or expression

- **As an expression**, such as a `const`'s value or `@return`'s value, a fold
  gives its final accumulators. Its emitted values are kept.
- **As a statement**, its value is ignored. That's what you want when the body
  only emits.

## No depth limit

A fold runs as many iterations as its source has elements, up to `@for`'s
limit of 1,000,000. That makes it the tool for long computations that
[recursion](../macros/recursion.md), limited to 32 nested calls or 4,096 tail
calls, can't handle.

## Everywhere `@for` works

**At the top level** the fold is unrolled before anything else, like a
top-level `@for`. The source must be a range written in place, and the
accumulators are integers. `const x = @fold ...` names the result, which can
then bound a later top-level `@for`:

```basm
const total = @fold acc = 0 @for i in 0..4 {
    @next acc + i
}

macro show(value: int) {
    @emit value
}

show total
```

```emits
6
```

**In a struct construction**, a fold can compute field values:

```basm
from std.array import Array

macro prefix_sums() -> Array<int, 4> {
    @return Array<int, 4> {
        @fold acc = 0 @for i in 0..4 {
            __el`i`: acc + i,
            @next acc + i
        }
    }
}

macro show_all<const N: int>(values: Array<int, N>) {
    @for v in values {
        @emit v
    }
}

show_all prefix_sums()
```

```emits
0 1 3 6
```

**In a struct declaration**, a fold can compute field names. There, the
accumulators must be integers.
