# `impl`

[`std`](../index.md) › [`wasm`](index.md) › `impl`

WebAssembly instructions: a representative subset of control, variable
and `i32` instructions, each emitted as its binary encoding.

```basm
from std.wasm.impl import *

local_get(0)
i32_const(-1)
i32_add
end
```

```bytes
20 00 41 7f 6a 0b
```

WebAssembly's dotted names aren't identifiers here, so `i32.const` is
`i32_const` and `local.get` is `local_get`. Names that are keywords in
other languages end in `_`: `if_`, `else_`, `return_`. `std.wasm.module`
builds the module around them.

## Macros

### `unreachable`

Traps immediately.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `unreachable` |  | emits `Byte` |  |

### `nop`

Does nothing.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `nop` |  | emits `Byte` |  |

### `block`

Starts a block. `br` to it jumps to its `end`. `blocktype` is
`EMPTY_BLOCKTYPE` or the type of the value it produces.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `block blocktype` | `blocktype: Byte` | emits `OpWithImm<1>` |  |

### `loop`

Starts a loop. `br` to it jumps back to its start.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `loop blocktype` | `blocktype: Byte` | emits `OpWithImm<1>` |  |

### `if_`

Starts an `if`: runs what follows if the `i32` it pops isn't zero, and
the `else_` part (if any) otherwise.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `if_ blocktype` | `blocktype: Byte` | emits `OpWithImm<1>` |  |

### `else_`

Starts the `else` part of an `if_`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `else_` |  | emits `Byte` |  |

### `end`

Ends a `block`, `loop`, `if_` or function body.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `end` |  | emits `Byte` |  |

### `br`

Branches to the enclosing block `depth` levels out: `br(0)` is the
innermost.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `br depth` | `depth: int` | emits `OpWithImm<...>` |  |

### `br_if`

Pops an `i32`, and does `br(depth)` if it isn't zero.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `br_if depth` | `depth: int` | emits `OpWithImm<...>` |  |

### `return_`

Returns from the current function.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `return_` |  | emits `Byte` |  |

### `call`

Calls function number `func_index`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `call func_index` | `func_index: int` | emits `OpWithImm<...>` |  |

### `drop`

Pops a value and discards it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `drop` |  | emits `Byte` |  |

### `local_get`

Pushes local variable number `index`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `local_get index` | `index: int` | emits `OpWithImm<...>` |  |

### `local_set`

Pops a value into local variable number `index`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `local_set index` | `index: int` | emits `OpWithImm<...>` |  |

### `local_tee`

Stores the top of the stack in local variable number `index`, without
popping it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `local_tee index` | `index: int` | emits `OpWithImm<...>` |  |

### `i32_load`

Pops an address and pushes the `i32` at `address + offset`. `align` is
the alignment the address is promised to have, as a power of two:
2, a 4-byte boundary, by default.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_load offset, align` | `offset: int = 0`, `align: int = 2` | emits `MemOp<...>` |  |

### `i32_store`

Pops an `i32` value, then an address, and stores the value at
`address + offset`. `align` means what it does for `i32_load`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_store offset, align` | `offset: int = 0`, `align: int = 2` | emits `MemOp<...>` |  |

### `i32_const`

Pushes `value` as an `i32`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_const value` | `value: int` | emits `OpWithImm<...>` |  |

### `i32_eqz`

Pops an `i32` and pushes 1 if it's zero, else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_eqz` |  | emits `Byte` |  |

### `i32_lt_s`

Pops two `i32`s and pushes 1 if the first is less than the second as
signed numbers, else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_lt_s` |  | emits `Byte` |  |

### `i32_add`

Pops two `i32`s and pushes their sum.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_add` |  | emits `Byte` |  |

### `i32_sub`

Pops two `i32`s and pushes the first minus the second.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_sub` |  | emits `Byte` |  |

### `i32_mul`

Pops two `i32`s and pushes their product.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `i32_mul` |  | emits `Byte` |  |

## Types

### `OpWithImm`

`struct OpWithImm<const N: int>`

An opcode followed by `N` bytes of immediates.

### `MemOp`

`struct MemOp<const N: int>`

A load or store: its opcode, then its memory operand, the alignment as a
power of two and the offset added to the address.

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `I32` | `Byte` | `Byte(0x7F)` | The `i32` value type. |
| `I64` | `Byte` | `Byte(0x7E)` | The `i64` value type. |
| `F32` | `Byte` | `Byte(0x7D)` | The `f32` value type. |
| `F64` | `Byte` | `Byte(0x7C)` | The `f64` value type. |
| `EMPTY_BLOCKTYPE` | `Byte` | `Byte(0x40)` | The block type of a `block`, `loop` or `if_` that produces no value. A value type such as `I32` means it produces one value of that type. |
