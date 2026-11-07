# `impl`

[`std`](../index.md) › [`x86_64`](index.md) › `impl`

The x86-64 base instruction set: general-purpose registers, memory
operands, and the core integer instructions, each emitted as its machine
code.

Coverage: 299 of 301 operand forms (99%), through `std.x86_64.nasm`.

Import a dialect rather than this module: `std.x86_64.intel`,
`std.x86_64.nasm` or `std.x86_64.att`. Here every instruction takes its
operands in order, destination first. The operand size comes from the
register: `rax` is 64 bits, `eax` 32, `ax` 16 and `al` 8. A memory
operand with no register to size it takes a final `OperandSize`: `byte`,
`word`, `dword`, `qword`, or `oword` for `cmpxchg16b`.

```basm
from std.x86_64.impl import *

mov rax, rbx
mov eax, Mem(rsp, 8)
add Mem(rbx, 0), 1, qword

again:
    sub rcx, 1
    jne again
```

```bytes
48 89 d8
8b 44 24 08
48 81 03 01 00 00 00
48 81 e9 01 00 00 00
0f 85 f3 ff ff ff
```

The first instructions written here also have a form that takes the
size as a final `w` instead, 1 for 64 bits and 0 for 32, with every
register a `Reg`: `mov rax, rbx, 0` is `mov eax, ebx`. `std.x86_64.att`
uses those.

Jumps and calls use a 32-bit offset, worked out by `bitter` once the
program is laid out; there's no automatic choice of the short form.
`loop`, `jrcxz` and `jecxz` have only the 8-bit one.

## Macros

### `gpr`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `gpr(r)` | `r: Reg` | returns `Gpr` | `r` as a 64-bit `Gpr`. |
| `gpr(r)` | `r: Reg32` | returns `Gpr` | `r` as a 32-bit `Gpr`. |
| `gpr(r)` | `r: Reg16` | returns `Gpr` | `r` as a 16-bit `Gpr`. |
| `gpr(r)` | `r: Reg8` | returns `Gpr` | `r` as an 8-bit `Gpr`. |
| `gpr(_x)` | `_x: int` | returns `Gpr` | Fails: an `int` where an instruction expects a register. |
| `gpr(_m)` | `_m: MemOperand` | returns `Gpr` | Fails: a memory operand here needs its size written out. |

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
| `mov rd, src` | `rd: Reg`, `src: RipLabel` |  | Loads the value at the label `src` into `rd`. |
| `mov rd, src` | `rd: Reg32`, `src: RipLabel` |  |  |
| `mov dst, rs` | `dst: RipLabel`, `rs: Reg` |  | Stores `rs` at the label `dst`. |
| `mov dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `mov rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs`. |
| `mov rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = imm`. A 64-bit `rd` takes a full 64-bit `imm`. |
| `mov rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd`. |
| `mov rd, src, size` | `<R>`, `rd: R`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads the value at `src` into `rd`; `size` must match `rd`'s. |
| `mov dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Stores `rs` at `dst`. |
| `mov dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Stores `imm` at `dst`, `size` wide. A 64-bit store takes a 32-bit `imm`, sign-extended. |
| `mov _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `mov [dst], imm` needs a size, like `mov qword [rbx], 1`. |
| `mov rd, sr` | `<R>`, `rd: R`, `sr: SReg` | emits `Bytes<...>` | `rd = sr`. |
| `mov sr, rs` | `<R>`, `sr: SReg`, `rs: R` | emits `Bytes<...>` | Loads `sr` from the low 16 bits of `rs`. |
| `mov dst, sr` | `dst: MemOperand`, `sr: SReg` | emits `Bytes<...>` | Stores `sr` at `dst`, 16 bits wide. |
| `mov dst, sr, size` | `dst: MemOperand`, `sr: SReg`, `size: OperandSize` | emits `Bytes<...>` | Stores `sr` at `dst`; `size` must be `word`. |
| `mov sr, src` | `sr: SReg`, `src: MemOperand` | emits `Bytes<...>` | Loads `sr` from the 16 bits at `src`. |
| `mov sr, src, size` | `sr: SReg`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads `sr` from `src`; `size` must be `word`. |

### `add`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `add rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd + rs`. |
| `add rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd + imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `add dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + rs`. |
| `add dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + imm`, with a 32-bit `imm`. |
| `add dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] + rs`, where `dst` is a label. |
| `add dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] + rs`, where `dst` is a label. |
| `add dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `add rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd + rs`. |
| `add rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd + imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `add rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd + [src]`. |
| `add dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] + rs`. |
| `add dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + imm`, `size` wide. |
| `add _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `add [dst], imm` needs a size, like `add qword [rbx], 1`. |

### `or`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `or rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd \| rs`. |
| `or rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd \| imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `or dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| rs`. |
| `or dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| imm`, with a 32-bit `imm`. |
| `or dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] \| rs`, where `dst` is a label. |
| `or dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] \| rs`, where `dst` is a label. |
| `or dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `or rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd \| rs`. |
| `or rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd \| imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `or rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd \| [src]`. |
| `or dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] \| rs`. |
| `or dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] \| imm`, `size` wide. |
| `or _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `or [dst], imm` needs a size, like `or qword [rbx], 1`. |

### `and`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `and rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd & rs`. |
| `and rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd & imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `and dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & rs`. |
| `and dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & imm`, with a 32-bit `imm`. |
| `and dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] & rs`, where `dst` is a label. |
| `and dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] & rs`, where `dst` is a label. |
| `and dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `and rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd & rs`. |
| `and rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd & imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `and rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd & [src]`. |
| `and dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] & rs`. |
| `and dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] & imm`, `size` wide. |
| `and _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `and [dst], imm` needs a size, like `and qword [rbx], 1`. |

### `sub`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sub rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd - rs`. |
| `sub rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd - imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `sub dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - rs`. |
| `sub dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - imm`, with a 32-bit `imm`. |
| `sub dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] - rs`, where `dst` is a label. |
| `sub dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] - rs`, where `dst` is a label. |
| `sub dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `sub rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd - rs`. |
| `sub rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd - imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `sub rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd - [src]`. |
| `sub dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] - rs`. |
| `sub dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - imm`, `size` wide. |
| `sub _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `sub [dst], imm` needs a size, like `sub qword [rbx], 1`. |

