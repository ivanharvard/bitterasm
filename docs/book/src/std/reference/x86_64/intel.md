# `intel`

[`std`](../index.md) › [`x86_64`](index.md) › `intel`

Intel-syntax x86-64 assembly: destination first, registers by name,
and memory operands in brackets.

```basm
from std.x86_64.intel import *

mov rax, [rsp+8]
add eax, 1
mov r8, [rbx+rcx*4+0x10]
```

```bytes
48 8b 44 24 08
81 c0 01 00 00 00
4c 8b 44 8b 10
```

The operand size comes from the register's name: `rax`, `r8` and the
other `Reg`s are 64-bit, and `eax`, `r8d` and the other `Reg32`s are
32-bit. Memory operands are `[base]`, `[base+disp]`,
`[base+index*scale+disp]` and `[rip+disp]`. Every instruction of
`std.x86_64.impl` stays available in its explicit form too, with the
operand size as a final argument: `mov rax, rbx, 0`.

For NASM's `[rel label]` and data directives, use `std.x86_64.nasm`.

Re-exports [`std.x86_64.impl`](impl.md).

## Macros

### `reg_field`

The low 3 bits of `r`'s number: the part a ModRM or SIB field, or an
opcode, holds.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `reg_field(r)` | `r: Reg` | returns `int` |  | [`std.x86_64.impl`](impl.md) |

### `reg_ext`

Bit 3 of `r`'s number, which goes in the REX prefix: 1 for `r8` to
`r15`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `reg_ext(r)` | `r: Reg` | returns `int` |  | [`std.x86_64.impl`](impl.md) |

### `rex_byte`

A REX prefix, `0100WRXB`. `w` selects 64-bit operands; `r`, `x` and `b`
are bit 3 of the ModRM.reg, SIB.index and ModRM.rm (or SIB.base, or
opcode) register numbers.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rex_byte(w, r, x, b)` | `w: int`, `r: int`, `x: int`, `b: int` | returns `Byte` |  | [`std.x86_64.impl`](impl.md) |

### `modrm_byte`

A ModRM byte: `mod` (2 bits), `reg` (3) and `rm` (3).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `modrm_byte(mod, reg, rm)` | `mod: int`, `reg: int`, `rm: int` | returns `Byte` |  | [`std.x86_64.impl`](impl.md) |

### `sib_byte`

A SIB byte: `scale` (2 bits, log2 of the index's multiplier), `index`
(3) and `base` (3).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sib_byte(scale, index, base)` | `scale: int`, `index: int`, `base: int` | returns `Byte` |  | [`std.x86_64.impl`](impl.md) |

### `byte_of`

Byte `index` of `value`, counting from the least significant, byte 0.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `byte_of(value, index)` | `value: int`, `index: int` | returns `Byte` |  | [`std.x86_64.impl`](impl.md) |

### `mov`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mov rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rs`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = imm`. With `w` = 1 it takes a full 64-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `mov dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Stores `rs` at `dst`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, src, w` | `rd: Reg`, `src: MemOperand`, `w: int` | emits `Bytes<...>` | Loads the value at `src` into `rd`. | [`std.x86_64.impl`](impl.md) |
| `mov dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Stores `imm` at `dst`: 32 bits, sign-extended to 64 when `w` is 1. | [`std.x86_64.impl`](impl.md) |
| `mov rd, src, w` | `rd: Reg`, `src: RipLabel`, `w: int` |  | Loads the value at the label `src` into `rd`. | [`std.x86_64.impl`](impl.md) |
| `mov dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Stores `rs` at the label `dst`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rs`. |  |
| `mov rd, imm` | `rd: Reg`, `imm: int` |  | `rd = imm`. |  |
| `mov rd, rs` | `rd: Reg32`, `rs: Reg32` |  | `rd = rs`, in 32 bits. |  |
| `mov rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = imm`, in 32 bits. |  |

### `add`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `add rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd + rs`. | [`std.x86_64.impl`](impl.md) |
| `add rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd + imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + rs`. | [`std.x86_64.impl`](impl.md) |
| `add dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] + rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `add rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rd + rs`. |  |
| `add rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd + imm`. |  |
| `add rd, rs` | `rd: Reg32`, `rs: Reg32` |  | `rd = rd + rs`, in 32 bits. |  |
| `add rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd + imm`, in 32 bits. |  |

