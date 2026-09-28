# `link`

[`std`](../index.md) › [`bitter`](index.md) › `link`

Positions `bitter` defines when it links a program, the way a linker
defines `_end`. `span(image_start, image_end)` is the image's size in
bytes, and `span(image_start, label)` is `label`'s offset into it, however
many files the program spans: what an executable header needs.

```basm
from std.bitter.deferred import *
from std.bitter.link import image_start, image_end

macro size_byte() {
    @emit Positioned<8> { value: span(image_start, image_end) }
}

size_byte
size_byte
```

```bytes
02 02
```

## Labels

| Label | Description |
|---|---|
| `image_start` | The first byte of the linked image. |
| `image_end` | Just past the last byte of the linked image. |
