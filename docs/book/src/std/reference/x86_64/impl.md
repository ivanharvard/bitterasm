# `impl`

[`std`](../index.md) › [`x86_64`](index.md) › `impl`

The x86-64 base instruction set: general-purpose registers, memory
operands, and the core integer instructions, each emitted as its machine
code.

Import a dialect rather than this module: `std.x86_64.intel`,
`std.x86_64.nasm` or `std.x86_64.att`. Here every instruction takes its
operands in order, destination first, plus a final `w`: 1 for 64-bit
operands and 0 for 32-bit. The dialects pick `w` from the register's
name instead (`rax` or `eax`).

```basm
from std.x86_64.impl import *

mov rax, rbx, 1
mov rax, Mem(rsp, 8), 1

loop:
    sub rcx, 1, 1
    jne loop
```

```bytes
48 89 d8
48 8b 44 24 08
48 81 e9 01 00 00 00
0f 85 f3 ff ff ff
```

Jumps and calls always use a 32-bit offset, worked out by `bitter` once
the program is laid out; there's no automatic choice of the short form.

## Macros

### `reg_field`

The low 3 bits of `r`'s number: the part a ModRM or SIB field, or an
opcode, holds.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `reg_field(r)` | `r: Reg` | returns `int` |  |

### `reg_ext`

Bit 3 of `r`'s number, which goes in the REX prefix: 1 for `r8` to
`r15`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `reg_ext(r)` | `r: Reg` | returns `int` |  |

### `rex_byte`

A REX prefix, `0100WRXB`. `w` selects 64-bit operands; `r`, `x` and `b`
are bit 3 of the ModRM.reg, SIB.index and ModRM.rm (or SIB.base, or
opcode) register numbers.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rex_byte(w, r, x, b)` | `w: int`, `r: int`, `x: int`, `b: int` | returns `Byte` |  |

### `modrm_byte`

A ModRM byte: `mod` (2 bits), `reg` (3) and `rm` (3).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `modrm_byte(mod, reg, rm)` | `mod: int`, `reg: int`, `rm: int` | returns `Byte` |  |

### `sib_byte`

A SIB byte: `scale` (2 bits, log2 of the index's multiplier), `index`
(3) and `base` (3).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sib_byte(scale, index, base)` | `scale: int`, `index: int`, `base: int` | returns `Byte` |  |

### `byte_of`

Byte `index` of `value`, counting from the least significant, byte 0.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `byte_of(value, index)` | `value: int`, `index: int` | returns `Byte` |  |

### `mov`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rs`. |
| `mov rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = imm`. With `w` = 1 it takes a full 64-bit `imm`. |
| `mov dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Stores `rs` at `dst`. |
| `mov rd, src, w` | `rd: Reg`, `src: MemOperand`, `w: int` | emits `Bytes<...>` | Loads the value at `src` into `rd`. |
| `mov dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Stores `imm` at `dst`: 32 bits, sign-extended to 64 when `w` is 1. |
| `mov rd, src, w` | `rd: Reg`, `src: RipLabel`, `w: int` |  | Loads the value at the label `src` into `rd`. |
| `mov dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Stores `rs` at the label `dst`. |

### `add`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `add rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd + rs`. |
| `add rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd + imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `add dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + rs`. |
| `add dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + imm`, with a 32-bit `imm`. |
| `add dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] + rs`, where `dst` is a label. |

### `or`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `or rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd \| rs`. |
| `or rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd \| imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `or dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| rs`. |
| `or dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| imm`, with a 32-bit `imm`. |
| `or dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] \| rs`, where `dst` is a label. |

### `and`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `and rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd & rs`. |
| `and rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd & imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `and dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & rs`. |
| `and dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & imm`, with a 32-bit `imm`. |
| `and dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] & rs`, where `dst` is a label. |

