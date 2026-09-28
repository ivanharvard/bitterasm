# `impl`

[`std`](../index.md) › [`riscv`](index.md) › `impl`

RISC-V RV32I: its registers, instruction formats, and every base
instruction, each emitted as a little-endian 32-bit word.

Import a dialect rather than this module: `std.riscv.native` for the
standard assembly syntax, or `std.riscv.c_like`. Here every instruction
takes its operands in order, `lw rd, offset, rs1` rather than
`lw rd, offset(rs1)`.

```basm
from std.riscv.impl import *

loop:
    addi a0, a0, -1
    bne a0, zero, loop
```

```bytes
13 05 f5 ff
e3 1e 05 fe
```

Immediates are plain `int`s, truncated to their field, so a negative one
becomes its two's-complement encoding. Branch and jump targets are labels;
`bitter` works out the offset once the program is laid out.

## Macros

### `add`

`rd = rs1 + rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `add rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `sub`

`rd = rs1 - rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sub rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `sll`

`rd = rs1 << rs2`, shifting by the low 5 bits of `rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sll rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `slt`

`rd = 1` if `rs1 < rs2` as signed numbers, else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `slt rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `sltu`

`rd = 1` if `rs1 < rs2` as unsigned numbers, else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sltu rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `xor`

`rd = rs1 ^ rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xor rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `srl`

`rd = rs1 >> rs2`, shifting in zeros, by the low 5 bits of `rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `srl rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `sra`

`rd = rs1 >> rs2`, shifting in copies of the sign bit, by the low 5 bits of
`rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sra rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `or`

`rd = rs1 | rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `or rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `and`

`rd = rs1 & rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `and rd, rs1, rs2` | `rd: Reg`, `rs1: Reg`, `rs2: Reg` | emits `LittleEndian<RType, 32>` |  |

### `addi`

`rd = rs1 + imm`, with a 12-bit signed `imm`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `addi rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  |

### `slti`

`rd = 1` if `rs1 < imm` as signed numbers, else 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `slti rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  |

### `sltiu`

`rd = 1` if `rs1 < imm` as unsigned numbers, else 0. `imm` is
sign-extended first, so `sltiu rd, rs1, 1` tests for zero.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sltiu rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  |

### `xori`

`rd = rs1 ^ imm`. `xori rd, rs1, -1` is bitwise NOT.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xori rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  |

### `ori`

`rd = rs1 | imm`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ori rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  |

### `andi`

`rd = rs1 & imm`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `andi rd, rs1, imm` | `rd: Reg`, `rs1: Reg`, `imm: int` | emits `LittleEndian<IType, 32>` |  |

### `slli`

`rd = rs1 << shamt`, for `shamt` from 0 to 31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `slli rd, rs1, shamt` | `rd: Reg`, `rs1: Reg`, `shamt: int` | emits `LittleEndian<IType, 32>` |  |

### `srli`

`rd = rs1 >> shamt`, shifting in zeros, for `shamt` from 0 to 31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `srli rd, rs1, shamt` | `rd: Reg`, `rs1: Reg`, `shamt: int` | emits `LittleEndian<IType, 32>` |  |

### `srai`

`rd = rs1 >> shamt`, shifting in copies of the sign bit, for `shamt` from 0
to 31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `srai rd, rs1, shamt` | `rd: Reg`, `rs1: Reg`, `shamt: int` | emits `LittleEndian<IType, 32>` |  |

### `jalr`

Jumps to `rs1 + offset` (with bit 0 cleared) and puts the address of the
next instruction in `rd`. `jalr zero, ra, 0` returns from a function.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jalr rd, rs1, offset` | `rd: Reg`, `rs1: Reg`, `offset: int` | emits `LittleEndian<IType, 32>` |  |

### `ecall`

Calls the execution environment, e.g. a Linux system call.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ecall` |  | emits `LittleEndian<IType, 32>` |  |

### `ebreak`

Stops in a debugger.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ebreak` |  | emits `LittleEndian<IType, 32>` |  |

### `lb`

Loads the byte at `rs1 + offset` into `rd`, sign-extended.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lb rd, offset, rs1` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  |

### `lh`

Loads the 16-bit halfword at `rs1 + offset` into `rd`, sign-extended.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lh rd, offset, rs1` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  |

### `lw`

Loads the 32-bit word at `rs1 + offset` into `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lw rd, offset, rs1` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  |

### `lbu`

Loads the byte at `rs1 + offset` into `rd`, zero-extended.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lbu rd, offset, rs1` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  |

### `lhu`

