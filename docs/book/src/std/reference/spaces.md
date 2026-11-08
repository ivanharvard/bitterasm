# `spaces`

[`std`](index.md) › `spaces`

Facets for describing spaces. A space is a memory that sections go in,
like a chip's flash or its RAM. Every space is built from `stream`, the
default one, and gains fields from its facets:

```basm
from std.binary import bits
from std.spaces import start, unit

space flash = stream
    | unit<bits<16>>
    | start 0x8000

macro show(value: int) {
    @emit value
}

show flash.start
```

```emits
32768
```

A facet is an ordinary macro whose first parameter is the space it's
applied to, and which returns that space, usually with more fields from
`@extend`. `| start 0x8000` calls `start(flash, 0x8000)`.

Nothing reads these fields yet: laying sections out in their spaces is
still to come.

## Macros

### `start`

The address of `sp`'s first box.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `start(sp, address)` | `sp: space`, `address: int` | returns `space` |  |

### `unit`

The size of one box in `sp`: as wide as `S`, e.g. `bits<16>` for a chip
whose memory holds 16-bit words.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `unit(sp)` | `<S>`, `sp: space` | returns `space` |  |