### `xor`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xor rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd ^ rs`. |
| `xor rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd ^ imm`, with a 32-bit `imm` sign-extended to 64 bits. |
| `xor dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ rs`. |
| `xor dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ imm`, with a 32-bit `imm`. |
| `xor dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] ^ rs`, where `dst` is a label. |
| `xor dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] ^ rs`, where `dst` is a label. |
| `xor dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `xor rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd ^ rs`. |
| `xor rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd ^ imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `xor rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd ^ [src]`. |
| `xor dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] ^ rs`. |
| `xor dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] ^ imm`, `size` wide. |
| `xor _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `xor [dst], imm` needs a size, like `xor qword [rbx], 1`. |

### `cmp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmp rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - rs`, without storing it. |
| `cmp rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - imm`, without storing it. |
| `cmp dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - rs`, without storing it. |
| `cmp dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - imm`, without storing it. |
| `cmp dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] - rs`, where `dst` is a label, without storing it. |
| `cmp dst, rs` | `dst: RipLabel`, `rs: Reg` |  | Sets the flags from `[dst] - rs`, where `dst` is a label, without storing it. |
| `cmp dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `cmp rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets the flags from `rd - rs`, without storing it. |
| `cmp rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Sets the flags from `rd - imm`, without storing it; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `cmp rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets the flags from `rd - [src]`, without storing it. |
| `cmp dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Sets the flags from `[dst] - rs`, without storing it. |
| `cmp dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Sets the flags from `[dst] - imm`, without storing it, `size` wide. |
| `cmp _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `cmp [dst], imm` needs a size, like `cmp qword [rbx], 1`. |

### `test`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `test rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & rs`, without storing it. |
| `test rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & imm`, without storing it. |
| `test dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & rs`, without storing it. |
| `test dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & imm`, without storing it. |
| `test dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] & rs`, where `dst` is a label, without storing it. |
| `test dst, rs` | `dst: RipLabel`, `rs: Reg` |  | Sets the flags from `[dst] & rs`, where `dst` is a label, without storing it. |
| `test dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  |
| `test rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets the flags from `rd & rs`, without storing it. |
| `test rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Sets the flags from `rd & imm`, without storing it. |
| `test rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets the flags from `rd & [src]`, without storing it. |
| `test dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Sets the flags from `[dst] & rs`, without storing it. |
| `test dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Sets the flags from `[dst] & imm`, `size` wide, without storing it. |
| `test _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `test [dst], imm` needs a size, like `test qword [rbx], 1`. |

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

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jmp target` | `target: int` | emits `Rel32Instr` | Jumps to the label `target`. |
| `jmp r` | `r: Reg` | emits `Bytes<...>` | Jumps to the address in `r`. |
| `jmp src` | `src: MemOperand` | emits `Bytes<...>` | Jumps to the address stored at `src`. |
| `jmp src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `jmp src`; `size` must be `qword`. |

### `call`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `call target` | `target: int` | emits `Rel32Instr` | Pushes the address of the next instruction and jumps to the label `target`. |
| `call r` | `r: Reg` | emits `Bytes<...>` | Calls the address in `r`. |
| `call src` | `src: MemOperand` | emits `Bytes<...>` | Calls the address stored at `src`. |
| `call src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `call src`; `size` must be `qword`. |

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

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ret` |  | emits `Byte` | Returns: pops an address and jumps to it. |
| `ret n` | `n: int` | emits `Bytes<...>` | Returns, then pops `n` more bytes. |

### `shl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shl rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd << imm`. |
| `shl rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd << imm`. |
| `shl rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | `rd = rd << cl`. |
| `shl dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] << imm`, `size` wide. |
| `shl dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] << cl`, `size` wide. |

### `shl_cl`

`rd = rd << cl`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shl_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  |

### `shr`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shr rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in zeros. |
| `shr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in zeros. |
| `shr rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | `rd = rd >> cl`, shifting in zeros. |
| `shr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> imm`, shifting in zeros, `size` wide. |
| `shr dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> cl`, shifting in zeros, `size` wide. |

### `shr_cl`

`rd = rd >> cl`, shifting in zeros.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shr_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  |

### `sar`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sar rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in copies of the sign bit. |
| `sar rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in copies of the sign bit. |
| `sar rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | `rd = rd >> cl`, shifting in copies of the sign bit. |
| `sar dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> imm`, shifting in copies of the sign bit, `size` wide. |
| `sar dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> cl`, shifting in copies of the sign bit, `size` wide. |

### `sar_cl`

`rd = rd >> cl`, shifting in copies of the sign bit.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sar_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  |

### `lea`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lea rd, src, w` | `rd: Reg`, `src: MemOperand`, `w: int` | emits `Bytes<...>` | Loads the address `src` stands for into `rd`, without reading memory. |
| `lea rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the address `src` stands for into `rd`, without reading memory. |
| `lea rd, src, w` | `rd: Reg`, `src: RipLabel`, `w: int` |  | Loads the address of the label `src` into `rd`. |
| `lea rd, src` | `rd: Reg`, `src: RipLabel` |  | Loads the address of the label `src` into `rd`. |
| `lea rd, src` | `rd: Reg32`, `src: RipLabel` |  |  |

### `rip_label_instr`

An instruction with opcode `opcode` whose memory operand is the label
`addr`, and whose other operand is `r`. The dialects' `[rel label]`
forms are built on it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rip_label_instr opcode, addr, r, w` | `opcode: int`, `addr: RipLabel`, `r: Reg`, `w: int` |  |  |

