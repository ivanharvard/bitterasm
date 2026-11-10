# Structs

A struct groups named fields into one value.

## Declaring a struct

```basm,ignore
struct Name {
    field: Type,
    pub field: Type,
    pub field: Type = default,
}
```

- Fields are separated by commas or newlines; a trailing comma is fine.
- `pub` makes a field readable from other files, and visible to `@for`.
  See [Struct fields](fields.md).
- `= default` gives a value used when a construction leaves the field out.
- `pub struct` lets other files import the struct itself.
- Facets such as [`invariant`](invariants.md) go between the name and the
  `{`.

## Constructing a struct

There are three ways to build a struct value:

```basm
struct Point {
    pub x: int,
    pub y: int = 5,
}

macro emit_point(p: Point) {
    @emit p
}

emit_point Point(1, 2)            # positional
emit_point Point(x = 3)           # named; y uses its default
emit_point Point { x: 4, y: 6 }   # braces
```

```emits
Point { x: 1, y: 2 }
Point { x: 3, y: 5 }
Point { x: 4, y: 6 }
```

Every field without a default must be given a value:

```basm,fail
struct Point {
    pub x: int,
    pub y: int,
}

macro emit_point(p: Point) {
    @emit p
}

emit_point Point(1)
```

```error
`Point` expects 2 argument(s), but 1 were supplied
```

A [generic struct](../generics/index.md) is constructed with its generic
arguments first, then braces or parentheses: `Pair<int> { a: 1, b: 2 }` or
`Pair<int>(1, 2)`.

## Reading fields

Use `.field`:

```basm
struct Point {
    pub x: int,
    pub y: int,
}

macro show(value: int) {
    @emit value
}

const p = Point(3, 4)
show p.x * p.x + p.y * p.y
```

```emits
25
```

Values are never modified. To "change" a field, build a new struct.

## Structs as machine code

When an evaluator like `bitter` packs a struct, it concatenates its fields,
first field in the most significant bits. That's how an instruction format is
described. Here's RISC-V's R-type format from `std.riscv.impl`:

```basm,ignore
pub struct RType {
    funct7: Funct7,     # bits<7>
    rs2: Reg,           # bits<5>
    rs1: Reg,           # bits<5>
    funct3: Funct3,     # bits<3>
    rd: Reg,            # bits<5>
    opcode: Opcode,     # bits<7>
}
```

A 32-bit instruction is just a struct whose fields add up to 32 bits. See
[Packing bytes with `bitter`](../output/bitter.md).

## Anonymous structs

A struct doesn't need a declaration. Braces make one on the spot, with its
fields by name or by position:

```basm
macro show(value: int) {
    @emit value
}

const point = { x: 1, y: 2 }
const pair = { 3, 4 }

show point.y
show pair.__el0
```

```emits
2 3
```

Fields given by position are named `__el0`, `__el1`, ..., like a string's
characters. Its type is written `struct { ... }`:

```basm
macro show(value: int) {
    @emit value
}

macro sum(p: struct { x: int, y: int }) -> int {
    @return p.x + p.y
}

const origin: struct { x: int, y: int } = { 0, 0 }

show sum({ x: 1, y: 2 })
show sum({ 3, 4 })
show sum(origin)
```

```emits
3 7 0
```

Values given by position take their names from the type they're declared
as, a parameter's or a `const`'s, in order. An anonymous struct can also be
converted with `as`: `{ 3, 4 } as struct { x: int, y: int }`.

Two anonymous structs have the same type when their fields have the same
names and types, in the same order. A declared struct is always its own
type, even if another has the same fields, so `{ x: 1, y: 2 }` isn't a
`Point`, and `==` between anonymous structs compares their fields:

```basm
macro show(value: int) {
    @emit value
}

show { x: 1, y: 2 } == { x: 1, y: 2 }
show { x: 1, y: 2 } == { x: 1, y: 3 }
```

```emits
1 0
```

## Generating fields

A struct's fields can be generated with [`@for`](../meta/for.md) and
[`@if`](../meta/if.md), using the struct's generic parameters. This is how
`std.array` declares an array of any length:

```basm
pub struct Array<T, const N: int>
    | invariant N >= 0
{
    @for i in 0..N {
        pub __el`i`: T,
    }

    pub skip len: int = N,
}

macro show_all<const N: int>(values: Array<int, N>) {
    @for v in values {
        @emit v
    }
}

show_all Array<int, 3>(7, 8, 9)
```

```emits
7 8 9
```

A construction can generate its field values the same way:

```basm
from std.array import Array

macro squares<const N: int>(_len: Array<int, N>) -> Array<int, N> {
    @return Array<int, N> {
        @for i in 0..N {
            __el`i`: i * i,
        }
    }
}

macro show_all<const N: int>(values: Array<int, N>) {
    @for v in values {
        @emit v
    }
}

show_all squares(Array<int, 4>(0, 0, 0, 0))
```

```emits
0 1 4 9
```

Positional arguments fill generated fields too, in order: that's how
`Array<int, 3>(7, 8, 9)` above gives `__el0`, `__el1` and `__el2` their
values. A `skip` field such as `len` is passed over, since it's derived;
naming it is the only way to set it.

`` __el`i` `` builds the field names `__el0`, `__el1`, ... See
[Spliced names](../splicing/names.md).
