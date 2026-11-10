# Spaces

A space is a memory that sections go in. A laptop has one memory, holding
both code and data. A microcontroller often has several: its program lives
in flash, and its variables in RAM. Each is a separate space, with its own
addresses starting from 0.

Like labels, spaces are compile-time values: a macro can take one, store
it and read its fields, but never emit it.

## `stream` and `space`

Every space is built from `stream`, the default space, the same way every
type is built from `int`. A `space` declaration makes a new one:

```basm,ignore
space name = stream
space name = stream | facet | facet ...
```

A space starts out with the fields of the space it's built from, and gains
more from its **facets**. `stream` itself has none.

## Facets are macros

A facet is an ordinary macro whose first parameter is the space it's
applied to, and which returns that space, usually with more fields.
`| start 0x8000` calls `start(flash, 0x8000)`. Facets apply in the order
they're written, on the declaration's line or on lines of their own.

```basm
macro start(sp: space, address: int) -> space {
    @return @extend sp { start: int = address }
}

macro size(sp: space, boxes: int) -> space {
    @return @extend sp { size: int = boxes }
}

macro show(value: int) {
    @emit value
}

space flash = stream
    | start 0x8000
    | size 1024

show flash.start
show flash.size
```

```emits
32768 1024
```

`std.spaces` provides `start`, and `unit`, which sets how wide one box of
the space is.

A facet has to return the space it was given. Returning a different one is
an error, so a facet can add to a space but never swap it out:

```basm,fail
macro swap(_sp: space) -> space {
    @return stream
}

space flash = stream | swap
```

```error
facet `swap` must return the space `flash` it was given
```

A facet can also be called like any other macro, with the space written
out: `start(stream, 0x8000)` is a space too.

## `@extend`

`@extend subject { name: Type = value, ... }` gives a copy of `subject`
with more fields. Nothing in BitterASM changes once it's made, so `@extend`
never changes `subject` itself: it gives a new value.

A field can't be added twice:

```basm,fail
macro size(sp: space, boxes: int) -> space {
    @return @extend sp { size: int = boxes }
}

space flash = stream
    | size 1
    | size 2
```

```error
space `flash` already has a field `size`
```

A field can also hold a type, written `name: type = T`. That's how
`std.spaces`'s `unit<bits<16>>` records the width of a box:

```basm,ignore
macro unit<S>(sp: space) -> space {
    @return @extend sp { unit: type = S }
}
```

Only a space or a section can be extended. A struct's fields are fixed by
its type.

## `@hasfield`

`@hasfield subject, name` is `1` if `subject` has a field called `name`,
and `0` if not. A facet can use it to report a clearer error:

```basm
macro start(sp: space, address: int) -> space {
    @if @hasfield sp, start {
        @error "this space already has a start address"
    }
    @return @extend sp { start: int = address }
}

macro show(value: int) {
    @emit value
}

space flash = stream | start 0x8000

show @hasfield flash, start
show @hasfield stream, start
```

```emits
1 0
```

## Sections in a space

A `section` line can say which space the section goes in, and give it
facets of its own:

```basm,ignore
section .text in flash
    | progbits
    | alloc
```

A bare `section .text`, with no space or facets, only switches to the
section. A section has one space and one set of facets, so entering it
again with different ones is an error. Without `in`, a section goes in
`stream`.

A section's facets work the same way, taking and returning a `section`.
Its space is its `space` field, so a facet can check where its section
went:

```basm,fail
macro start(sp: space, address: int) -> space {
    @return @extend sp { start: int = address }
}

macro high(sec: section) -> section {
    @assert sec.space.start >= 0x8000, "this section has to be in high memory"
    @return sec
}

space ram = stream | start 0x100

section .vectors in ram
    | high
```

```error
this section has to be in high memory
```

A `section` line inside a macro can use the macro's arguments in its space
and facets. Running the same line again is fine as long as it gives the
same section; giving a different one is an error.

The first time a section gets its space or facets has to come before
anything goes in it: a bare `section .x` followed later by
`section .x in flash` is an error.

## Sections from a library

A [section is a value](sections.md#sections-are-values), so a library
shares one as a const, and a program switches to it with `@into`:

```basm,ignore
from std.formats.elf import TEXT

@into TEXT
```

`std.formats.elf` has `TEXT`, `RODATA`, `DATA` and `BSS`, the sections
`.text`, `.rodata`, `.data` and `.bss` with ELF's section facets,
`progbits`, `nobits`, `alloc`, `write` and `exec`.

## Labels in a space

A label is in the space of the section it's in. Its type says so:
`label<flash>` is a label in `flash`. A bare `label`, or `label<...>`,
takes a label in any space. A generic `label<S>` takes a label in whatever
space `S` turns out to be, so two parameters typed `label<S>` must be in
the same space:

```basm,fail
macro distance<S>(_from: label<S>, _to: label<S>) {
}

space flash = stream
space ram = stream

section .text in flash
start:
    distance start, buffer
section .data in ram
buffer:
```

```error
type mismatch for `S`: expected `label<flash>`, found `label<ram>`
```

That's how `std.bitter.deferred`'s `span` rejects two labels in different
spaces: there's no distance between two different memories.

## What's not done yet

Nothing reads a space's or section's fields yet except your own code.
Sections are still laid out one after another in a single image, whatever
space they're in.