### `push`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `push rd` | `rd: Reg` | emits `Bytes<...>` | Pushes the 64-bit `rd` onto the stack. |
| `push rd` | `rd: Reg16` | emits `Bytes<...>` | Pushes the 16-bit `rd`. |
| `push imm` | `imm: int` | emits `Bytes<...>` | Pushes `imm`, a 32-bit immediate sign-extended to 64 bits. |
| `push src` | `src: MemOperand` | emits `Bytes<...>` | Pushes the 64 bits at `src`. |
| `push src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Pushes the `size`-wide value at `src`: `qword` or `word`. |
| `push sr` | `sr: SReg` | emits `Bytes<...>` | Pushes `fs` or `gs`, the only segment registers 64-bit mode can push. |

### `pop`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pop rd` | `rd: Reg` | emits `Bytes<...>` | Pops 64 bits off the stack into `rd`. |
| `pop rd` | `rd: Reg16` | emits `Bytes<...>` | Pops 16 bits into `rd`. |
| `pop dst` | `dst: MemOperand` | emits `Bytes<...>` | Pops 64 bits into the memory at `dst`. |
| `pop dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Pops a `size`-wide value into the memory at `dst`: `qword` or `word`. |
| `pop sr` | `sr: SReg` | emits `Bytes<...>` | Pops `fs` or `gs`, the only segment registers 64-bit mode can pop. |

### `syscall`

Calls the operating system. On Linux, `rax` holds the call number and
`rdi`, `rsi`, `rdx`, ... its arguments.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `syscall` |  | emits `Bytes<2>` |  |

### `assert_cl`

Fails to compile unless `count` is `cl`, the only register a shift can
take its count from.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `assert_cl count` | `count: Reg8` |  |  |

### `assert_size`

Fails to compile unless `size` is `bits` bits.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `assert_size size, bits` | `size: OperandSize`, `bits: int` |  |  |

### `adc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd + rs + CF`: add with carry. |
| `adc rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd + imm + CF`: add with carry; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `adc rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd + [src] + CF`: add with carry. |
| `adc dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] + rs + CF`: add with carry. |
| `adc dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + imm + CF`: add with carry, `size` wide. |
| `adc _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `adc [dst], imm` needs a size, like `adc qword [rbx], 1`. |

### `sbb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd - rs - CF`: subtract with borrow. |
| `sbb rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd - imm - CF`: subtract with borrow; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. |
| `sbb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd - [src] - CF`: subtract with borrow. |
| `sbb dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] - rs - CF`: subtract with borrow. |
| `sbb dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - imm - CF`: subtract with borrow, `size` wide. |
| `sbb _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `sbb [dst], imm` needs a size, like `sbb qword [rbx], 1`. |

### `xchg`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xchg rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Swaps `rd` and `rs`. |
| `xchg rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Swaps `rd` and the value at `src`, atomically. |
| `xchg dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Swaps the value at `dst` and `rs`, atomically. |

### `xadd`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xadd rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Exchanges and adds: `rs` gets `rd`, and `rd` gets their sum. |
| `xadd dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Exchanges and adds: `rs` gets `[dst]`, and `[dst]` gets their sum. |

### `cmpxchg`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpxchg rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Compares the accumulator (`al`, `ax`, `eax` or `rax`) with `rd`: if equal, `rd = rs`, else the accumulator gets `rd`. |
| `cmpxchg dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Compares the accumulator with `[dst]`: if equal, `[dst] = rs`, else the accumulator gets `[dst]`. |

### `cmpxchg8b`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpxchg8b dst` | `dst: MemOperand` | emits `Bytes<...>` | Compares `edx:eax` with the 64 bits at `dst`: if equal, stores `ecx:ebx` there, else loads them into `edx:eax`. |
| `cmpxchg8b dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `cmpxchg8b`; `size` must be `qword`. |

### `cmpxchg16b`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpxchg16b dst` | `dst: MemOperand` | emits `Bytes<...>` | Compares `rdx:rax` with the 128 bits at `dst`: if equal, stores `rcx:rbx` there, else loads them into `rdx:rax`. |
| `cmpxchg16b dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `cmpxchg16b`; `size` must be `oword`. |

### `not`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `not rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = ~rd`. |
| `not dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = ~[dst]`, `size` wide. |

### `neg`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `neg rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = -rd`. |
| `neg dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = -[dst]`, `size` wide. |

### `mul`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mul rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Unsigned multiply: `rdx:rax = rax * rd` (or the smaller registers for a smaller operand). |
| `mul dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Unsigned multiply: `rdx:rax = rax * [dst]` (or the smaller registers for a smaller operand), `size` wide. |

### `imul`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `imul rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Signed multiply: `rdx:rax = rax * rd` (or the smaller registers for a smaller operand). |
| `imul dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Signed multiply: `rdx:rax = rax * [dst]` (or the smaller registers for a smaller operand), `size` wide. |
| `imul rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd * rs`, signed, keeping the low half. |
| `imul rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd * [src]`, signed, keeping the low half. |
| `imul rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | `rd = rs * imm`, signed, keeping the low half. |
| `imul rd, src, imm` | `<R>`, `rd: R`, `src: MemOperand`, `imm: int` | emits `Bytes<...>` | `rd = [src] * imm`, signed, keeping the low half. |

### `div`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `div rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Unsigned divide: `rax = rdx:rax / rd`, `rdx` = the remainder (or the smaller registers for a smaller operand). |
| `div dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Unsigned divide: `rax = rdx:rax / [dst]`, `rdx` = the remainder (or the smaller registers for a smaller operand), `size` wide. |

### `idiv`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `idiv rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Signed divide: `rax = rdx:rax / rd`, `rdx` = the remainder (or the smaller registers for a smaller operand). |
| `idiv dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Signed divide: `rax = rdx:rax / [dst]`, `rdx` = the remainder (or the smaller registers for a smaller operand), `size` wide. |

### `inc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `inc rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = rd + 1`, leaving CF alone. |
| `inc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + 1`, leaving CF alone, `size` wide. |

### `dec`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dec rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = rd - 1`, leaving CF alone. |
| `dec dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - 1`, leaving CF alone, `size` wide. |

### `rol`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rol rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` left by `imm`. |
| `rol rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` left by `cl`. |
| `rol dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` left by `imm`, `size` wide. |
| `rol dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` left by `cl`, `size` wide. |

### `ror`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ror rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` right by `imm`. |
| `ror rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` right by `cl`. |
| `ror dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` right by `imm`, `size` wide. |
| `ror dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` right by `cl`, `size` wide. |

### `rcl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcl rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` and CF left by `imm`. |
| `rcl rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` and CF left by `cl`. |
| `rcl dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF left by `imm`, `size` wide. |
| `rcl dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF left by `cl`, `size` wide. |

### `rcr`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` and CF right by `imm`. |
| `rcr rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` and CF right by `cl`. |
| `rcr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF right by `imm`, `size` wide. |
| `rcr dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF right by `cl`, `size` wide. |

