# `@emit`

`@emit` adds one value to the program's output.

## Syntax

```basm,ignore
@emit value
```

## Example

```basm
macro bytes3(a: int, b: int, c: int) {
    @emit a
    @emit b
    @emit c
}

bytes3 1, 2, 3
```

```emits
1 2 3
```

## What can be emitted

Any value: an integer, a struct, an enum variant. The compiler doesn't care
what a value means; it just records it in order in the `.em` file. What the
value turns into is the [evaluator's](../output/index.md) decision. For
example, `bitter` packs a `bits<8>` into one byte and an instruction struct
into its encoding.

```basm
from std.binary import bits

struct Pair {
    pub hi: bits<4>,
    pub lo: bits<4>,
}

macro pair(hi: int, lo: int) {
    @emit Pair(hi as bits<4>, lo as bits<4>)
}

pair 0xA, 0xB
```

```emits
Pair { hi: bits<4> { value: 10 }, lo: bits<4> { value: 11 } }
```

```bytes
ab
```

## Emitting from nested calls

A macro's output includes everything emitted by the macros it calls, in
order. So an instruction macro can be built from smaller ones.

A call used *as an expression* can only emit when its values have somewhere
to go. See
[Where emitted values can go](../macros/returning.md#where-emitted-values-can-go).

## Restricting what a macro emits: `emits`

The `emits` facet declares which types a macro may emit. It's optional, but
if it's there, it's enforced: emitting anything else is a compile error.
Give several `emits` facets to allow several types, and use a
[wildcard](../generics/wildcards.md) to allow any instance of a generic
type, as in `| emits bits<...>`. This is how instruction macros declare what
they encode to; `->` is only for what a macro [returns](../macros/returning.md).

```basm
from std.binary import bits

macro byte(value: int)
    | emits bits<8>
{
    @emit value as bits<8>
}

byte 0x41
```

```emits
bits<8> { value: 65 }
```

```basm,fail
from std.binary import bits

macro byte(value: int)
    | emits bits<8>
{
    @emit value
}

byte 0x41
```

```error
`@emit`ed value has type `int`, but this macro's `emits` facet(s) only declare `bits<8>`
```

A macro with no `emits` facet may emit anything.

## Emitting moves labels

Each emitted value takes up one position in the output, and a
[label](../programs/labels.md) is the position of the next value emitted
after it.
