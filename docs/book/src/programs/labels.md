# Labels

A label names a position in the program's output.

## Syntax

```basm,ignore
name:
pub name:
```

A label goes on a line of its own. Its name can start with `.`, which is the
convention for a label that's only used nearby, like a loop. The dot is just
part of the name: `.loop` isn't scoped to anything, and every label name must
be unique in its file.

## A label's value is a position

A label has type `label`: a position in the output, not a number. Like a
macro, it's compile-time only. A macro can take one as a parameter, store it
and compare it, but it can't `@emit` it or do arithmetic with it, since a
position isn't an address.

When you do need the number, `as int` gives the index of the next value
emitted after the label: `0` for the first emitted value, `1` for the
second, and so on.

```basm
macro show(value: int) {
    @emit value
}

macro position(at: label) {
    @emit at as int
}

start:
    show 10
    show 20
middle:
    position start
    position middle
    position end
end:
```

```emits
10 20 0 2 5
```

A label can be used before it appears in the file, like `end` above.

Because `label` and `int` are different types, an instruction can take
either and mean different things. That's how `std.x86_64.att` reads
`buffer(%rip)` as the address of a label but `8(%rip)` as a plain
displacement, the way GNU `as` does.

## From positions to addresses

A position counts emitted values, not bytes. How many bytes each value takes
up is only known once an evaluator packs them. So byte distances are left to
the evaluator: `std.bitter.deferred` provides `span(a, b)`, a value that
`bitter` resolves to the number of bytes between positions `a` and `b`.

Wrap it in a `Positioned<N>` to give the result a width. Here a length byte
is written before the data it measures:

```basm
from std.binary import bits
from std.bitter.deferred import Positioned, span

macro db(value: int) {
    @emit value as bits<8>
}

macro dw(value: int) {
    @emit value as bits<16>
}

macro length_byte(start: label, end: label) {
    @emit Positioned<8> { value: span(start, end) }
}

    length_byte body, done
body:
    db 1
    dw 2
done:
```

```bytes
03 01 00 02
```

`here()` is the position of the value being emitted. A relative branch
encodes `span(here(), target)`, and that's how every jump and branch in
`std.riscv` and `std.x86_64` works. See
[Packing bytes with `bitter`](../output/bitter.md).

## Labels across files

`pub` exports a label. Another file imports it **by name**; `import *` never
brings in labels:

```basm,file=util.basm
pub double:
```

```basm
from .util import double

macro position(at: label) {
    @emit at as int
}

position double
```

```emits
<double>
```

The importing file can't know where `double` is: that depends on how the
files are laid out when they're linked. So the value is left unresolved, and
`bitter build` fills it in. See [Linking multiple files](linking.md).