### `shld`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shld rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `rd` left by `imm`, filling from the top of `rs`. |
| `shld rd, rs, count` | `<R>`, `rd: R`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `rd` left by `cl`, filling from the top of `rs`. |
| `shld dst, rs, imm` | `<R>`, `dst: MemOperand`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `[dst]` left by `imm`, filling from the top of `rs`. |
| `shld dst, rs, count` | `<R>`, `dst: MemOperand`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `[dst]` left by `cl`, filling from the top of `rs`. |

### `shrd`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shrd rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `rd` right by `imm`, filling from the bottom of `rs`. |
| `shrd rd, rs, count` | `<R>`, `rd: R`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `rd` right by `cl`, filling from the bottom of `rs`. |
| `shrd dst, rs, imm` | `<R>`, `dst: MemOperand`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `[dst]` right by `imm`, filling from the bottom of `rs`. |
| `shrd dst, rs, count` | `<R>`, `dst: MemOperand`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `[dst]` right by `cl`, filling from the bottom of `rs`. |

### `bt`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bt rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF. |
| `bt rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF. |
| `bt dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF. The bit string can extend past `dst`. |
| `bt dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, `size` wide. |
| `bt _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `bt [dst], imm` needs a size, like `bt qword [rbx], 1`. |

### `bts`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bts rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF, then sets it. |
| `bts rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF, then sets it. |
| `bts dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF, then sets it. The bit string can extend past `dst`. |
| `bts dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, then sets it, `size` wide. |
| `bts _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `bts [dst], imm` needs a size, like `bts qword [rbx], 1`. |

### `btr`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `btr rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF, then clears it. |
| `btr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF, then clears it. |
| `btr dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF, then clears it. The bit string can extend past `dst`. |
| `btr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, then clears it, `size` wide. |
| `btr _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `btr [dst], imm` needs a size, like `btr qword [rbx], 1`. |

### `btc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `btc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF, then flips it. |
| `btc rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF, then flips it. |
| `btc dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF, then flips it. The bit string can extend past `dst`. |
| `btc dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, then flips it, `size` wide. |
| `btc _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `btc [dst], imm` needs a size, like `btc qword [rbx], 1`. |

### `bsf`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bsf rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets `rd` to the index of the lowest set bit of `rs`; ZF = 1 if `rs` is 0. |
| `bsf rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets `rd` to the index of the lowest set bit of `[src]`; ZF = 1 if it's 0. |

### `bsr`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bsr rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets `rd` to the index of the highest set bit of `rs`; ZF = 1 if `rs` is 0. |
| `bsr rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets `rd` to the index of the highest set bit of `[src]`; ZF = 1 if it's 0. |

### `bswap`

Reverses the byte order of the 32- or 64-bit `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bswap rd` | `<R>`, `rd: R` | emits `Bytes<...>` |  |

### `movzx`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movzx rd, rs` | `<D, S>`, `rd: D`, `rs: S` | emits `Bytes<...>` | `rd = rs`, zero-extended. |
| `movzx rd, src, size` | `<D>`, `rd: D`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads the `size`-wide value at `src` into `rd`, zero-extended. |

### `movsx`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsx rd, rs` | `<D, S>`, `rd: D`, `rs: S` | emits `Bytes<...>` | `rd = rs`, sign-extended. |
| `movsx rd, src, size` | `<D>`, `rd: D`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads the `size`-wide value at `src` into `rd`, sign-extended. |

### `movsxd`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsxd rd, rs` | `<D>`, `rd: D`, `rs: Reg32` | emits `Bytes<...>` | `rd = rs`, a 32-bit register sign-extended into the 64-bit `rd`. |
| `movsxd rd, src` | `<D>`, `rd: D`, `src: MemOperand` | emits `Bytes<...>` | Loads the 32 bits at `src` into the 64-bit `rd`, sign-extended. |
| `movsxd rd, src, size` | `<D>`, `rd: D`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `movsxd`; `size` must be `dword`. |

### `cmovo`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovo rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` on signed overflow (OF = 1). |
| `cmovo rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` on signed overflow (OF = 1). |

### `cmovno`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovno rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` without signed overflow (OF = 0). |
| `cmovno rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` without signed overflow (OF = 0). |

### `cmovb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if below, unsigned (CF = 1). |
| `cmovb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if below, unsigned (CF = 1). |

### `cmovae`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovae rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if above or equal, unsigned (CF = 0). |
| `cmovae rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if above or equal, unsigned (CF = 0). |

### `cmove`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmove rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if equal (ZF = 1). |
| `cmove rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if equal (ZF = 1). |

### `cmovne`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovne rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if not equal (ZF = 0). |
| `cmovne rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if not equal (ZF = 0). |

### `cmovbe`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovbe rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if below or equal, unsigned. |
| `cmovbe rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if below or equal, unsigned. |

### `cmova`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmova rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if above, unsigned. |
| `cmova rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if above, unsigned. |

### `cmovs`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovs rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the result was negative (SF = 1). |
| `cmovs rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the result was negative (SF = 1). |

### `cmovns`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovns rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the result wasn't negative (SF = 0). |
| `cmovns rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the result wasn't negative (SF = 0). |

### `cmovp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovp rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the parity flag is set (PF = 1). |
| `cmovp rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the parity flag is set (PF = 1). |

### `cmovnp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnp rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the parity flag is clear (PF = 0). |
| `cmovnp rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the parity flag is clear (PF = 0). |

### `cmovl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovl rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if less, signed. |
| `cmovl rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if less, signed. |

### `cmovge`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovge rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if greater or equal, signed. |
| `cmovge rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if greater or equal, signed. |

### `cmovle`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovle rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if less or equal, signed. |
| `cmovle rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if less or equal, signed. |

### `cmovg`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovg rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if greater, signed. |
| `cmovg rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if greater, signed. |

### `cmovz`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovz rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmove`, under another name. |
| `cmovz rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmove`, under another name. |

### `cmovnz`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnz rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovne`, under another name. |
| `cmovnz rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovne`, under another name. |

### `cmovc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovb`, under another name. |
| `cmovc rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovb`, under another name. |

### `cmovnae`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnae rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovb`, under another name. |
| `cmovnae rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovb`, under another name. |

