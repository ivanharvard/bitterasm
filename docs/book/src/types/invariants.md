# Invariants

An invariant is a rule that every value of a type must follow. It's checked
every time a value of the type is made, so a value that exists is known to
be valid.

## Syntax

```basm,ignore
struct Name
    | invariant condition
{
    fields
}

type Name = Type
    | invariant condition
```

A type can have several invariants, and all of them must hold.

## On structs

A struct's invariant can use its fields and its generic parameters. Refer to
a field by its bare name or as `source.field`; both work.

```basm
struct Range
    | invariant lo <= hi
{
    pub lo: int,
    pub hi: int,
}

macro size(r: Range) -> int {
    @return r.hi - r.lo
}

macro show(value: int) {
    @emit value
}

show size(Range(2, 10))
```

```emits
8
```

```basm,fail
struct Range
    | invariant lo <= hi
{
    pub lo: int,
    pub hi: int,
}

macro size(r: Range) -> int {
    @return r.hi - r.lo
}

macro show(value: int) {
    @emit value
}

show size(Range(10, 2))
```

```error
invariant `(lo <= hi)` was violated for `Range`
```

It's checked at every construction, in any form: `Range(10, 2)`,
`Range(lo = 10, hi = 2)`, `Range { lo: 10, hi: 2 }`, or conversion with `as`.

## `bits<N>` is an invariant

The standard library's most important type is a struct with one invariant.
From `std.binary`:

```basm,ignore
pub struct bits<const width: int>
    | invariant fits_inside_width(width, value)
{
    pub skip value: int
}
```

`bits<8>` is an integer that's been checked to fit in 8 bits. An instruction
field typed `bits<5>` therefore can't be handed a register number of 40.

## Sharing checks

The condition can call macros, so a rule used by several types can live in
one place. That's what `fits_inside_width` above is:

```basm,ignore
pub macro fits_inside_width(width: int, value: int) -> bool {
    @return value >= 0 && value < (1 << width)
}
```

## On type aliases

An invariant on a type alias turns it into a new type that only
[`as`](conversions.md) can produce. The value being checked can have any
free name. See [Type aliases](aliases.md#aliases-with-invariants-a-new-type).

```basm
from std.binary import bits

type Imm12 = bits<12>
    | invariant n % 4 == 0

macro emit_offset(offset: Imm12) {
    @emit offset
}

emit_offset 64 as Imm12
```

```emits
bits<12> { value: 64 }
```

## Checks written as macros

A struct can also name a macro as a facet. Every value the struct makes is
passed to that macro, which can check it with `@assert` and must return it
unchanged. Unlike an invariant's condition, the check can take arguments,
be shared between structs, and give its own error:

```basm,fail
macro below(reg: Reg, limit: int) -> Reg {
    @assert reg.id < limit, "register out of range"
    @return reg
}

struct Reg
    | below 16
{
    pub id: int,
}

macro show(reg: Reg) {
    @emit reg.id
}

show Reg(20)
```

```error
register out of range
```

The macro takes the value as its first parameter, and `| below 16` calls
`below(value, 16)`. A value of the same struct made inside the macro isn't
passed back through it. [Spaces and sections](../programs/spaces.md) take
facets written as macros too.

## Invariants versus `@assert`

| | Invariant | [`@assert`](../meta/assert.md) |
|---|---|---|
| Belongs to | A type | A macro |
| Checked | Whenever a value of the type is made | When that line runs |
| Good for | Rules about what a value *is* | Rules about one operation's inputs |

An invariant may use the struct's private fields, since it always runs as
part of the file that declared the struct.