Loads the 16-bit halfword at `rs1 + offset` into `rd`, zero-extended.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lhu rd, offset, rs1` | `rd: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<IType, 32>` |  |

### `sb`

Stores the low byte of `rs2` at `rs1 + offset`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sb rs2, offset, rs1` | `rs2: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<SType, 32>` |  |

### `sh`

Stores the low 16 bits of `rs2` at `rs1 + offset`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sh rs2, offset, rs1` | `rs2: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<SType, 32>` |  |

### `sw`

Stores `rs2` at `rs1 + offset`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sw rs2, offset, rs1` | `rs2: Reg`, `offset: int`, `rs1: Reg` | emits `LittleEndian<SType, 32>` |  |

### `beq`

Branches to the label `target` if `rs1 == rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `beq rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  |

### `bne`

Branches to the label `target` if `rs1 != rs2`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bne rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  |

### `blt`

Branches to the label `target` if `rs1 < rs2` as signed numbers.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `blt rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  |

### `bge`

Branches to the label `target` if `rs1 >= rs2` as signed numbers.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bge rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  |

### `bltu`

Branches to the label `target` if `rs1 < rs2` as unsigned numbers.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bltu rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  |

### `bgeu`

Branches to the label `target` if `rs1 >= rs2` as unsigned numbers.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bgeu rs1, rs2, target` | `rs1: Reg`, `rs2: Reg`, `target: int` | emits `LittleEndian<BType, 32>` |  |

### `lui`

`rd = imm << 12`: loads a 20-bit upper immediate.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lui rd, imm` | `rd: Reg`, `imm: int` | emits `LittleEndian<UType, 32>` |  |

### `auipc`

`rd = pc + (imm << 12)`: an address relative to this instruction.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `auipc rd, imm` | `rd: Reg`, `imm: int` | emits `LittleEndian<UType, 32>` |  |

### `jal`

Jumps to the label `target` and puts the address of the next instruction
in `rd`. `jal ra, f` calls `f`; `jal zero, l` just jumps.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jal rd, target` | `rd: Reg`, `target: int` | emits `LittleEndian<JType, 32>` |  |

## Types

### `Reg`

`type Reg = bits<5>`

A register number, 0 to 31.

### `Opcode`

`type Opcode = bits<7>`

The 7-bit major opcode, which picks the format and family.

### `Funct3`

`type Funct3 = bits<3>`

The 3-bit `funct3` field, which picks the instruction within a family.

### `Funct7`

`type Funct7 = bits<7>`

The 7-bit `funct7` field of an R-type instruction.

### `Bit1`

`type Bit1 = bits<1>`

A one-bit field.

### `Imm4`

`type Imm4 = bits<4>`

A 4-bit immediate field.

### `Imm5`

`type Imm5 = bits<5>`

A 5-bit immediate field.

### `Imm6`

`type Imm6 = bits<6>`

A 6-bit immediate field.

### `Imm7`

`type Imm7 = bits<7>`

A 7-bit immediate field.

### `Imm8`

`type Imm8 = bits<8>`

An 8-bit immediate field.

### `Imm10`

`type Imm10 = bits<10>`

A 10-bit immediate field.

### `Imm12`

`type Imm12 = bits<12>`

A 12-bit immediate field.

### `Imm20`

`type Imm20 = bits<20>`

A 20-bit immediate field.

### `RType`

`struct RType`

The R-type format: register-register operations.

### `IType`

`struct IType`

The I-type format: register-immediate operations, loads, `jalr` and
system calls.

### `SType`

`struct SType`

The S-type format: stores. The 12-bit offset is split around the
registers.

### `BType`

`struct BType`

The B-type format: conditional branches. The offset's bits are spread
across the word the way the spec lays them out, and resolved by
`bitter`.

### `UType`

`struct UType`

The U-type format: a 20-bit upper immediate.

### `JType`

`struct JType`