### `cmovnc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovae`, under another name. |
| `cmovnc rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovae`, under another name. |

### `cmovnb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovae`, under another name. |
| `cmovnb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovae`, under another name. |

### `cmovnbe`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnbe rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmova`, under another name. |
| `cmovnbe rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmova`, under another name. |

### `cmovna`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovna rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovbe`, under another name. |
| `cmovna rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovbe`, under another name. |

### `cmovnge`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnge rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovl`, under another name. |
| `cmovnge rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovl`, under another name. |

### `cmovnl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnl rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovge`, under another name. |
| `cmovnl rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovge`, under another name. |

### `cmovng`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovng rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovle`, under another name. |
| `cmovng rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovle`, under another name. |

### `cmovnle`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovnle rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovg`, under another name. |
| `cmovnle rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovg`, under another name. |

### `cmovpe`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovpe rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovp`, under another name. |
| `cmovpe rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovp`, under another name. |

### `cmovpo`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmovpo rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovnp`, under another name. |
| `cmovpo rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovnp`, under another name. |

### `seto`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `seto rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 on signed overflow (OF = 1), else 0. |
| `seto dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 on signed overflow (OF = 1), else 0. |
| `seto dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `seto dst`; `size` must be `byte`. |

### `setno`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setno rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 without signed overflow (OF = 0), else 0. |
| `setno dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 without signed overflow (OF = 0), else 0. |
| `setno dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setno dst`; `size` must be `byte`. |

### `setb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setb rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if below, unsigned (CF = 1), else 0. |
| `setb dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if below, unsigned (CF = 1), else 0. |
| `setb dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setb dst`; `size` must be `byte`. |

### `setae`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setae rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if above or equal, unsigned (CF = 0), else 0. |
| `setae dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if above or equal, unsigned (CF = 0), else 0. |
| `setae dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setae dst`; `size` must be `byte`. |

### `sete`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sete rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if equal (ZF = 1), else 0. |
| `sete dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if equal (ZF = 1), else 0. |
| `sete dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `sete dst`; `size` must be `byte`. |

### `setne`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setne rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if not equal (ZF = 0), else 0. |
| `setne dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if not equal (ZF = 0), else 0. |
| `setne dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setne dst`; `size` must be `byte`. |

### `setbe`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setbe rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if below or equal, unsigned, else 0. |
| `setbe dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if below or equal, unsigned, else 0. |
| `setbe dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setbe dst`; `size` must be `byte`. |

### `seta`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `seta rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if above, unsigned, else 0. |
| `seta dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if above, unsigned, else 0. |
| `seta dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `seta dst`; `size` must be `byte`. |

### `sets`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sets rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the result was negative (SF = 1), else 0. |
| `sets dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the result was negative (SF = 1), else 0. |
| `sets dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `sets dst`; `size` must be `byte`. |

### `setns`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setns rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the result wasn't negative (SF = 0), else 0. |
| `setns dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the result wasn't negative (SF = 0), else 0. |
| `setns dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setns dst`; `size` must be `byte`. |

### `setp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setp rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the parity flag is set (PF = 1), else 0. |
| `setp dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the parity flag is set (PF = 1), else 0. |
| `setp dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setp dst`; `size` must be `byte`. |

### `setnp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnp rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the parity flag is clear (PF = 0), else 0. |
| `setnp dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the parity flag is clear (PF = 0), else 0. |
| `setnp dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnp dst`; `size` must be `byte`. |

### `setl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setl rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if less, signed, else 0. |
| `setl dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if less, signed, else 0. |
| `setl dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setl dst`; `size` must be `byte`. |

### `setge`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setge rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if greater or equal, signed, else 0. |
| `setge dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if greater or equal, signed, else 0. |
| `setge dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setge dst`; `size` must be `byte`. |

### `setle`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setle rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if less or equal, signed, else 0. |
| `setle dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if less or equal, signed, else 0. |
| `setle dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setle dst`; `size` must be `byte`. |

### `setg`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setg rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if greater, signed, else 0. |
| `setg dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if greater, signed, else 0. |
| `setg dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setg dst`; `size` must be `byte`. |

### `setz`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setz rd` | `rd: Reg8` | emits `Bytes<...>` | `sete`, under another name. |
| `setz dst` | `dst: MemOperand` | emits `Bytes<...>` | `sete`, under another name. |
| `setz dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setz dst`; `size` must be `byte`. |

### `setnz`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnz rd` | `rd: Reg8` | emits `Bytes<...>` | `setne`, under another name. |
| `setnz dst` | `dst: MemOperand` | emits `Bytes<...>` | `setne`, under another name. |
| `setnz dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnz dst`; `size` must be `byte`. |

### `setc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setc rd` | `rd: Reg8` | emits `Bytes<...>` | `setb`, under another name. |
| `setc dst` | `dst: MemOperand` | emits `Bytes<...>` | `setb`, under another name. |
| `setc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setc dst`; `size` must be `byte`. |

### `setnae`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnae rd` | `rd: Reg8` | emits `Bytes<...>` | `setb`, under another name. |
| `setnae dst` | `dst: MemOperand` | emits `Bytes<...>` | `setb`, under another name. |
| `setnae dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnae dst`; `size` must be `byte`. |

### `setnc`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnc rd` | `rd: Reg8` | emits `Bytes<...>` | `setae`, under another name. |
| `setnc dst` | `dst: MemOperand` | emits `Bytes<...>` | `setae`, under another name. |
| `setnc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnc dst`; `size` must be `byte`. |

### `setnb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnb rd` | `rd: Reg8` | emits `Bytes<...>` | `setae`, under another name. |
| `setnb dst` | `dst: MemOperand` | emits `Bytes<...>` | `setae`, under another name. |
| `setnb dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnb dst`; `size` must be `byte`. |

### `setnbe`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnbe rd` | `rd: Reg8` | emits `Bytes<...>` | `seta`, under another name. |
| `setnbe dst` | `dst: MemOperand` | emits `Bytes<...>` | `seta`, under another name. |
| `setnbe dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnbe dst`; `size` must be `byte`. |

### `setna`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setna rd` | `rd: Reg8` | emits `Bytes<...>` | `setbe`, under another name. |
| `setna dst` | `dst: MemOperand` | emits `Bytes<...>` | `setbe`, under another name. |
| `setna dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setna dst`; `size` must be `byte`. |