### `sub`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sub rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd - rs`. |
| `sub rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd - imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `sub dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - rs`. |
| `sub dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - imm`, with a 32-bit `imm`. |
| `sub dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] - rs`, where `dst` is a label. |

### `xor`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xor rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd ^ rs`. |
| `xor rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd ^ imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `xor dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ rs`. |
| `xor dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ imm`, with a 32-bit `imm`. |
| `xor dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] ^ rs`, where `dst` is a label. |

### `cmp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmp rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - rs`, without storing it. |
| `cmp rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - imm`, without storing it. |
| `cmp dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - rs`, without storing it. |
| `cmp dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - imm`, without storing it. |
| `cmp dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] - rs`, where `dst` is a label, without storing it. |

### `test`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `test rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & rs`, without storing it. |
| `test rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & imm`, without storing it. |
| `test dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & rs`, without storing it. |
| `test dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & imm`, without storing it. |
| `test dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] & rs`, where `dst` is a label, without storing it. |

### `Mem`

The memory at `[base + disp]`. A displacement from -128 to 127 takes one
byte; any other takes four.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `Mem(base, disp)` | `base: Reg`, `disp: int` | returns `MemOperand` |  |

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

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `MemIndexed(base, index, scale, disp)` | `base: Reg`, `index: Reg`, `scale: int`, `disp: int` | returns `MemOperand` |  |

### `MemRipRelative`

The memory at `[rip + disp]`: `disp` bytes past the end of the
instruction. To address a label, use a `RipLabel` instead.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `MemRipRelative(disp)` | `disp: int` | returns `MemOperand` |  |

### `jmp`

Jumps to the label `target`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jmp target` | `target: int` | emits `Rel32Instr` |  |

### `call`

Pushes the address of the next instruction and jumps to the label
`target`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `call target` | `target: int` | emits `Rel32Instr` |  |

### `je`

Jumps to the label `target` if equal (ZF = 1).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `je target` | `target: int` | emits `Rel32Instr2` |  |

### `jne`

Jumps to the label `target` if not equal (ZF = 0).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jne target` | `target: int` | emits `Rel32Instr2` |  |

### `jb`

Jumps to the label `target` if below, unsigned (CF = 1).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jb target` | `target: int` | emits `Rel32Instr2` |  |

### `jae`

Jumps to the label `target` if above or equal, unsigned (CF = 0).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jae target` | `target: int` | emits `Rel32Instr2` |  |

### `ja`

Jumps to the label `target` if above, unsigned.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ja target` | `target: int` | emits `Rel32Instr2` |  |

### `jbe`

Jumps to the label `target` if below or equal, unsigned.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jbe target` | `target: int` | emits `Rel32Instr2` |  |

### `jl`

Jumps to the label `target` if less, signed.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jl target` | `target: int` | emits `Rel32Instr2` |  |

### `jge`

Jumps to the label `target` if greater or equal, signed.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jge target` | `target: int` | emits `Rel32Instr2` |  |

### `jle`

Jumps to the label `target` if less or equal, signed.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jle target` | `target: int` | emits `Rel32Instr2` |  |

### `jg`

Jumps to the label `target` if greater, signed.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jg target` | `target: int` | emits `Rel32Instr2` |  |

### `js`

Jumps to the label `target` if the result was negative (SF = 1).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `js target` | `target: int` | emits `Rel32Instr2` |  |

### `jns`

Jumps to the label `target` if the result wasn't negative (SF = 0).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jns target` | `target: int` | emits `Rel32Instr2` |  |

### `jo`

Jumps to the label `target` on signed overflow (OF = 1).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jo target` | `target: int` | emits `Rel32Instr2` |  |

### `jno`

Jumps to the label `target` without signed overflow (OF = 0).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jno target` | `target: int` | emits `Rel32Instr2` |  |

### `jp`

Jumps to the label `target` if the parity flag is set (PF = 1).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jp target` | `target: int` | emits `Rel32Instr2` |  |

### `jnp`

Jumps to the label `target` if the parity flag is clear (PF = 0).

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnp target` | `target: int` | emits `Rel32Instr2` |  |

### `jz`

`je`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jz target` | `target: int` | emits `Rel32Instr2` |  |

### `jnz`

`jne`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnz target` | `target: int` | emits `Rel32Instr2` |  |

### `jc`

`jb`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jc target` | `target: int` | emits `Rel32Instr2` |  |

### `jnae`

`jb`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnae target` | `target: int` | emits `Rel32Instr2` |  |

### `jnc`

`jae`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnc target` | `target: int` | emits `Rel32Instr2` |  |

### `jnb`

`jae`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnb target` | `target: int` | emits `Rel32Instr2` |  |

### `jnbe`

`ja`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnbe target` | `target: int` | emits `Rel32Instr2` |  |

### `jna`

`jbe`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jna target` | `target: int` | emits `Rel32Instr2` |  |

### `jnge`

`jl`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnge target` | `target: int` | emits `Rel32Instr2` |  |

### `jnl`

`jge`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnl target` | `target: int` | emits `Rel32Instr2` |  |

