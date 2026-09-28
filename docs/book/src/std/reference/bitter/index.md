# `bitter`

[`std`](../index.md) › `bitter`

| Name | Summary |
|---|---|
| [`byte_order`](byte_order.md) | Byte order for `bitter`. Its default is most significant byte first; `LittleEndian` asks for the reverse. |
| [`deferred`](deferred.md) | Values that depend on where things end up, such as the distance to a branch target. They're built as `Deferred` expressions that `bitter` resolves once the image is laid out, and given a width by `Positioned<N>`. |
| [`layout`](layout.md) | Padding that depends on the final layout, which only `bitter` knows: `align` and `pad_image`. Emit them as whole values, never as fields inside another struct. |
| [`link`](link.md) | Positions `bitter` defines when it links a program, the way a linker defines `_end`. `span(image_start, image_end)` is the image's size in bytes, and `span(image_start, label)` is `label`'s offset into it, however many files the program spans: what an executable header needs. |