### `setnge`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnge rd` | `rd: Reg8` | emits `Bytes<...>` | `setl`, under another name. |
| `setnge dst` | `dst: MemOperand` | emits `Bytes<...>` | `setl`, under another name. |
| `setnge dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnge dst`; `size` must be `byte`. |

### `setnl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnl rd` | `rd: Reg8` | emits `Bytes<...>` | `setge`, under another name. |
| `setnl dst` | `dst: MemOperand` | emits `Bytes<...>` | `setge`, under another name. |
| `setnl dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnl dst`; `size` must be `byte`. |

### `setng`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setng rd` | `rd: Reg8` | emits `Bytes<...>` | `setle`, under another name. |
| `setng dst` | `dst: MemOperand` | emits `Bytes<...>` | `setle`, under another name. |
| `setng dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setng dst`; `size` must be `byte`. |

### `setnle`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setnle rd` | `rd: Reg8` | emits `Bytes<...>` | `setg`, under another name. |
| `setnle dst` | `dst: MemOperand` | emits `Bytes<...>` | `setg`, under another name. |
| `setnle dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnle dst`; `size` must be `byte`. |

### `setpe`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setpe rd` | `rd: Reg8` | emits `Bytes<...>` | `setp`, under another name. |
| `setpe dst` | `dst: MemOperand` | emits `Bytes<...>` | `setp`, under another name. |
| `setpe dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setpe dst`; `size` must be `byte`. |

### `setpo`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `setpo rd` | `rd: Reg8` | emits `Bytes<...>` | `setnp`, under another name. |
| `setpo dst` | `dst: MemOperand` | emits `Bytes<...>` | `setnp`, under another name. |
| `setpo dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setpo dst`; `size` must be `byte`. |

### `jmp_far`

Jumps to the far pointer at `src`: a 64-bit offset, then a 16-bit
segment selector.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jmp_far src` | `src: MemOperand` | emits `Bytes<...>` |  |

### `call_far`

Calls the far pointer at `src`: a 64-bit offset, then a 16-bit segment
selector.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `call_far src` | `src: MemOperand` | emits `Bytes<...>` |  |

### `retf`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `retf` |  | emits `Bytes<...>` | Returns from a far call. |
| `retf n` | `n: int` | emits `Bytes<...>` | Returns from a far call, then pops `n` more bytes. |

### `enter`

Makes a stack frame of `size` bytes, nested `level` deep.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `enter size, level` | `size: int`, `level: int` | emits `Bytes<...>` |  |

### `leave`

Undoes `enter`: `mov rsp, rbp`, then `pop rbp`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `leave` |  | emits `Bytes<...>` |  |

### `interrupt`

Calls interrupt handler `n`. The dialects spell it `int n`; `int` itself
names the integer type.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `interrupt n` | `n: int` | emits `Bytes<...>` |  |

### `loop`

Decrements `rcx`, then jumps to the label `target` unless it's 0. `target`
must be within 127 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `loop target` | `target: int` | emits `Rel8Instr` |  |

### `loope`

Decrements `rcx`, then jumps to the label `target` if it isn't 0 and ZF = 1.
`target` must be within 127 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `loope target` | `target: int` | emits `Rel8Instr` |  |

### `loopz`

`loope`, under another name. `target` must be within 127 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `loopz target` | `target: int` | emits `Rel8Instr` |  |

### `loopne`

Decrements `rcx`, then jumps to the label `target` if it isn't 0 and ZF = 0.
`target` must be within 127 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `loopne target` | `target: int` | emits `Rel8Instr` |  |

### `loopnz`

`loopne`, under another name. `target` must be within 127 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `loopnz target` | `target: int` | emits `Rel8Instr` |  |

### `jrcxz`

Jumps to the label `target` if `rcx` is 0. `target` must be within 127 bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jrcxz target` | `target: int` | emits `Rel8Instr` |  |

### `jecxz`

Jumps to the label `target` if `ecx` is 0. `target` must be within 127
bytes.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jecxz target` | `target: int` | emits `Rel8Instr2` |  |

### `nop`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `nop` |  | emits `Bytes<...>` | Does nothing. |
| `nop rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Does nothing, in a longer encoding that names `rd`: for padding. |
| `nop dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Does nothing, in a longer encoding that names `dst`: for padding. |

### `ud0`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ud0 rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Raises an invalid-opcode exception. |
| `ud0 rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Raises an invalid-opcode exception. |

### `ud1`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ud1 rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Raises an invalid-opcode exception. |
| `ud1 rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Raises an invalid-opcode exception. |

### `out`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `out port, rs` | `<R>`, `port: Reg16`, `rs: R` | emits `Bytes<...>` | Writes `rs` (`al`, `ax` or `eax`) to the I/O port in `port`, which must be `dx`. |
| `out port, rs` | `<R>`, `port: int`, `rs: R` | emits `Bytes<...>` | Writes `rs` (`al`, `ax` or `eax`) to I/O port `port`, 0 to 255. |

### `lfs`

Loads the far pointer at `src` into `fs` and `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lfs rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` |  |

### `lgs`

Loads the far pointer at `src` into `gs` and `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lgs rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` |  |

### `lss`

Loads the far pointer at `src` into `ss` and `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lss rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` |  |

### `clc`

Clears the carry flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clc` |  | emits `Bytes<...>` |  |

### `stc`

Sets the carry flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `stc` |  | emits `Bytes<...>` |  |

### `cmc`

Flips the carry flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmc` |  | emits `Bytes<...>` |  |

### `cld`

Clears the direction flag, so string instructions count up.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cld` |  | emits `Bytes<...>` |  |

### `std`

Sets the direction flag, so string instructions count down.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `std` |  | emits `Bytes<...>` |  |

### `cli`

Clears the interrupt flag. Needs I/O privilege.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cli` |  | emits `Bytes<...>` |  |

### `sti`

Sets the interrupt flag. Needs I/O privilege.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sti` |  | emits `Bytes<...>` |  |

### `cbw`

Sign-extends `al` into `ax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cbw` |  | emits `Bytes<...>` |  |

### `cwde`

Sign-extends `ax` into `eax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cwde` |  | emits `Bytes<...>` |  |

### `cdqe`