### `jng`

`jle`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jng target` | `target: int` | emits `Rel32Instr2` |  |

### `jnle`

`jg`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jnle target` | `target: int` | emits `Rel32Instr2` |  |

### `jpe`

`jp`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jpe target` | `target: int` | emits `Rel32Instr2` |  |

### `jpo`

`jnp`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jpo target` | `target: int` | emits `Rel32Instr2` |  |

### `ret`

Returns: pops an address and jumps to it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ret` |  | emits `Byte` |  |

### `shl`

`rd = rd << imm`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shl rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` |  |

### `shl_cl`

`rd = rd << cl`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shl_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  |

### `shr`

`rd = rd >> imm`, shifting in zeros.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shr rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` |  |

### `shr_cl`

`rd = rd >> cl`, shifting in zeros.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shr_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  |

### `sar`

`rd = rd >> imm`, shifting in copies of the sign bit.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sar rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` |  |

### `sar_cl`

`rd = rd >> cl`, shifting in copies of the sign bit.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sar_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  |

### `lea`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lea rd, src, w` | `rd: Reg`, `src: MemOperand`, `w: int` | emits `Bytes<...>` | Loads the address `src` stands for into `rd`, without reading memory. |
| `lea rd, src, w` | `rd: Reg`, `src: RipLabel`, `w: int` |  | Loads the address of the label `src` into `rd`. |

### `rip_label_instr`

