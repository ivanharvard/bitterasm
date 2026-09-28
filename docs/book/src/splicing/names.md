# Spliced names

A name can be built from pieces: `` r`id` `` is `r` followed by the value of
`id`. With `id = 3`, it's the name `r3`.

```basm
@for i in 0..4 {
    pub const k`i` = i * i
}

macro show_k(n: int) {
    @emit k`n`
}

show_k 3
```

```emits
9
```

The top-level `@for` declares `k0` to `k3`, and `` k`n` `` in `show_k` reads
the one named by `n`.

## Where names can be spliced

| Position | Example |
|---|---|
| A declaration's name | `` pub const x`i` = ... ``, `` struct Word`n` { ... } `` |
| A struct field's name | `` pub __el`i`: T, `` |
| After a `.` | `` arr.__el`i` `` |
| An expression | `` k`n` `` |

The pieces can be any number of literal parts and splices, in any order:
`` WORD`bits`_BYTES `` is `WORD16_BYTES` when `bits` is `16`. A splice can
hold any expression: `` arr.__el`arr.len - 1 - i` ``.

## Arrays are built this way

`std.array`'s `Array<T, N>` has fields `__el0`, `__el1`, ... up to `N - 1`,
generated with spliced names, and its macros reach them the same way:

```basm
from std.array import Array

macro reversed_sum<const N: int>(arr: Array<int, N>) {
    @for i in 0..N {
        @emit arr.__el`N - 1 - i`
    }
}

reversed_sum Array<int, 3> { __el0: 1, __el1: 2, __el2: 3 }
```

```emits
3 2 1
```

## The backtick must touch the name

`` k`n` `` is one spliced name. `` k `n` ``, with a space, is the name `k`
followed by a separate splice of `n`.

## Reading private constants

A spliced read finds the current file's non-`pub` constants as well as `pub`
ones.

## A spliced name can't carry values between iterations

A `const` declared inside a `@for` body exists only for that iteration.
Spliced names can't be used to pass a value from one iteration to the next.
Use [`@fold`](../meta/fold.md) for that.