The J-type format: `jal`. Like B-type, its offset's bits are spread across
the word, and resolved by `bitter`.

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `x0` |  | `Reg(0)` | A general-purpose register. `x0` always reads as zero. |
| `x1` |  | `Reg(1)` | A general-purpose register. `x0` always reads as zero. |
| `x2` |  | `Reg(2)` | A general-purpose register. `x0` always reads as zero. |
| `x3` |  | `Reg(3)` | A general-purpose register. `x0` always reads as zero. |
| `x4` |  | `Reg(4)` | A general-purpose register. `x0` always reads as zero. |
| `x5` |  | `Reg(5)` | A general-purpose register. `x0` always reads as zero. |
| `x6` |  | `Reg(6)` | A general-purpose register. `x0` always reads as zero. |
| `x7` |  | `Reg(7)` | A general-purpose register. `x0` always reads as zero. |
| `x8` |  | `Reg(8)` | A general-purpose register. `x0` always reads as zero. |
| `x9` |  | `Reg(9)` | A general-purpose register. `x0` always reads as zero. |
| `x10` |  | `Reg(10)` | A general-purpose register. `x0` always reads as zero. |
| `x11` |  | `Reg(11)` | A general-purpose register. `x0` always reads as zero. |
| `x12` |  | `Reg(12)` | A general-purpose register. `x0` always reads as zero. |
| `x13` |  | `Reg(13)` | A general-purpose register. `x0` always reads as zero. |
| `x14` |  | `Reg(14)` | A general-purpose register. `x0` always reads as zero. |
| `x15` |  | `Reg(15)` | A general-purpose register. `x0` always reads as zero. |
| `x16` |  | `Reg(16)` | A general-purpose register. `x0` always reads as zero. |
| `x17` |  | `Reg(17)` | A general-purpose register. `x0` always reads as zero. |
| `x18` |  | `Reg(18)` | A general-purpose register. `x0` always reads as zero. |
| `x19` |  | `Reg(19)` | A general-purpose register. `x0` always reads as zero. |
| `x20` |  | `Reg(20)` | A general-purpose register. `x0` always reads as zero. |
| `x21` |  | `Reg(21)` | A general-purpose register. `x0` always reads as zero. |
| `x22` |  | `Reg(22)` | A general-purpose register. `x0` always reads as zero. |
| `x23` |  | `Reg(23)` | A general-purpose register. `x0` always reads as zero. |
| `x24` |  | `Reg(24)` | A general-purpose register. `x0` always reads as zero. |
| `x25` |  | `Reg(25)` | A general-purpose register. `x0` always reads as zero. |
| `x26` |  | `Reg(26)` | A general-purpose register. `x0` always reads as zero. |
| `x27` |  | `Reg(27)` | A general-purpose register. `x0` always reads as zero. |
| `x28` |  | `Reg(28)` | A general-purpose register. `x0` always reads as zero. |
| `x29` |  | `Reg(29)` | A general-purpose register. `x0` always reads as zero. |
| `x30` |  | `Reg(30)` | A general-purpose register. `x0` always reads as zero. |
| `x31` |  | `Reg(31)` | A general-purpose register. `x0` always reads as zero. |
| `zero` |  | `x0` | `x0`, which always reads as zero. |
| `ra` |  | `x1` | `x1`: the return address. |
| `sp` |  | `x2` | `x2`: the stack pointer. |
| `gp` |  | `x3` | `x3`: the global pointer. |
| `tp` |  | `x4` | `x4`: the thread pointer. |
| `t0` |  | `x5` | `x5`: a temporary. |
| `t1` |  | `x6` | `x6`: a temporary. |
| `t2` |  | `x7` | `x7`: a temporary. |
| `s0` |  | `x8` | `x8`: saved across calls. |
| `fp` |  | `x8` | `x8`: the frame pointer, the same register as `s0`. |
| `s1` |  | `x9` | `x9`: saved across calls. |
| `a0` |  | `x10` | `x10`: a function argument and return value. |
| `a1` |  | `x11` | `x11`: a function argument and return value. |
| `a2` |  | `x12` | `x12`: a function argument. |
| `a3` |  | `x13` | `x13`: a function argument. |
| `a4` |  | `x14` | `x14`: a function argument. |
| `a5` |  | `x15` | `x15`: a function argument. |
| `a6` |  | `x16` | `x16`: a function argument. |
| `a7` |  | `x17` | `x17`: a function argument. |
| `s2` |  | `x18` | `x18`: saved across calls. |
| `s3` |  | `x19` | `x19`: saved across calls. |
| `s4` |  | `x20` | `x20`: saved across calls. |
| `s5` |  | `x21` | `x21`: saved across calls. |
| `s6` |  | `x22` | `x22`: saved across calls. |
| `s7` |  | `x23` | `x23`: saved across calls. |
| `s8` |  | `x24` | `x24`: saved across calls. |
| `s9` |  | `x25` | `x25`: saved across calls. |
| `s10` |  | `x26` | `x26`: saved across calls. |
| `s11` |  | `x27` | `x27`: saved across calls. |
| `t3` |  | `x28` | `x28`: a temporary. |
| `t4` |  | `x29` | `x29`: a temporary. |
| `t5` |  | `x30` | `x30`: a temporary. |
| `t6` |  | `x31` | `x31`: a temporary. |
