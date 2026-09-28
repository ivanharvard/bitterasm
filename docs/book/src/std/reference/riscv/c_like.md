# `c_like`

[`std`](../index.md) › [`riscv`](index.md) › `c_like`

RISC-V RV32I written as expressions instead of mnemonics:
`a0 = a1 + a2`, `a1 = mem[sp + 8]`, `if (a0 != zero) goto loop`.

```basm
from std.riscv.c_like import *

loop:
    a0 <- a0 + -1
    if (a0 != zero) goto loop
    a1 = mem[sp + 8]
```

```bytes
13 05 f5 ff
e3 1e 05 fe
83 25 81 00
```

- A register-register operation uses `=`, and one with an immediate uses
  `<-`: `a0 = a0 + a1` is `add`, and `a0 <- a0 + 1` is `addi`.
- `>>>` is a logical shift right, and `>>` an arithmetic one.
- Loads and stores smaller than a word name their width before `mem`:
  `i8`, `u8`, `i16` or `u16`.
- Unsigned comparisons (`sltu`, `sltiu`, `bltu`, `bgeu`), `lui`, `auipc`,
  `ecall` and `ebreak` have no C-like spelling, and keep their plain
  `name a, b, c` syntax.

Import this instead of `std.riscv.native`, never alongside it: both
assign syntax to the same instructions.

Re-exports [`std.riscv.impl`](impl.md).

## Macros

### `add`

`rd = rs1 + rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 + rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sub`

`rd = rs1 - rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 - rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sll`

`rd = rs1 << rs2`, shifting by the low 5 bits of `rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 << rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `slt`

`rd = 1` if `rs1 < rs2` as signed numbers, else 0.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 < rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sltu`

`rd = 1` if `rs1 < rs2` as unsigned numbers, else 0.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sltu rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `xor`

`rd = rs1 ^ rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 ^ rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `srl`

`rd = rs1 >> rs2`, shifting in zeros, by the low 5 bits of `rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 >>> rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sra`

`rd = rs1 >> rs2`, shifting in copies of the sign bit, by the low 5 bits of
`rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 >> rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `or`

`rd = rs1 | rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 \| rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `and`

`rd = rs1 & rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = rs1 & rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `addi`

`rd = rs1 + imm`, with a 12-bit signed `imm`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 + imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `slti`

`rd = 1` if `rs1 < imm` as signed numbers, else 0.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 < imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sltiu`

`rd = 1` if `rs1 < imm` as unsigned numbers, else 0. `imm` is
sign-extended first, so `sltiu rd, rs1, 1` tests for zero.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sltiu rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `xori`

`rd = rs1 ^ imm`. `xori rd, rs1, -1` is bitwise NOT.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 ^ imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `ori`

`rd = rs1 | imm`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 \| imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `andi`

`rd = rs1 & imm`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 & imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `slli`

`rd = rs1 << shamt`, for `shamt` from 0 to 31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 << shamt` | `rd: Reg`, `rs1: Reg`, `shamt: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `srli`

`rd = rs1 >> shamt`, shifting in zeros, for `shamt` from 0 to 31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 >>> shamt` | `rd: Reg`, `rs1: Reg`, `shamt: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `srai`

`rd = rs1 >> shamt`, shifting in copies of the sign bit, for `shamt` from 0
to 31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd <- rs1 >> shamt` | `rd: Reg`, `rs1: Reg`, `shamt: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `jalr`

Jumps to `rs1 + offset` (with bit 0 cleared) and puts the address of the
next instruction in `rd`. `jalr zero, ra, 0` returns from a function.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = call[rs1 + offset]` | `rd: Reg`, `rs1: Reg`, `offset: int` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `ecall`

Calls the execution environment, e.g. a Linux system call.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ecall` |  | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `ebreak`

Stops in a debugger.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ebreak` |  | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `lb`

Loads the byte at `rs1 + offset` into `rd`, sign-extended.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = i8 mem[rs1 + offset]` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `lh`

Loads the 16-bit halfword at `rs1 + offset` into `rd`, sign-extended.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = i16 mem[rs1 + offset]` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `lw`

Loads the 32-bit word at `rs1 + offset` into `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = mem[rs1 + offset]` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `lbu`

Loads the byte at `rs1 + offset` into `rd`, zero-extended.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = u8 mem[rs1 + offset]` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `lhu`

Loads the 16-bit halfword at `rs1 + offset` into `rd`, zero-extended.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = u16 mem[rs1 + offset]` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sb`

Stores the low byte of `rs2` at `rs1 + offset`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `i8 mem[rs1 + offset] = rs2` | `rs2: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<SType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sh`

Stores the low 16 bits of `rs2` at `rs1 + offset`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `i16 mem[rs1 + offset] = rs2` | `rs2: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<SType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `sw`

Stores `rs2` at `rs1 + offset`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mem[rs1 + offset] = rs2` | `rs2: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<SType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `beq`

Branches to the label `target` if `rs1 == rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `if (rs1 == rs2) goto target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `bne`

Branches to the label `target` if `rs1 != rs2`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `if (rs1 != rs2) goto target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `blt`

Branches to the label `target` if `rs1 < rs2` as signed numbers.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `if (rs1 < rs2) goto target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `bge`

Branches to the label `target` if `rs1 >= rs2` as signed numbers.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `if (rs1 >= rs2) goto target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `bltu`

Branches to the label `target` if `rs1 < rs2` as unsigned numbers.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bltu rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `bgeu`

Branches to the label `target` if `rs1 >= rs2` as unsigned numbers.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bgeu rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `lui`

`rd = imm << 12`: loads a 20-bit upper immediate.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lui rd, imm` | `rd: Reg`, `imm: int` | emits `LittleEndian<UType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `auipc`

`rd = pc + (imm << 12)`: an address relative to this instruction.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `auipc rd, imm` | `rd: Reg`, `imm: int` | emits `LittleEndian<UType, 32>` |  | [`std.riscv.impl`](impl.md) |

### `jal`

Jumps to the label `target` and puts the address of the next instruction
in `rd`. `jal ra, f` calls `f`; `jal zero, l` just jumps.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rd = call(target)` | `rd: Reg`, `target: int` | emits `LittleEndian<JType, 32>` |  | [`std.riscv.impl`](impl.md) |

## Re-exported

From [`std.riscv.impl`](impl.md): `Reg`, `x0`, `x1`, `x2`, `x3`, `x4`, `x5`, `x6`, `x7`, `x8`, `x9`, `x10`, `x11`, `x12`, `x13`, `x14`, `x15`, `x16`, `x17`, `x18`, `x19`, `x20`, `x21`, `x22`, `x23`, `x24`, `x25`, `x26`, `x27`, `x28`, `x29`, `x30`, `x31`, `zero`, `ra`, `sp`, `gp`, `tp`, `t0`, `t1`, `t2`, `s0`, `fp`, `s1`, `a0`, `a1`, `a2`, `a3`, `a4`, `a5`, `a6`, `a7`, `s2`, `s3`, `s4`, `s5`, `s6`, `s7`, `s8`, `s9`, `s10`, `s11`, `t3`, `t4`, `t5`, `t6`, `Opcode`, `Funct3`, `Funct7`, `Bit1`, `Imm4`, `Imm5`, `Imm6`, `Imm7`, `Imm8`, `Imm10`, `Imm12`, `Imm20`, `RType`, `IType`, `SType`, `BType`, `UType`, `JType`.