### `or`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `or rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd \| rs`. | [`std.x86_64.impl`](impl.md) |
| `or rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd \| imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| rs`. | [`std.x86_64.impl`](impl.md) |
| `or dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] \| rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `or rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rd \| rs`. |  |
| `or rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd \| imm`. |  |
| `or rd, rs` | `rd: Reg32`, `rs: Reg32` |  | `rd = rd \| rs`, in 32 bits. |  |
| `or rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd \| imm`, in 32 bits. |  |

### `and`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `and rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd & rs`. | [`std.x86_64.impl`](impl.md) |
| `and rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd & imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & rs`. | [`std.x86_64.impl`](impl.md) |
| `and dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] & rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `and rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rd & rs`. |  |
| `and rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd & imm`. |  |
| `and rd, rs` | `rd: Reg32`, `rs: Reg32` |  | `rd = rd & rs`, in 32 bits. |  |
| `and rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd & imm`, in 32 bits. |  |

### `sub`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sub rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd - rs`. | [`std.x86_64.impl`](impl.md) |
| `sub rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd - imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - rs`. | [`std.x86_64.impl`](impl.md) |
| `sub dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] - rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `sub rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rd - rs`. |  |
| `sub rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd - imm`. |  |
| `sub rd, rs` | `rd: Reg32`, `rs: Reg32` |  | `rd = rd - rs`, in 32 bits. |  |
| `sub rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd - imm`, in 32 bits. |  |

### `xor`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `xor rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd ^ rs`. | [`std.x86_64.impl`](impl.md) |
| `xor rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd ^ imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ rs`. | [`std.x86_64.impl`](impl.md) |
| `xor dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] ^ rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `xor rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rd ^ rs`. |  |
| `xor rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd ^ imm`. |  |
| `xor rd, rs` | `rd: Reg32`, `rs: Reg32` |  | `rd = rd ^ rs`, in 32 bits. |  |
| `xor rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd ^ imm`, in 32 bits. |  |

### `cmp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmp rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] - rs`, where `dst` is a label, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp rd, rs` | `rd: Reg`, `rs: Reg` |  | Sets the flags from `rd - rs`, without storing it. |  |
| `cmp rd, imm` | `rd: Reg`, `imm: int` |  | Sets the flags from `rd - imm`, without storing it. |  |
| `cmp rd, rs` | `rd: Reg32`, `rs: Reg32` |  | Sets the flags from `rd - rs`, in 32 bits, without storing it. |  |
| `cmp rd, imm` | `rd: Reg32`, `imm: int` |  | Sets the flags from `rd - imm`, in 32 bits, without storing it. |  |

### `test`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `test rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] & rs`, where `dst` is a label, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test rd, rs` | `rd: Reg`, `rs: Reg` |  | Sets the flags from `rd & rs`, without storing it. |  |
| `test rd, imm` | `rd: Reg`, `imm: int` |  | Sets the flags from `rd & imm`, without storing it. |  |
| `test rd, rs` | `rd: Reg32`, `rs: Reg32` |  | Sets the flags from `rd & rs`, in 32 bits, without storing it. |  |
| `test rd, imm` | `rd: Reg32`, `imm: int` |  | Sets the flags from `rd & imm`, in 32 bits, without storing it. |  |

### `Mem`

The memory at `[base + disp]`. A displacement from -128 to 127 takes one
byte; any other takes four.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `Mem(base, disp)` | `base: Reg`, `disp: int` | returns `MemOperand` |  | [`std.x86_64.impl`](impl.md) |

### `MemIndexed`

The memory at `[base + index * scale + disp]`. `scale` is 1, 2, 4 or 8,
and `index` can be any register but `rsp`.