Sign-extends `eax` into `rax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cdqe` |  | emits `Bytes<...>` |  |

### `cwd`

Sign-extends `ax` into `dx:ax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cwd` |  | emits `Bytes<...>` |  |

### `cdq`

Sign-extends `eax` into `edx:eax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cdq` |  | emits `Bytes<...>` |  |

### `cqo`

Sign-extends `rax` into `rdx:rax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cqo` |  | emits `Bytes<...>` |  |

### `lahf`

Loads the low byte of the flags into `ah`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lahf` |  | emits `Bytes<...>` |  |

### `sahf`

Stores `ah` into the low byte of the flags.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sahf` |  | emits `Bytes<...>` |  |

### `pushf`

Pushes the flags.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pushf` |  | emits `Bytes<...>` |  |

### `pushfq`

Pushes the flags.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pushfq` |  | emits `Bytes<...>` |  |

### `popf`

Pops the flags.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `popf` |  | emits `Bytes<...>` |  |

### `popfq`

Pops the flags.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `popfq` |  | emits `Bytes<...>` |  |

### `pause`

Hints that this is a spin-wait loop.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pause` |  | emits `Bytes<...>` |  |

### `ud2`

Raises an invalid-opcode exception.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ud2` |  | emits `Bytes<...>` |  |

### `cpuid`

Reads processor identification and features, picked by `eax`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cpuid` |  | emits `Bytes<...>` |  |

### `int3`

Stops in a debugger.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `int3` |  | emits `Bytes<...>` |  |

### `iret`

Returns from an interrupt.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `iret` |  | emits `Bytes<...>` |  |

### `iretd`

Returns from an interrupt.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `iretd` |  | emits `Bytes<...>` |  |

### `iretq`

Returns from an interrupt, with 64-bit stack values.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `iretq` |  | emits `Bytes<...>` |  |

### `xlatb`

`al = [rbx + al]`: looks `al` up in a table.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xlatb` |  | emits `Bytes<...>` |  |

### `rep`

Repeats the next string instruction `rcx` times.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rep` |  | emits `Bytes<...>` |  |

### `repe`

Repeats the next `cmps` or `scas` while equal, up to `rcx` times.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `repe` |  | emits `Bytes<...>` |  |

### `repz`

`repe`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `repz` |  | emits `Bytes<...>` |  |

### `repne`

Repeats the next `cmps` or `scas` while not equal, up to `rcx` times.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `repne` |  | emits `Bytes<...>` |  |

### `repnz`

`repne`, under another name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `repnz` |  | emits `Bytes<...>` |  |

### `lock`

Makes the next instruction's memory access atomic.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lock` |  | emits `Bytes<...>` |  |

### `movsb`

Copies a byte from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsb` |  | emits `Bytes<...>` |  |

### `movsw`

Copies a word from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsw` |  | emits `Bytes<...>` |  |

### `movsd`

Copies a doubleword from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsd` |  | emits `Bytes<...>` |  |

### `movsq`

Copies a quadword from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsq` |  | emits `Bytes<...>` |  |

### `cmpsb`

Compares a byte at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpsb` |  | emits `Bytes<...>` |  |

### `cmpsw`

Compares a word at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpsw` |  | emits `Bytes<...>` |  |

### `cmpsd`

Compares a doubleword at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpsd` |  | emits `Bytes<...>` |  |

### `cmpsq`

Compares a quadword at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpsq` |  | emits `Bytes<...>` |  |

### `scasb`

Compares the accumulator with a byte at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `scasb` |  | emits `Bytes<...>` |  |

### `scasw`

Compares the accumulator with a word at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `scasw` |  | emits `Bytes<...>` |  |

### `scasd`

Compares the accumulator with a doubleword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `scasd` |  | emits `Bytes<...>` |  |

### `scasq`

Compares the accumulator with a quadword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `scasq` |  | emits `Bytes<...>` |  |

### `lodsb`

Loads a byte from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lodsb` |  | emits `Bytes<...>` |  |

### `lodsw`

Loads a word from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lodsw` |  | emits `Bytes<...>` |  |

### `lodsd`

Loads a doubleword from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lodsd` |  | emits `Bytes<...>` |  |

### `lodsq`

Loads a quadword from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lodsq` |  | emits `Bytes<...>` |  |

### `stosb`

Stores the accumulator's low byte at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `stosb` |  | emits `Bytes<...>` |  |

### `stosw`

Stores the accumulator's low word at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `stosw` |  | emits `Bytes<...>` |  |

### `stosd`

Stores the accumulator's low doubleword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `stosd` |  | emits `Bytes<...>` |  |

### `stosq`

Stores the accumulator's low quadword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `stosq` |  | emits `Bytes<...>` |  |

### `insb`

Reads a byte from I/O port `dx` into `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `insb` |  | emits `Bytes<...>` |  |

### `insw`

Reads a word from I/O port `dx` into `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `insw` |  | emits `Bytes<...>` |  |

### `insd`