An instruction with opcode `opcode` whose memory operand is the label
`addr`, and whose other operand is `r`. The dialects' `[rel label]`
forms are built on it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rip_label_instr opcode, addr, r, w` | `opcode: int`, `addr: RipLabel`, `r: Reg`, `w: int` |  |  |

### `push`

Pushes the 64-bit `rd` onto the stack.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `push rd` | `rd: Reg` | emits `Bytes<...>` |  |

### `pop`

Pops 64 bits off the stack into `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pop rd` | `rd: Reg` | emits `Bytes<...>` |  |

### `syscall`

Calls the operating system. On Linux, `rax` holds the call number and
`rdi`, `rsi`, `rdx`, ... its arguments.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `syscall` |  | emits `Bytes<2>` |  |

## Types

### `Reg`

`type Reg = bits<4>`

A register number, 0 to 15. As an operand it means the whole 64-bit
register.

### `Reg32`

`struct Reg32`

The low 32 bits of a register. An instruction given one works on 32 bits
instead of 64, and writing one zeroes the register's upper half.

| Field | Type | Description |
|---|---|---|
| `reg` | `Reg` | The register it's the low half of. |

### `Byte`

`type Byte = bits<8>`

A byte.

### `Bytes`

`struct Bytes<const N: int>`

`N` bytes, packed by `bitter` in order: an instruction's encoding.

Some of its fields are generated by `@for` or `@if`.

### `MemBase`

`struct MemBase`

`[base + disp]`. `Mem` builds one.

| Field | Type | Description |
|---|---|---|
| `base` | `Reg` | The base register. |
| `disp` | `int` | The displacement added to it. |

### `MemSib`

`struct MemSib`

`[base + index * scale + disp]`. `MemIndexed` builds one.

| Field | Type | Description |
|---|---|---|
| `base` | `Reg` | The base register. |
| `index` | `Reg` | The index register. |
| `scale` | `int` | The index's multiplier: 1, 2, 4 or 8. |
| `disp` | `int` | The displacement. |

### `MemRip`

`struct MemRip`

`[rip + disp]`. `MemRipRelative` builds one.

| Field | Type | Description |
|---|---|---|
| `disp` | `int` | The displacement from the end of the instruction. |

### `MemOperand`

`enum MemOperand`

A memory operand, built by `Mem`, `MemIndexed` or `MemRipRelative`.

| Variant | Payload | Description |
|---|---|---|
| `Base` | `MemBase` | `[base + disp]`. |
| `Indexed` | `MemSib` | `[base + index * scale + disp]`. |
| `RipRelative` | `MemRip` | `[rip + disp]`. |

### `Rel32Instr`

`struct Rel32Instr`

A `jmp` or `call`: an opcode byte and a 32-bit offset `bitter` works out.

| Field | Type | Description |
|---|---|---|
| `opcode` | `Byte` | The opcode. |
| `rel32` | `LittleEndian<Positioned<32>, 32>` | The offset from the next instruction to the target. |

### `Rel32Instr2`

`struct Rel32Instr2`

A conditional jump: two opcode bytes and a 32-bit offset `bitter` works
out.

| Field | Type | Description |
|---|---|---|
| `opcode1` | `Byte` | The first opcode byte, `0x0F`. |
| `opcode2` | `Byte` | The second opcode byte, which holds the condition. |
| `rel32` | `LittleEndian<Positioned<32>, 32>` | The offset from the next instruction to the target. |

### `RipLabel`

`struct RipLabel`

A label used as a memory operand, addressed relative to the next
instruction: NASM's `[rel label]`.

| Field | Type | Description |
|---|---|---|
| `target` | `int` | The label. |

### `RipRelInstr`

`struct RipRelInstr`

An instruction with a `RipLabel` operand and no REX prefix.

| Field | Type | Description |
|---|---|---|
| `opcode` | `Byte` | The opcode. |
| `modrm` | `Byte` | The ModRM byte, with `rm` = RIP-relative. |
| `disp32` | `LittleEndian<Positioned<32>, 32>` | The displacement from the next instruction to the label. |

### `RipRelInstrRex`

`struct RipRelInstrRex`

An instruction with a `RipLabel` operand and a REX prefix.

| Field | Type | Description |
|---|---|---|
| `rex` | `Byte` | The REX prefix. |
| `opcode` | `Byte` | The opcode. |
| `modrm` | `Byte` | The ModRM byte, with `rm` = RIP-relative. |
| `disp32` | `LittleEndian<Positioned<32>, 32>` | The displacement from the next instruction to the label. |

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `r0` | `Reg` | `0` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r1` | `Reg` | `1` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r2` | `Reg` | `2` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r3` | `Reg` | `3` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r4` | `Reg` | `4` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r5` | `Reg` | `5` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r6` | `Reg` | `6` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r7` | `Reg` | `7` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r8` | `Reg` | `8` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r9` | `Reg` | `9` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r10` | `Reg` | `10` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r11` | `Reg` | `11` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r12` | `Reg` | `12` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r13` | `Reg` | `13` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r14` | `Reg` | `14` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `r15` | `Reg` | `15` | A 64-bit general-purpose register. `r0` to `r7` are better known as `rax` to `rdi`. |
| `rax` |  | `r0` | `r0`, the accumulator: where a function returns its result. |
| `rcx` |  | `r1` | `r1`, the counter: a function's fourth argument. |
| `rdx` |  | `r2` | `r2`: a function's third argument. |
| `rbx` |  | `r3` | `r3`, saved across calls. |
| `rsp` |  | `r4` | `r4`, the stack pointer. |
| `rbp` |  | `r5` | `r5`, the frame pointer, saved across calls. |
| `rsi` |  | `r6` | `r6`: a function's second argument. |
| `rdi` |  | `r7` | `r7`: a function's first argument. |
| `eax` |  | `Reg32(reg = rax)` | The low 32 bits of `rax`. |
| `ecx` |  | `Reg32(reg = rcx)` | The low 32 bits of `rcx`. |
| `edx` |  | `Reg32(reg = rdx)` | The low 32 bits of `rdx`. |
| `ebx` |  | `Reg32(reg = rbx)` | The low 32 bits of `rbx`. |
| `esp` |  | `Reg32(reg = rsp)` | The low 32 bits of `rsp`. |
| `ebp` |  | `Reg32(reg = rbp)` | The low 32 bits of `rbp`. |
| `esi` |  | `Reg32(reg = rsi)` | The low 32 bits of `rsi`. |
| `edi` |  | `Reg32(reg = rdi)` | The low 32 bits of `rdi`. |
| `r8d` |  | `Reg32(reg = r8)` | The low 32 bits of `r8`. |
| `r9d` |  | `Reg32(reg = r9)` | The low 32 bits of `r9`. |
| `r10d` |  | `Reg32(reg = r10)` | The low 32 bits of `r10`. |
| `r11d` |  | `Reg32(reg = r11)` | The low 32 bits of `r11`. |
| `r12d` |  | `Reg32(reg = r12)` | The low 32 bits of `r12`. |
| `r13d` |  | `Reg32(reg = r13)` | The low 32 bits of `r13`. |
| `r14d` |  | `Reg32(reg = r14)` | The low 32 bits of `r14`. |
| `r15d` |  | `Reg32(reg = r15)` | The low 32 bits of `r15`. |
