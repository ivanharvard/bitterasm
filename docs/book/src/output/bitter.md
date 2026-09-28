# Packing bytes with `bitter`

`bitter` turns each emitted value into bits, and each value's bits into
whole bytes.

## `bits<N>`: a number with a width

`std.binary`'s `bits<N>` is the basic unit: an integer that fits in `N` bits.
It packs to exactly `N` bits.

```basm
from std.binary import bits

macro db(value: int) {
    @emit value as bits<8>
}

macro dw(value: int) {
    @emit value as bits<16>
}

db 0x41
dw 0x1234
```

```bytes
41 12 34
```

A bare `int` has no width, so `bitter` rejects it. Neither does an enum,
which is rejected too.

## Structs: fields in order

Any other struct packs as its fields, concatenated in declaration order. The
first field goes in the most significant bits:

```basm
from std.binary import bits

struct Nibbles {
    hi: bits<4>,
    lo: bits<4>,
}

macro nibbles(hi: int, lo: int) {
    @emit Nibbles(hi as bits<4>, lo as bits<4>)
}

nibbles 0xA, 0xB
```

```bytes
ab
```

That's the whole instruction-encoding model. An instruction format is a
struct whose fields add up to the instruction's width, and `bitter` knows
nothing about opcodes or registers.

## Rounding up to bytes

Each top-level value is padded with zero bits *at the top* to a whole number
of bytes:

```basm
from std.binary import bits

macro emit12(value: int) {
    @emit value as bits<12>
}

emit12 0xabc
```

```bytes
0a bc
```

## Byte order

With nothing else said, `bitter` writes the most significant byte first
(big-endian). That's the only order that makes sense for a machine that
isn't byte-addressed at all.

An architecture that wants little-endian output wraps each value in
`std.bitter.byte_order`'s `LittleEndian<T, width>`, and `bitter` reverses its
bytes. `width` must match the wrapped value's width, and be a multiple of 8.

```basm
from std.binary import bits
from std.bitter.byte_order import LittleEndian

macro dw_le(value: int) {
    @emit LittleEndian<bits<16>, 16> { value: value as bits<16> }
}

dw_le 0x1234
```

```bytes
34 12
```

`std.riscv` and `std.x86_64` wrap every instruction this way.

## Values not known yet: `Positioned<N>`

Some values depend on where things end up, such as the distance to a
branch target. `std.bitter.deferred` builds such values as a `Deferred`
expression, and `Positioned<N>` gives one a width:

| Expression | `bitter` resolves it to |
|---|---|
| `here()` | The position of the value being packed |
| `span(a, b)` | The number of bytes from position `a` to position `b`; negative if `b` comes first |
| `add`, `sub`, `shr`, `band`, ... | Arithmetic on the above |

`bitter` lays the image out first, then resolves each `Positioned<N>` and
packs it into `N` bits. See [Labels](../programs/labels.md#from-positions-to-addresses)
for an example.

Only use `here()` directly inside the `span` of the value being emitted.
Stored and used by a different value, it silently refers to that other
value's position instead.

## Layout: `align` and `pad_image`

`std.bitter.layout` provides two directives that only `bitter` can resolve,
because they depend on the final layout:

- **`align n`** emits zero bytes until the next value starts at a multiple of
  `n` bytes from the start of the image.
- **`pad_image n`** pads the end of the finished image with zeros to a
  multiple of `n` bytes, wherever it's written.

```basm
from std.binary import bits
from std.bitter.layout import align

macro db(value: int) {
    @emit value as bits<8>
}

db 1
align 4
db 2
```

```bytes
01 00 00 00 02
```

Both must be emitted as whole values, never as fields inside another
struct.

## Sections

Before packing, `bitter` groups values by [section](../programs/sections.md).
With several input files, `bitter build` also links them. See
[Linking multiple files](../programs/linking.md).