Reads a doubleword from I/O port `dx` into `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `insd` |  | emits `Bytes<...>` |  |

### `outsb`

Writes a byte from `[rsi]` to I/O port `dx`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `outsb` |  | emits `Bytes<...>` |  |

### `outsw`

Writes a word from `[rsi]` to I/O port `dx`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `outsw` |  | emits `Bytes<...>` |  |

### `outsd`

Writes a doubleword from `[rsi]` to I/O port `dx`, then steps it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `outsd` |  | emits `Bytes<...>` |  |

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

### `Reg16`

`struct Reg16`

The low 16 bits of a register. An instruction given one works on 16
bits, with a `0x66` prefix.

| Field | Type | Description |
|---|---|---|
| `reg` | `Reg` | The register it's the low quarter of. |

### `Reg8`

`struct Reg8`

An 8-bit register: the low byte of a register, or for `ah`, `ch`, `dh`
and `bh`, the byte above it.

| Field | Type | Description |
|---|---|---|
| `reg` | `Reg` | The register number it's encoded with: `ah` is 4, like `spl`. |
| `high` | `int` | 1 for `ah`, `ch`, `dh` and `bh`, which can't be used in an instruction with a REX prefix; 0 otherwise. |

### `SReg`

`struct SReg`

A segment register. In 64-bit mode only `fs` and `gs` still have a base
address of their own, usually for thread-local storage.

| Field | Type | Description |
|---|---|---|
| `num` | `int` | Its number: `es` 0, `cs` 1, `ss` 2, `ds` 3, `fs` 4, `gs` 5. |

### `Gpr`

`struct Gpr`

A general-purpose register of any size, with what encoding it needs.

| Field | Type | Description |
|---|---|---|
| `num` | `Reg` | The register number. |
| `size` | `int` | The operand size in bits: 8, 16, 32 or 64. |
| `rex` | `int` | 1 if it needs a REX prefix (`spl`, `bpl`, `sil`, `dil`), -1 if it can't have one (`ah`, `ch`, `dh`, `bh`), 0 otherwise. |

### `Byte`

`type Byte = bits<8>`

A byte.

### `Bytes`

`struct Bytes<const N: int>`

`N` bytes, packed by `bitter` in order: an instruction's encoding.

Some of its fields are generated by `@for` or `@if`.

### `OperandSize`

`struct OperandSize`

An operand size, for an instruction whose memory operand has no
register to give it one, as in `add qword [rbx], 1`.

| Field | Type | Description |
|---|---|---|
| `bits` | `int` | The size in bits. |

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

### `Rel8Instr`

`struct Rel8Instr`

A short jump: an opcode and an 8-bit offset `bitter` works out.

| Field | Type | Description |
|---|---|---|
| `opcode` | `Byte` | The opcode. |
| `rel8` | `Positioned<8>` | The offset from the next instruction to the target. |

### `Rel8Instr2`

`struct Rel8Instr2`

A short jump with a prefix byte: `jecxz`.

| Field | Type | Description |
|---|---|---|
| `prefix` | `Byte` | The prefix. |
| `opcode` | `Byte` | The opcode. |
| `rel8` | `Positioned<8>` | The offset from the next instruction to the target. |

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
| `ax` |  | `Reg16(reg = rax)` | The low 16 bits of `rax`. |
| `cx` |  | `Reg16(reg = rcx)` | The low 16 bits of `rcx`. |
| `dx` |  | `Reg16(reg = rdx)` | The low 16 bits of `rdx`. |
| `bx` |  | `Reg16(reg = rbx)` | The low 16 bits of `rbx`. |
| `sp` |  | `Reg16(reg = rsp)` | The low 16 bits of `rsp`. |
| `bp` |  | `Reg16(reg = rbp)` | The low 16 bits of `rbp`. |
| `si` |  | `Reg16(reg = rsi)` | The low 16 bits of `rsi`. |
| `di` |  | `Reg16(reg = rdi)` | The low 16 bits of `rdi`. |
| `r8w` |  | `` Reg16(reg = r`8`) `` | The low 16 bits of `r8` to `r15`. |
| `r9w` |  | `` Reg16(reg = r`9`) `` | The low 16 bits of `r8` to `r15`. |
| `r10w` |  | `` Reg16(reg = r`10`) `` | The low 16 bits of `r8` to `r15`. |
| `r11w` |  | `` Reg16(reg = r`11`) `` | The low 16 bits of `r8` to `r15`. |
| `r12w` |  | `` Reg16(reg = r`12`) `` | The low 16 bits of `r8` to `r15`. |
| `r13w` |  | `` Reg16(reg = r`13`) `` | The low 16 bits of `r8` to `r15`. |
| `r14w` |  | `` Reg16(reg = r`14`) `` | The low 16 bits of `r8` to `r15`. |
| `r15w` |  | `` Reg16(reg = r`15`) `` | The low 16 bits of `r8` to `r15`. |
| `al` |  | `Reg8(reg = rax, high = 0)` | The low 8 bits of `rax`. |
| `cl` |  | `Reg8(reg = rcx, high = 0)` | The low 8 bits of `rcx`. |
| `dl` |  | `Reg8(reg = rdx, high = 0)` | The low 8 bits of `rdx`. |
| `bl` |  | `Reg8(reg = rbx, high = 0)` | The low 8 bits of `rbx`. |
| `spl` |  | `Reg8(reg = rsp, high = 0)` | The low 8 bits of `rsp`. |
| `bpl` |  | `Reg8(reg = rbp, high = 0)` | The low 8 bits of `rbp`. |
| `sil` |  | `Reg8(reg = rsi, high = 0)` | The low 8 bits of `rsi`. |
| `dil` |  | `Reg8(reg = rdi, high = 0)` | The low 8 bits of `rdi`. |
| `r8b` |  | `` Reg8(reg = r`8`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r9b` |  | `` Reg8(reg = r`9`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r10b` |  | `` Reg8(reg = r`10`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r11b` |  | `` Reg8(reg = r`11`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r12b` |  | `` Reg8(reg = r`12`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r13b` |  | `` Reg8(reg = r`13`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r14b` |  | `` Reg8(reg = r`14`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `r15b` |  | `` Reg8(reg = r`15`, high = 0) `` | The low 8 bits of `r8` to `r15`. |
| `ah` |  | `Reg8(reg = rsp, high = 1)` | Bits 8 to 15 of `rax`. |
| `ch` |  | `Reg8(reg = rbp, high = 1)` | Bits 8 to 15 of `rcx`. |
| `dh` |  | `Reg8(reg = rsi, high = 1)` | Bits 8 to 15 of `rdx`. |
| `bh` |  | `Reg8(reg = rdi, high = 1)` | Bits 8 to 15 of `rbx`. |
| `es` |  | `SReg(num = 0)` | The extra segment register. |
| `cs` |  | `SReg(num = 1)` | The code segment register. |
| `ss` |  | `SReg(num = 2)` | The stack segment register. |
| `ds` |  | `SReg(num = 3)` | The data segment register. |
| `fs` |  | `SReg(num = 4)` | A segment register with a base address, used for thread-local storage. |
| `gs` |  | `SReg(num = 5)` | A segment register with a base address, used for thread-local storage. |
| `byte` |  | `OperandSize(bits = 8)` | 8 bits. |
| `word` |  | `OperandSize(bits = 16)` | 16 bits. |
| `dword` |  | `OperandSize(bits = 32)` | 32 bits. |
| `qword` |  | `OperandSize(bits = 64)` | 64 bits. |
| `oword` |  | `OperandSize(bits = 128)` | 128 bits, for `cmpxchg16b`. |
