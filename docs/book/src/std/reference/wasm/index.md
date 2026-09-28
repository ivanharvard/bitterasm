# `wasm`

[`std`](../index.md) › `wasm`

| Name | Summary |
|---|---|
| [`impl`](impl.md) | WebAssembly instructions: a representative subset of control, variable and `i32` instructions, each emitted as its binary encoding. |
| [`leb128`](leb128.md) | LEB128, the variable-length integer encoding WebAssembly uses for every index, count and constant: 7 bits per byte, low bits first, with the top bit set on every byte but the last. |
| [`module`](module.md) | The WebAssembly module container: the header, sections, and their length prefixes. A section starts with its id byte and its length in bytes, which `deferred_uleb128` fills in once `bitter` knows it. |
