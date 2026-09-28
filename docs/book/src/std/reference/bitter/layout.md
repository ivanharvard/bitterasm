# `layout`

[`std`](../index.md) › [`bitter`](index.md) › `layout`

Padding that depends on the final layout, which only `bitter` knows:
`align` and `pad_image`. Emit them as whole values, never as fields
inside another struct.

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

## Macros

### `align`

Pads with zeros so the next value starts at a multiple of `n` bytes from
the start of the image.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `align n` | `n: int` |  |  |

### `pad_image`

Pads the finished image with zeros to a multiple of `n` bytes, wherever
it's written, for formats whose size must be rounded up (PE rounds its
sections to 512 bytes). It takes no space where it appears, so `span` and
`std.bitter.link`'s `image_end` measure the image without the padding.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pad_image n` | `n: int` |  |  |

## Types

### `Align`

`struct Align<const n: int>`

Zero bytes up to the next multiple of `n` bytes from the start of the
image. `align` emits one. A `span` across it counts the padding.

### `PadImage`

`struct PadImage<const n: int>`

Zero bytes at the end of the image, up to a multiple of `n` bytes.
`pad_image` emits one.
