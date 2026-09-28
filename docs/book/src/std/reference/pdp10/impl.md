# `impl`

[`std`](../index.md) › [`pdp10`](index.md) › `impl`

The PDP-10: 36-bit words, 16 accumulators, and a representative subset
of its instructions, all in the one word format.

```basm
from std.pdp10.impl import *

movei ac1, 0, Index(0), 42
```

```bytes
04 08 80 00 2a
```

Each instruction takes its operands as the manual's fields: accumulator
`ac`, indirect bit `i`, index register `x`, and address `y`. The machine
is word-addressed, so a word has no byte order; `bitter` packs its 36
bits into 5 bytes with 4 zero bits above them.

Opcodes are from the DEC PDP-10 System Reference Manual (DEC-10-HGAA-D),
Appendix A.

## Macros

### `halt`

Stops the processor. It's `JRST 4,`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `halt` |  | emits `Instr` |  |

### `jrst`

Jumps to the effective address.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jrst i, x, y` | `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `jumpa`

Jumps to the effective address. `JUMPA` is `JUMP` with the "always"
condition.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jumpa i, x, y` | `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `movei`

Loads the effective address itself, not what's there, into `ac`: an
immediate load.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movei ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `move`

Loads the word at the effective address into `ac`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `move ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `movem`

Stores `ac` at the effective address.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movem ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `add`

Adds the word at the effective address into `ac`. Addresses 0 to 15 are
the accumulators, so `add(ac1, 0, Index(0), 2)` adds `ac2` into `ac1`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `add ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `addi`

Adds the effective address itself into `ac`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `addi ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `sub`

Subtracts the word at the effective address from `ac`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sub ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `and`

ANDs the word at the effective address into `ac`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `and ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `cain`

Compares `ac` with the effective address itself, and skips the next
instruction if they're not equal.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cain ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

### `exch`

Swaps `ac` with the word at the effective address.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `exch ac, i, x, y` | `ac: AC`, `i: int`, `x: Index`, `y: int` | emits `Instr` |  |

## Types

### `AC`

`type AC = bits<4>`

An accumulator number, 0 to 15.

### `Opcode`

`type Opcode = bits<9>`

The 9-bit opcode.

### `Indirect`

`type Indirect = bits<1>`

The indirect bit: 1 means `y` holds the address of the operand's address.

### `Index`

`type Index = bits<4>`

An index register number. Any accumulator but 0 can be one; 0 means
none.

### `Addr`

`type Addr = bits<18>`

An 18-bit address.

### `Instr`

`struct Instr`

The PDP-10 instruction word: opcode (9 bits), `ac` (4), `i` (1), `x` (4)
and `y` (18), most significant first.

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `ac0` |  | `AC(0)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac1` |  | `AC(1)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac2` |  | `AC(2)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac3` |  | `AC(3)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac4` |  | `AC(4)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac5` |  | `AC(5)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac6` |  | `AC(6)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac7` |  | `AC(7)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac8` |  | `AC(8)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac9` |  | `AC(9)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac10` |  | `AC(10)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac11` |  | `AC(11)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac12` |  | `AC(12)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac13` |  | `AC(13)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac14` |  | `AC(14)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
| `ac15` |  | `AC(15)` | An accumulator. None is wired to zero, and there are no ABI names: each PDP-10 operating system had its own conventions. |
