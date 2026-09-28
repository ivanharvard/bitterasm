# `module`

[`std`](../index.md) › [`wasm`](index.md) › `module`

The WebAssembly module container: the header, sections, and their
length prefixes. A section starts with its id byte and its length in
bytes, which `deferred_uleb128` fills in once `bitter` knows it.

```basm
from std.wasm.module import *
from std.wasm.impl import *
from std.bitter.deferred import *

header

# The type section: one function type, () -> i32.
byte(0x01)
deferred_uleb128 span(types_start, types_end), 1
types_start:
byte(0x01)
func_type_0_to_1(I32)
types_end:
```

```bytes
00 61 73 6d 01 00 00 00
01 05 01 60 00 01 7f
```

`section_header` writes a section's id and size, and the helpers below
write the entries of the type, import, function, memory, export, code
and data sections: enough for a WASI program like
`examples/wasm/hello.basm`. The table, global, start and element sections
have no helpers yet.

## Macros

### `deferred_uleb128`

`value`, usually `span(start, end)`, as unsigned LEB128 in exactly `n`
bytes, padded if it turns out to need fewer. `n` has to be enough for
the final value: one byte holds up to 127.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `deferred_uleb128 value, n` | `value: Deferred`, `n: int` | emits `DeferredLeb128<...>` |  |

### `byte`

One byte, such as a section id.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `byte value` | `value: int` | emits `Byte` |  |

### `header`

The 8 bytes every module starts with: `\0asm`, then version 1.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `header` |  | emits `Bytes<8>` |  |

### `func_type_0_to_1`

A function type with no parameters and one result of type `result`,
such as `I32`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `func_type_0_to_1 result` | `result: Byte` | emits `Bytes<4>` |  |

### `u32`

`value`, which can't be negative, as unsigned LEB128: the spec's `u32`,
which every count and index is.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `u32 value` | `value: int` | emits `Bytes<...>` |  |

### `size`

The number of bytes from label `start` to label `end`, as unsigned
LEB128: the size before a section or a function body. It always takes 5
bytes, since it isn't known until layout, and 5 hold any size.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `size start, end` | `start: int`, `end: int` | emits `DeferredLeb128<5>` |  |

### `section_header`

A section's id, such as `TYPE_SECTION`, and its size. Put the label
`start` right after it and `end` after the section's contents.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `section_header id, start, end` | `id: int`, `start: int`, `end: int` |  |  |

### `params`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `params()` |  | returns `Bytes<1>` | A function's parameter types: `params(I32, I32)`. Up to four. |
| `params(a)` | `a: Byte` | returns `Bytes<2>` |  |
| `params(a, b)` | `a: Byte`, `b: Byte` | returns `Bytes<3>` |  |
| `params(a, b, c)` | `a: Byte`, `b: Byte`, `c: Byte` | returns `Bytes<4>` |  |
| `params(a, b, c, d)` | `a: Byte`, `b: Byte`, `c: Byte`, `d: Byte` | returns `Bytes<5>` |  |

### `results`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `results()` |  | returns `Bytes<1>` | A function's result types: `results(I32)`, or `results()` for none. |
| `results(a)` | `a: Byte` | returns `Bytes<2>` |  |

### `func_type`

A function type: `func_type params(I32), results(I32)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `func_type parameters, returned` | `parameters: Bytes<...>`, `returned: Bytes<...>` |  |  |

### `name`

A name: its length in bytes, then its UTF-8 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `name source` | `<S>`, `source: S` |  |  |

### `import_func`

Imports the function `item_name` from the module `module_name`, with the
type numbered `type_index` in the type section. Imported functions are
numbered before the module's own.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `import_func module_name, item_name, type_index` | `<S, T>`, `module_name: S`, `item_name: T`, `type_index: int` |  |  |

### `memory`

A memory of `pages` 64 KiB pages, with no maximum.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `memory pages` | `pages: int` |  |  |

### `export_func`

Exports function number `index` as `export_name`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `export_func export_name, index` | `<S>`, `export_name: S`, `index: int` |  |  |

### `export_memory`

Exports memory number `index` as `export_name`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `export_memory export_name, index` | `<S>`, `export_name: S`, `index: int` |  |  |

### `data_segment`

A data segment that copies the string `source`'s UTF-8 bytes into
memory 0 at address `offset` when the module starts.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `data_segment offset, source` | `<S>`, `offset: int`, `source: S` |  |  |

## Types

### `Leb128Group`

`struct Leb128Group`

One byte of a LEB128 value `bitter` works out later.

| Field | Type | Description |
|---|---|---|
| `continuation` | `bool` | Set on every byte but the last. |
| `payload` | `Positioned<7>` | Seven bits of the value. |

### `DeferredLeb128`

`struct DeferredLeb128<const N: int>`

`N` bytes of a LEB128 value `bitter` works out later.

Some of its fields are generated by `@for` or `@if`.

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `CUSTOM_SECTION` |  | `0` | The id of the custom section. |
| `TYPE_SECTION` |  | `1` | The id of the type section: function signatures. |
| `IMPORT_SECTION` |  | `2` | The id of the import section. |
| `FUNCTION_SECTION` |  | `3` | The id of the function section: each defined function's type. |
| `TABLE_SECTION` |  | `4` | The id of the table section. |
| `MEMORY_SECTION` |  | `5` | The id of the memory section. |
| `GLOBAL_SECTION` |  | `6` | The id of the global section. |
| `EXPORT_SECTION` |  | `7` | The id of the export section. |
| `START_SECTION` |  | `8` | The id of the start section. |
| `ELEMENT_SECTION` |  | `9` | The id of the element section. |
| `CODE_SECTION` |  | `10` | The id of the code section: each defined function's body. |
| `DATA_SECTION` |  | `11` | The id of the data section. |
