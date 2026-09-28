# `tops10`

[`std`](../index.md) › [`pdp10`](index.md) › `tops10`

TOPS-10 monitor calls: the operating system's side of a PDP-10 program,
kept apart from `std.pdp10.impl`, which is only the machine.

```basm
from std.pdp10.impl import *
from std.pdp10.tops10 import *

start:
    outstr JOBDA + msg
    exit
msg:
    asciz "Hi\r\n"
```

```bytes
01 49 80 00 62
01 38 00 00 0a
09 1a 46 8a 00
```

TOPS-10 loads a program at `JOBDA`. Every instruction and data word is
one emitted value, so a label's position counts words, and `JOBDA` plus
the label is its address.

Opcodes are from the DECsystem-10 Monitor Calls manual (AA-0974G-TB).

## Macros

### `outstr`

Types the ASCIZ string at `address` on the terminal: `TTCALL 3,`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `outstr address` | `address: int` | emits `Instr` |  |

### `outchr`

Types the character in the low 7 bits of the word at `address` on the
terminal: `TTCALL 1,`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `outchr address` | `address: int` | emits `Instr` |  |

### `exit`

Ends the program and returns to the monitor: `CALLI 12`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `exit` |  | emits `Instr` |  |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `JOBDA` |  | `0o140` | `.JBDA`, the first address after the job data area: where TOPS-10 loads a program's code. |