```basm
from std.x86_64.impl import *

mov rax, MemIndexed(rbx, r12, 4, 0), 1
```

```bytes
4a 8b 04 a3
```

```basm,fail
from std.x86_64.impl import *

mov rax, MemIndexed(rbx, rsp, 4, 0), 1
```

```error
rsp cannot be a SIB index register
```

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `MemIndexed(base, index, scale, disp)` | `base: Reg`, `index: Reg`, `scale: int`, `disp: int` | returns `MemOperand` |  | [`std.x86_64.impl`](impl.md) |

### `MemRipRelative`

The memory at `[rip + disp]`: `disp` bytes past the end of the
instruction. To address a label, use a `RipLabel` instead.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `MemRipRelative(disp)` | `disp: int` | returns `MemOperand` |  | [`std.x86_64.impl`](impl.md) |

### `jmp`

Jumps to the label `target`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jmp target` | `target: int` | emits `Rel32Instr` |  | [`std.x86_64.impl`](impl.md) |

### `call`

Pushes the address of the next instruction and jumps to the label
`target`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call target` | `target: int` | emits `Rel32Instr` |  | [`std.x86_64.impl`](impl.md) |

### `je`

Jumps to the label `target` if equal (ZF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `je target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jne`

Jumps to the label `target` if not equal (ZF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jne target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jb`

Jumps to the label `target` if below, unsigned (CF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jb target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jae`

Jumps to the label `target` if above or equal, unsigned (CF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jae target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `ja`

Jumps to the label `target` if above, unsigned.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ja target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jbe`

Jumps to the label `target` if below or equal, unsigned.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jbe target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jl`

Jumps to the label `target` if less, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jl target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jge`

Jumps to the label `target` if greater or equal, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jge target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jle`

Jumps to the label `target` if less or equal, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jle target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jg`

Jumps to the label `target` if greater, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jg target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `js`

Jumps to the label `target` if the result was negative (SF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `js target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jns`

Jumps to the label `target` if the result wasn't negative (SF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jns target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jo`

Jumps to the label `target` on signed overflow (OF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jo target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jno`

Jumps to the label `target` without signed overflow (OF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jno target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jp`

Jumps to the label `target` if the parity flag is set (PF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jp target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnp`

Jumps to the label `target` if the parity flag is clear (PF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnp target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jz`

`je`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jz target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnz`

`jne`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnz target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jc`

`jb`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jc target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnae`

`jb`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnae target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnc`

`jae`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnc target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnb`

`jae`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnb target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnbe`

`ja`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnbe target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jna`

`jbe`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jna target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnge`

`jl`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnge target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnl`

`jge`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnl target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jng`

`jle`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jng target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnle`

`jg`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnle target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jpe`

`jp`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jpe target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jpo`

`jnp`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jpo target` | `target: int` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `ret`

Returns: pops an address and jumps to it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ret` |  | emits `Byte` |  | [`std.x86_64.impl`](impl.md) |

### `shl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shl rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd << imm`. | [`std.x86_64.impl`](impl.md) |
| `shl rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd << imm`. |  |
| `shl rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd << imm`, in 32 bits. |  |

### `shl_cl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shl_cl(rd, w)` | `rd: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd << cl`. | [`std.x86_64.impl`](impl.md) |
| `shl rd, cl` | `rd: Reg` |  | `rd = rd << cl`. |  |
| `shl rd, cl` | `rd: Reg32` |  | `rd = rd << cl`, in 32 bits. |  |

### `shr`

`rd = rd >> imm`, shifting in zeros.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shr rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `shr_cl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shr_cl(rd, w)` | `rd: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd >> cl`, shifting in zeros. | [`std.x86_64.impl`](impl.md) |
| `shr rd, cl` | `rd: Reg` |  | `rd = rd >> cl`, shifting in zeros. |  |
| `shr rd, cl` | `rd: Reg32` |  | `rd = rd >> cl`, shifting in zeros, in 32 bits. |  |

### `sar`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sar rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in copies of the sign bit. | [`std.x86_64.impl`](impl.md) |
| `sar rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd >> imm`, shifting in copies of the sign bit. |  |
| `sar rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd >> imm`, shifting in copies of the sign bit, in 32 bits. |  |

### `sar_cl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sar_cl(rd, w)` | `rd: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd >> cl`, shifting in copies of the sign bit. | [`std.x86_64.impl`](impl.md) |
| `sar rd, cl` | `rd: Reg` |  | `rd = rd >> cl`, shifting in copies of the sign bit. |  |
| `sar rd, cl` | `rd: Reg32` |  | `rd = rd >> cl`, shifting in copies of the sign bit, in 32 bits. |  |

### `lea`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lea rd, src, w` | `rd: Reg`, `src: MemOperand`, `w: int` | emits `Bytes<...>` | Loads the address `src` stands for into `rd`, without reading memory. | [`std.x86_64.impl`](impl.md) |
| `lea rd, src, w` | `rd: Reg`, `src: RipLabel`, `w: int` |  | Loads the address of the label `src` into `rd`. | [`std.x86_64.impl`](impl.md) |

### `rip_label_instr`

An instruction with opcode `opcode` whose memory operand is the label
`addr`, and whose other operand is `r`. The dialects' `[rel label]`
forms are built on it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rip_label_instr opcode, addr, r, w` | `opcode: int`, `addr: RipLabel`, `r: Reg`, `w: int` |  |  | [`std.x86_64.impl`](impl.md) |

### `push`

Pushes the 64-bit `rd` onto the stack.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `push rd` | `rd: Reg` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `pop`

Pops 64 bits off the stack into `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pop rd` | `rd: Reg` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `syscall`

Calls the operating system. On Linux, `rax` holds the call number and
`rdi`, `rsi`, `rdx`, ... its arguments.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `syscall` |  | emits `Bytes<2>` |  | [`std.x86_64.impl`](impl.md) |

### `assert_valid_reg`

Fails to compile unless `r` is a register number, 0 to 15. The
instructions here use it to check their operands.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `assert_valid_reg r` | `r: Reg` |  |  |

### `mov_load_base`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov rd, [base]` | `rd: Reg`, `base: Reg` |  | Loads the value at `[base]` into `rd`. |
| `mov rd, [base]` | `rd: Reg32`, `base: Reg` |  | Loads the 32-bit value at `[base]` into `rd`. |

### `mov_load_base_disp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov rd, [base+disp]` | `rd: Reg`, `base: Reg`, `disp: int` |  | Loads the value at `[base + disp]` into `rd`. |
| `mov rd, [base+disp]` | `rd: Reg32`, `base: Reg`, `disp: int` |  | Loads the 32-bit value at `[base + disp]` into `rd`. |

### `mov_load_indexed`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov rd, [base+index*scale+disp]` | `rd: Reg`, `base: Reg`, `index: Reg`, `scale: int`, `disp: int` |  | Loads the value at `[base + index * scale + disp]` into `rd`. |
| `mov rd, [base+index*scale+disp]` | `rd: Reg32`, `base: Reg`, `index: Reg`, `scale: int`, `disp: int` |  | Loads the 32-bit value at `[base + index * scale + disp]` into `rd`. |

### `mov_load_rip`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov rd, [rip+disp]` | `rd: Reg`, `disp: int` |  | Loads the value at `[rip + disp]`, `disp` bytes past the end of the instruction, into `rd`. |
| `mov rd, [rip+disp]` | `rd: Reg32`, `disp: int` |  | Loads the 32-bit value at `[rip + disp]`, `disp` bytes past the end of the instruction, into `rd`. |

### `mov_store_base`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov [base], rs` | `base: Reg`, `rs: Reg` |  | Stores `rs` at `[base]`. |
| `mov [base], rs` | `base: Reg`, `rs: Reg32` |  | Stores the 32-bit `rs` at `[base]`. |

### `mov_store_base_disp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov [base+disp], rs` | `base: Reg`, `disp: int`, `rs: Reg` |  | Stores `rs` at `[base + disp]`. |
| `mov [base+disp], rs` | `base: Reg`, `disp: int`, `rs: Reg32` |  | Stores the 32-bit `rs` at `[base + disp]`. |

### `mov_store_indexed`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov [base+index*scale+disp], rs` | `base: Reg`, `index: Reg`, `scale: int`, `disp: int`, `rs: Reg` |  | Stores `rs` at `[base + index * scale + disp]`. |
| `mov [base+index*scale+disp], rs` | `base: Reg`, `index: Reg`, `scale: int`, `disp: int`, `rs: Reg32` |  | Stores the 32-bit `rs` at `[base + index * scale + disp]`. |

### `mov_store_rip`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov [rip+disp], rs` | `disp: int`, `rs: Reg` |  | Stores `rs` at `[rip + disp]`, `disp` bytes past the end of the instruction. |
| `mov [rip+disp], rs` | `disp: int`, `rs: Reg32` |  | Stores the 32-bit `rs` at `[rip + disp]`, `disp` bytes past the end of the instruction. |

### `lea_base`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lea rd, [base]` | `rd: Reg`, `base: Reg` |  | Loads the address `[base]` into `rd`, without reading memory. |
| `lea rd, [base]` | `rd: Reg32`, `base: Reg` |  | Loads the address `[base]` into the 32-bit `rd`, without reading memory. |

### `lea_base_disp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lea rd, [base+disp]` | `rd: Reg`, `base: Reg`, `disp: int` |  | Loads the address `[base + disp]` into `rd`, without reading memory. |
| `lea rd, [base+disp]` | `rd: Reg32`, `base: Reg`, `disp: int` |  | Loads the address `[base + disp]` into the 32-bit `rd`, without reading memory. |

### `lea_indexed`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lea rd, [base+index*scale+disp]` | `rd: Reg`, `base: Reg`, `index: Reg`, `scale: int`, `disp: int` |  | Loads the address `[base + index * scale + disp]` into `rd`, without reading memory. |
| `lea rd, [base+index*scale+disp]` | `rd: Reg32`, `base: Reg`, `index: Reg`, `scale: int`, `disp: int` |  | Loads the address `[base + index * scale + disp]` into the 32-bit `rd`, without reading memory. |

### `lea_rip`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lea rd, [rip+disp]` | `rd: Reg`, `disp: int` |  | Loads the address `[rip + disp]`, `disp` bytes past the end of the instruction, into `rd`. |
| `lea rd, [rip+disp]` | `rd: Reg32`, `disp: int` |  | Loads the address `[rip + disp]`, `disp` bytes past the end of the instruction, into the 32-bit `rd`. |

### `shr_imm`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shr rd, imm` | `rd: Reg`, `imm: int` |  | `rd = rd >> imm`, shifting in zeros. |
| `shr rd, imm` | `rd: Reg32`, `imm: int` |  | `rd = rd >> imm`, shifting in zeros, in 32 bits. |

## Re-exported

From [`std.x86_64.impl`](impl.md): `Reg`, `r0`, `r1`, `r2`, `r3`, `r4`, `r5`, `r6`, `r7`, `r8`, `r9`, `r10`, `r11`, `r12`, `r13`, `r14`, `r15`, `rax`, `rcx`, `rdx`, `rbx`, `rsp`, `rbp`, `rsi`, `rdi`, `Reg32`, `eax`, `ecx`, `edx`, `ebx`, `esp`, `ebp`, `esi`, `edi`, `r8d`, `r9d`, `r10d`, `r11d`, `r12d`, `r13d`, `r14d`, `r15d`, `Byte`, `Bytes`, `MemBase`, `MemSib`, `MemRip`, `MemOperand`, `Rel32Instr`, `Rel32Instr2`, `RipLabel`, `RipRelInstr`, `RipRelInstrRex`.
