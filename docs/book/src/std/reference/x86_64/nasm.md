# `nasm`

[`std`](../index.md) › [`x86_64`](index.md) › `nasm`

NASM-flavored x86-64 assembly: everything `std.x86_64.intel` provides,
plus the NASM spellings it doesn't have.

```basm
from std.x86_64.nasm import *

lea rsi, [rel msg]
mov eax, [rel counter]
add [rel total], rcx

msg:
    db "Hello, World!", 10
counter:
    dd 0
total:
    dq 0
```

`[rip+disp]` from `std.x86_64.intel` takes a displacement you already
know; `[rel label]` takes a label and computes the displacement once the
code is laid out.

Not covered: NASM's `$`/`$$`, `equ`, `times`, `resb`, and
`global`/`extern` (use `const`, `@for`, and `pub` labels instead).

Re-exports [`std.x86_64.intel`](intel.md).

## Macros

### `gpr`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `gpr(r)` | `r: Reg` | returns `Gpr` | `r` as a 64-bit `Gpr`. | [`std.x86_64.impl`](impl.md) |
| `gpr(r)` | `r: Reg32` | returns `Gpr` | `r` as a 32-bit `Gpr`. | [`std.x86_64.impl`](impl.md) |
| `gpr(r)` | `r: Reg16` | returns `Gpr` | `r` as a 16-bit `Gpr`. | [`std.x86_64.impl`](impl.md) |
| `gpr(r)` | `r: Reg8` | returns `Gpr` | `r` as an 8-bit `Gpr`. | [`std.x86_64.impl`](impl.md) |
| `gpr(_x)` | `_x: int` | returns `Gpr` | Fails: an `int` where an instruction expects a register. | [`std.x86_64.impl`](impl.md) |
| `gpr(_m)` | `_m: MemOperand` | returns `Gpr` | Fails: a memory operand here needs its size written out. | [`std.x86_64.impl`](impl.md) |

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
| `mov rd, src` | `rd: Reg`, `src: RipLabel` |  | Loads the value at the label `src` into `rd`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, src` | `rd: Reg32`, `src: RipLabel` |  |  | [`std.x86_64.impl`](impl.md) |
| `mov dst, rs` | `dst: RipLabel`, `rs: Reg` |  | Stores `rs` at the label `dst`. | [`std.x86_64.impl`](impl.md) |
| `mov dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `mov rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = imm`. A 64-bit `rd` takes a full 64-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, src, size` | `<R>`, `rd: R`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads the value at `src` into `rd`; `size` must match `rd`'s. | [`std.x86_64.impl`](impl.md) |
| `mov dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Stores `rs` at `dst`. | [`std.x86_64.impl`](impl.md) |
| `mov dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Stores `imm` at `dst`, `size` wide. A 64-bit store takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `mov _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `mov [dst], imm` needs a size, like `mov qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |
| `mov rd, sr` | `<R>`, `rd: R`, `sr: SReg` | emits `Bytes<...>` | `rd = sr`. | [`std.x86_64.impl`](impl.md) |
| `mov sr, rs` | `<R>`, `sr: SReg`, `rs: R` | emits `Bytes<...>` | Loads `sr` from the low 16 bits of `rs`. | [`std.x86_64.impl`](impl.md) |
| `mov dst, sr` | `dst: MemOperand`, `sr: SReg` | emits `Bytes<...>` | Stores `sr` at `dst`, 16 bits wide. | [`std.x86_64.impl`](impl.md) |
| `mov dst, sr, size` | `dst: MemOperand`, `sr: SReg`, `size: OperandSize` | emits `Bytes<...>` | Stores `sr` at `dst`; `size` must be `word`. | [`std.x86_64.impl`](impl.md) |
| `mov sr, src` | `sr: SReg`, `src: MemOperand` | emits `Bytes<...>` | Loads `sr` from the 16 bits at `src`. | [`std.x86_64.impl`](impl.md) |
| `mov sr, src, size` | `sr: SReg`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads `sr` from `src`; `size` must be `word`. | [`std.x86_64.impl`](impl.md) |

### `add`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `add rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd + rs`. | [`std.x86_64.impl`](impl.md) |
| `add rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd + imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + rs`. | [`std.x86_64.impl`](impl.md) |
| `add dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] + imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] + rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] + rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `add rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd + rs`. | [`std.x86_64.impl`](impl.md) |
| `add rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd + imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `add rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd + [src]`. | [`std.x86_64.impl`](impl.md) |
| `add dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] + rs`. | [`std.x86_64.impl`](impl.md) |
| `add dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `add _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `add [dst], imm` needs a size, like `add qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `or`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `or rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd \| rs`. | [`std.x86_64.impl`](impl.md) |
| `or rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd \| imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| rs`. | [`std.x86_64.impl`](impl.md) |
| `or dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] \| imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] \| rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] \| rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `or rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd \| rs`. | [`std.x86_64.impl`](impl.md) |
| `or rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd \| imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `or rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd \| [src]`. | [`std.x86_64.impl`](impl.md) |
| `or dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] \| rs`. | [`std.x86_64.impl`](impl.md) |
| `or dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] \| imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `or _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `or [dst], imm` needs a size, like `or qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `and`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `and rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd & rs`. | [`std.x86_64.impl`](impl.md) |
| `and rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd & imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & rs`. | [`std.x86_64.impl`](impl.md) |
| `and dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] & imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] & rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] & rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `and rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd & rs`. | [`std.x86_64.impl`](impl.md) |
| `and rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd & imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `and rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd & [src]`. | [`std.x86_64.impl`](impl.md) |
| `and dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] & rs`. | [`std.x86_64.impl`](impl.md) |
| `and dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] & imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `and _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `and [dst], imm` needs a size, like `and qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `sub`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sub rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd - rs`. | [`std.x86_64.impl`](impl.md) |
| `sub rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd - imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - rs`. | [`std.x86_64.impl`](impl.md) |
| `sub dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] - imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] - rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] - rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `sub rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd - rs`. | [`std.x86_64.impl`](impl.md) |
| `sub rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd - imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `sub rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd - [src]`. | [`std.x86_64.impl`](impl.md) |
| `sub dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] - rs`. | [`std.x86_64.impl`](impl.md) |
| `sub dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `sub _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `sub [dst], imm` needs a size, like `sub qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `xor`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `xor rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `rd = rd ^ rs`. | [`std.x86_64.impl`](impl.md) |
| `xor rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd ^ imm`, with a 32-bit `imm` sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ rs`. | [`std.x86_64.impl`](impl.md) |
| `xor dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | `[dst] = [dst] ^ imm`, with a 32-bit `imm`. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | `[dst] = [dst] ^ rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs` | `dst: RipLabel`, `rs: Reg` |  | `[dst] = [dst] ^ rs`, where `dst` is a label. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `xor rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd ^ rs`. | [`std.x86_64.impl`](impl.md) |
| `xor rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd ^ imm`; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `xor rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd ^ [src]`. | [`std.x86_64.impl`](impl.md) |
| `xor dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] ^ rs`. | [`std.x86_64.impl`](impl.md) |
| `xor dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] ^ imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `xor _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `xor [dst], imm` needs a size, like `xor qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `cmp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmp rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd - imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] - imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] - rs`, where `dst` is a label, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs` | `dst: RipLabel`, `rs: Reg` |  | Sets the flags from `[dst] - rs`, where `dst` is a label, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `cmp rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets the flags from `rd - rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Sets the flags from `rd - imm`, without storing it; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `cmp rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets the flags from `rd - [src]`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Sets the flags from `[dst] - rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `cmp dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Sets the flags from `[dst] - imm`, without storing it, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `cmp _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `cmp [dst], imm` needs a size, like `cmp qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `test`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `test rd, rs, w` | `rd: Reg`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `rd & imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs, w` | `dst: MemOperand`, `rs: Reg`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, imm, w` | `dst: MemOperand`, `imm: int`, `w: int` | emits `Bytes<...>` | Sets the flags from `[dst] & imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs, w` | `dst: RipLabel`, `rs: Reg`, `w: int` |  | Sets the flags from `[dst] & rs`, where `dst` is a label, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs` | `dst: RipLabel`, `rs: Reg` |  | Sets the flags from `[dst] & rs`, where `dst` is a label, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs` | `dst: RipLabel`, `rs: Reg32` |  |  | [`std.x86_64.impl`](impl.md) |
| `test rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets the flags from `rd & rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Sets the flags from `rd & imm`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets the flags from `rd & [src]`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Sets the flags from `[dst] & rs`, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Sets the flags from `[dst] & imm`, `size` wide, without storing it. | [`std.x86_64.impl`](impl.md) |
| `test _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `test [dst], imm` needs a size, like `test qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

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

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jmp target` | `target: label` | emits `Rel32Instr` | Jumps to the label `target`. | [`std.x86_64.impl`](impl.md) |
| `jmp r` | `r: Reg` | emits `Bytes<...>` | Jumps to the address in `r`. | [`std.x86_64.impl`](impl.md) |
| `jmp src` | `src: MemOperand` | emits `Bytes<...>` | Jumps to the address stored at `src`. | [`std.x86_64.impl`](impl.md) |
| `jmp src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `jmp src`; `size` must be `qword`. | [`std.x86_64.impl`](impl.md) |

### `call`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call target` | `target: label` | emits `Rel32Instr` | Pushes the address of the next instruction and jumps to the label `target`. | [`std.x86_64.impl`](impl.md) |
| `call r` | `r: Reg` | emits `Bytes<...>` | Calls the address in `r`. | [`std.x86_64.impl`](impl.md) |
| `call src` | `src: MemOperand` | emits `Bytes<...>` | Calls the address stored at `src`. | [`std.x86_64.impl`](impl.md) |
| `call src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `call src`; `size` must be `qword`. | [`std.x86_64.impl`](impl.md) |

### `je`

Jumps to the label `target` if equal (ZF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `je target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jne`

Jumps to the label `target` if not equal (ZF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jne target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jb`

Jumps to the label `target` if below, unsigned (CF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jb target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jae`

Jumps to the label `target` if above or equal, unsigned (CF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jae target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `ja`

Jumps to the label `target` if above, unsigned.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ja target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jbe`

Jumps to the label `target` if below or equal, unsigned.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jbe target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jl`

Jumps to the label `target` if less, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jl target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jge`

Jumps to the label `target` if greater or equal, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jge target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jle`

Jumps to the label `target` if less or equal, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jle target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jg`

Jumps to the label `target` if greater, signed.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jg target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `js`

Jumps to the label `target` if the result was negative (SF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `js target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jns`

Jumps to the label `target` if the result wasn't negative (SF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jns target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jo`

Jumps to the label `target` on signed overflow (OF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jo target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jno`

Jumps to the label `target` without signed overflow (OF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jno target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jp`

Jumps to the label `target` if the parity flag is set (PF = 1).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jp target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnp`

Jumps to the label `target` if the parity flag is clear (PF = 0).

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnp target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jz`

`je`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jz target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnz`

`jne`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnz target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jc`

`jb`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jc target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnae`

`jb`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnae target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnc`

`jae`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnc target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnb`

`jae`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnb target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnbe`

`ja`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnbe target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jna`

`jbe`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jna target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnge`

`jl`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnge target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnl`

`jge`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnl target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jng`

`jle`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jng target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jnle`

`jg`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jnle target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jpe`

`jp`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jpe target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `jpo`

`jnp`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jpo target` | `target: label` | emits `Rel32Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `ret`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ret` |  | emits `Byte` | Returns: pops an address and jumps to it. | [`std.x86_64.impl`](impl.md) |
| `ret n` | `n: int` | emits `Bytes<...>` | Returns, then pops `n` more bytes. | [`std.x86_64.impl`](impl.md) |

### `shl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shl rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd << imm`. | [`std.x86_64.impl`](impl.md) |
| `shl rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd << imm`. | [`std.x86_64.impl`](impl.md) |
| `shl rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | `rd = rd << cl`. | [`std.x86_64.impl`](impl.md) |
| `shl dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] << imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `shl dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] << cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `shl_cl`

`rd = rd << cl`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shl_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `shr`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shr rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in zeros. | [`std.x86_64.impl`](impl.md) |
| `shr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in zeros. | [`std.x86_64.impl`](impl.md) |
| `shr rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | `rd = rd >> cl`, shifting in zeros. | [`std.x86_64.impl`](impl.md) |
| `shr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> imm`, shifting in zeros, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `shr dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> cl`, shifting in zeros, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `shr_cl`

`rd = rd >> cl`, shifting in zeros.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shr_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `sar`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sar rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in copies of the sign bit. | [`std.x86_64.impl`](impl.md) |
| `sar rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd >> imm`, shifting in copies of the sign bit. | [`std.x86_64.impl`](impl.md) |
| `sar rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | `rd = rd >> cl`, shifting in copies of the sign bit. | [`std.x86_64.impl`](impl.md) |
| `sar dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> imm`, shifting in copies of the sign bit, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `sar dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] >> cl`, shifting in copies of the sign bit, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `sar_cl`

`rd = rd >> cl`, shifting in copies of the sign bit.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sar_cl rd, w` | `rd: Reg`, `w: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lea`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lea rd, src, w` | `rd: Reg`, `src: MemOperand`, `w: int` | emits `Bytes<...>` | Loads the address `src` stands for into `rd`, without reading memory. | [`std.x86_64.impl`](impl.md) |
| `lea rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the address `src` stands for into `rd`, without reading memory. | [`std.x86_64.impl`](impl.md) |
| `lea rd, src, w` | `rd: Reg`, `src: RipLabel`, `w: int` |  | Loads the address of the label `src` into `rd`. | [`std.x86_64.impl`](impl.md) |
| `lea rd, src` | `rd: Reg`, `src: RipLabel` |  | Loads the address of the label `src` into `rd`. | [`std.x86_64.impl`](impl.md) |
| `lea rd, src` | `rd: Reg32`, `src: RipLabel` |  |  | [`std.x86_64.impl`](impl.md) |

### `rip_label_instr`

An instruction with opcode `opcode` whose memory operand is the label
`addr`, and whose other operand is `r`. The dialects' `[rel label]`
forms are built on it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rip_label_instr opcode, addr, r, w` | `opcode: int`, `addr: RipLabel`, `r: Reg`, `w: int` |  |  | [`std.x86_64.impl`](impl.md) |

### `push`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `push rd` | `rd: Reg` | emits `Bytes<...>` | Pushes the 64-bit `rd` onto the stack. | [`std.x86_64.impl`](impl.md) |
| `push rd` | `rd: Reg16` | emits `Bytes<...>` | Pushes the 16-bit `rd`. | [`std.x86_64.impl`](impl.md) |
| `push imm` | `imm: int` | emits `Bytes<...>` | Pushes `imm`, a 32-bit immediate sign-extended to 64 bits. | [`std.x86_64.impl`](impl.md) |
| `push src` | `src: MemOperand` | emits `Bytes<...>` | Pushes the 64 bits at `src`. | [`std.x86_64.impl`](impl.md) |
| `push src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Pushes the `size`-wide value at `src`: `qword` or `word`. | [`std.x86_64.impl`](impl.md) |
| `push sr` | `sr: SReg` | emits `Bytes<...>` | Pushes `fs` or `gs`, the only segment registers 64-bit mode can push. | [`std.x86_64.impl`](impl.md) |

### `pop`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pop rd` | `rd: Reg` | emits `Bytes<...>` | Pops 64 bits off the stack into `rd`. | [`std.x86_64.impl`](impl.md) |
| `pop rd` | `rd: Reg16` | emits `Bytes<...>` | Pops 16 bits into `rd`. | [`std.x86_64.impl`](impl.md) |
| `pop dst` | `dst: MemOperand` | emits `Bytes<...>` | Pops 64 bits into the memory at `dst`. | [`std.x86_64.impl`](impl.md) |
| `pop dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Pops a `size`-wide value into the memory at `dst`: `qword` or `word`. | [`std.x86_64.impl`](impl.md) |
| `pop sr` | `sr: SReg` | emits `Bytes<...>` | Pops `fs` or `gs`, the only segment registers 64-bit mode can pop. | [`std.x86_64.impl`](impl.md) |

### `syscall`

Calls the operating system. On Linux, `rax` holds the call number and
`rdi`, `rsi`, `rdx`, ... its arguments.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `syscall` |  | emits `Bytes<2>` |  | [`std.x86_64.impl`](impl.md) |

### `assert_cl`

Fails to compile unless `count` is `cl`, the only register a shift can
take its count from.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `assert_cl count` | `count: Reg8` |  |  | [`std.x86_64.impl`](impl.md) |

### `assert_size`

Fails to compile unless `size` is `bits` bits.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `assert_size size, bits` | `size: OperandSize`, `bits: int` |  |  | [`std.x86_64.impl`](impl.md) |

### `adc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `adc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd + rs + CF`: add with carry. | [`std.x86_64.impl`](impl.md) |
| `adc rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd + imm + CF`: add with carry; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `adc rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd + [src] + CF`: add with carry. | [`std.x86_64.impl`](impl.md) |
| `adc dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] + rs + CF`: add with carry. | [`std.x86_64.impl`](impl.md) |
| `adc dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + imm + CF`: add with carry, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `adc _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `adc [dst], imm` needs a size, like `adc qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `sbb`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd - rs - CF`: subtract with borrow. | [`std.x86_64.impl`](impl.md) |
| `sbb rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd - imm - CF`: subtract with borrow; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `sbb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd - [src] - CF`: subtract with borrow. | [`std.x86_64.impl`](impl.md) |
| `sbb dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] - rs - CF`: subtract with borrow. | [`std.x86_64.impl`](impl.md) |
| `sbb dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - imm - CF`: subtract with borrow, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `sbb _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `sbb [dst], imm` needs a size, like `sbb qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `xchg`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `xchg rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Swaps `rd` and `rs`. | [`std.x86_64.impl`](impl.md) |
| `xchg rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Swaps `rd` and the value at `src`, atomically. | [`std.x86_64.impl`](impl.md) |
| `xchg dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Swaps the value at `dst` and `rs`, atomically. | [`std.x86_64.impl`](impl.md) |

### `xadd`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `xadd rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Exchanges and adds: `rs` gets `rd`, and `rd` gets their sum. | [`std.x86_64.impl`](impl.md) |
| `xadd dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Exchanges and adds: `rs` gets `[dst]`, and `[dst]` gets their sum. | [`std.x86_64.impl`](impl.md) |

### `cmpxchg`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpxchg rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Compares the accumulator (`al`, `ax`, `eax` or `rax`) with `rd`: if equal, `rd = rs`, else the accumulator gets `rd`. | [`std.x86_64.impl`](impl.md) |
| `cmpxchg dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Compares the accumulator with `[dst]`: if equal, `[dst] = rs`, else the accumulator gets `[dst]`. | [`std.x86_64.impl`](impl.md) |

### `cmpxchg8b`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpxchg8b dst` | `dst: MemOperand` | emits `Bytes<...>` | Compares `edx:eax` with the 64 bits at `dst`: if equal, stores `ecx:ebx` there, else loads them into `edx:eax`. | [`std.x86_64.impl`](impl.md) |
| `cmpxchg8b dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `cmpxchg8b`; `size` must be `qword`. | [`std.x86_64.impl`](impl.md) |

### `cmpxchg16b`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpxchg16b dst` | `dst: MemOperand` | emits `Bytes<...>` | Compares `rdx:rax` with the 128 bits at `dst`: if equal, stores `rcx:rbx` there, else loads them into `rdx:rax`. | [`std.x86_64.impl`](impl.md) |
| `cmpxchg16b dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `cmpxchg16b`; `size` must be `oword`. | [`std.x86_64.impl`](impl.md) |

### `not`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `not rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = ~rd`. | [`std.x86_64.impl`](impl.md) |
| `not dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = ~[dst]`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `neg`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `neg rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = -rd`. | [`std.x86_64.impl`](impl.md) |
| `neg dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = -[dst]`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `mul`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mul rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Unsigned multiply: `rdx:rax = rax * rd` (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `mul dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Unsigned multiply: `rdx:rax = rax * [dst]` (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |

### `imul`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `imul rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Signed multiply: `rdx:rax = rax * rd` (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `imul dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Signed multiply: `rdx:rax = rax * [dst]` (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |
| `imul rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd * rs`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd * [src]`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | `rd = rs * imm`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul rd, src, imm` | `<R>`, `rd: R`, `src: MemOperand`, `imm: int` | emits `Bytes<...>` | `rd = [src] * imm`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |

### `div`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `div rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Unsigned divide: `rax = rdx:rax / rd`, `rdx` = the remainder (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `div dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Unsigned divide: `rax = rdx:rax / [dst]`, `rdx` = the remainder (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |

### `idiv`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `idiv rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Signed divide: `rax = rdx:rax / rd`, `rdx` = the remainder (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `idiv dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Signed divide: `rax = rdx:rax / [dst]`, `rdx` = the remainder (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |

### `inc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `inc rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = rd + 1`, leaving CF alone. | [`std.x86_64.impl`](impl.md) |
| `inc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + 1`, leaving CF alone, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `dec`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dec rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = rd - 1`, leaving CF alone. | [`std.x86_64.impl`](impl.md) |
| `dec dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - 1`, leaving CF alone, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `rol`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rol rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` left by `imm`. | [`std.x86_64.impl`](impl.md) |
| `rol rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` left by `cl`. | [`std.x86_64.impl`](impl.md) |
| `rol dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` left by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rol dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` left by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `ror`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ror rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` right by `imm`. | [`std.x86_64.impl`](impl.md) |
| `ror rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` right by `cl`. | [`std.x86_64.impl`](impl.md) |
| `ror dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` right by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `ror dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` right by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `rcl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcl rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` and CF left by `imm`. | [`std.x86_64.impl`](impl.md) |
| `rcl rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` and CF left by `cl`. | [`std.x86_64.impl`](impl.md) |
| `rcl dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF left by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rcl dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF left by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `rcr`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` and CF right by `imm`. | [`std.x86_64.impl`](impl.md) |
| `rcr rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` and CF right by `cl`. | [`std.x86_64.impl`](impl.md) |
| `rcr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF right by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rcr dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF right by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |

### `shld`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shld rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `rd` left by `imm`, filling from the top of `rs`. | [`std.x86_64.impl`](impl.md) |
| `shld rd, rs, count` | `<R>`, `rd: R`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `rd` left by `cl`, filling from the top of `rs`. | [`std.x86_64.impl`](impl.md) |
| `shld dst, rs, imm` | `<R>`, `dst: MemOperand`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `[dst]` left by `imm`, filling from the top of `rs`. | [`std.x86_64.impl`](impl.md) |
| `shld dst, rs, count` | `<R>`, `dst: MemOperand`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `[dst]` left by `cl`, filling from the top of `rs`. | [`std.x86_64.impl`](impl.md) |

### `shrd`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shrd rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `rd` right by `imm`, filling from the bottom of `rs`. | [`std.x86_64.impl`](impl.md) |
| `shrd rd, rs, count` | `<R>`, `rd: R`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `rd` right by `cl`, filling from the bottom of `rs`. | [`std.x86_64.impl`](impl.md) |
| `shrd dst, rs, imm` | `<R>`, `dst: MemOperand`, `rs: R`, `imm: int` | emits `Bytes<...>` | Shifts `[dst]` right by `imm`, filling from the bottom of `rs`. | [`std.x86_64.impl`](impl.md) |
| `shrd dst, rs, count` | `<R>`, `dst: MemOperand`, `rs: R`, `count: Reg8` | emits `Bytes<...>` | Shifts `[dst]` right by `cl`, filling from the bottom of `rs`. | [`std.x86_64.impl`](impl.md) |

### `bt`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bt rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF. | [`std.x86_64.impl`](impl.md) |
| `bt rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF. | [`std.x86_64.impl`](impl.md) |
| `bt dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF. The bit string can extend past `dst`. | [`std.x86_64.impl`](impl.md) |
| `bt dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `bt _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `bt [dst], imm` needs a size, like `bt qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `bts`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bts rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF, then sets it. | [`std.x86_64.impl`](impl.md) |
| `bts rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF, then sets it. | [`std.x86_64.impl`](impl.md) |
| `bts dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF, then sets it. The bit string can extend past `dst`. | [`std.x86_64.impl`](impl.md) |
| `bts dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, then sets it, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `bts _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `bts [dst], imm` needs a size, like `bts qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `btr`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `btr rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF, then clears it. | [`std.x86_64.impl`](impl.md) |
| `btr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF, then clears it. | [`std.x86_64.impl`](impl.md) |
| `btr dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF, then clears it. The bit string can extend past `dst`. | [`std.x86_64.impl`](impl.md) |
| `btr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, then clears it, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `btr _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `btr [dst], imm` needs a size, like `btr qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `btc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `btc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `rd` into CF, then flips it. | [`std.x86_64.impl`](impl.md) |
| `btc rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Copies bit `imm` of `rd` into CF, then flips it. | [`std.x86_64.impl`](impl.md) |
| `btc dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | Copies bit `rs` of `[dst]` into CF, then flips it. The bit string can extend past `dst`. | [`std.x86_64.impl`](impl.md) |
| `btc dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Copies bit `imm` of `[dst]` into CF, then flips it, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `btc _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `btc [dst], imm` needs a size, like `btc qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |

### `bsf`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bsf rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets `rd` to the index of the lowest set bit of `rs`; ZF = 1 if `rs` is 0. | [`std.x86_64.impl`](impl.md) |
| `bsf rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets `rd` to the index of the lowest set bit of `[src]`; ZF = 1 if it's 0. | [`std.x86_64.impl`](impl.md) |

### `bsr`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bsr rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Sets `rd` to the index of the highest set bit of `rs`; ZF = 1 if `rs` is 0. | [`std.x86_64.impl`](impl.md) |
| `bsr rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Sets `rd` to the index of the highest set bit of `[src]`; ZF = 1 if it's 0. | [`std.x86_64.impl`](impl.md) |

### `bswap`

Reverses the byte order of the 32- or 64-bit `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bswap rd` | `<R>`, `rd: R` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `movzx`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movzx rd, rs` | `<D, S>`, `rd: D`, `rs: S` | emits `Bytes<...>` | `rd = rs`, zero-extended. | [`std.x86_64.impl`](impl.md) |
| `movzx rd, src, size` | `<D>`, `rd: D`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads the `size`-wide value at `src` into `rd`, zero-extended. | [`std.x86_64.impl`](impl.md) |

### `movsx`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsx rd, rs` | `<D, S>`, `rd: D`, `rs: S` | emits `Bytes<...>` | `rd = rs`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `movsx rd, src, size` | `<D>`, `rd: D`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Loads the `size`-wide value at `src` into `rd`, sign-extended. | [`std.x86_64.impl`](impl.md) |

### `movsxd`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsxd rd, rs` | `<D>`, `rd: D`, `rs: Reg32` | emits `Bytes<...>` | `rd = rs`, a 32-bit register sign-extended into the 64-bit `rd`. | [`std.x86_64.impl`](impl.md) |
| `movsxd rd, src` | `<D>`, `rd: D`, `src: MemOperand` | emits `Bytes<...>` | Loads the 32 bits at `src` into the 64-bit `rd`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `movsxd rd, src, size` | `<D>`, `rd: D`, `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `movsxd`; `size` must be `dword`. | [`std.x86_64.impl`](impl.md) |

### `cmovo`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovo rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` on signed overflow (OF = 1). | [`std.x86_64.impl`](impl.md) |
| `cmovo rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` on signed overflow (OF = 1). | [`std.x86_64.impl`](impl.md) |

### `cmovno`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovno rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` without signed overflow (OF = 0). | [`std.x86_64.impl`](impl.md) |
| `cmovno rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` without signed overflow (OF = 0). | [`std.x86_64.impl`](impl.md) |

### `cmovb`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if below, unsigned (CF = 1). | [`std.x86_64.impl`](impl.md) |
| `cmovb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if below, unsigned (CF = 1). | [`std.x86_64.impl`](impl.md) |

### `cmovae`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovae rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if above or equal, unsigned (CF = 0). | [`std.x86_64.impl`](impl.md) |
| `cmovae rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if above or equal, unsigned (CF = 0). | [`std.x86_64.impl`](impl.md) |

### `cmove`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmove rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if equal (ZF = 1). | [`std.x86_64.impl`](impl.md) |
| `cmove rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if equal (ZF = 1). | [`std.x86_64.impl`](impl.md) |

### `cmovne`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovne rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if not equal (ZF = 0). | [`std.x86_64.impl`](impl.md) |
| `cmovne rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if not equal (ZF = 0). | [`std.x86_64.impl`](impl.md) |

### `cmovbe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovbe rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if below or equal, unsigned. | [`std.x86_64.impl`](impl.md) |
| `cmovbe rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if below or equal, unsigned. | [`std.x86_64.impl`](impl.md) |

### `cmova`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmova rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if above, unsigned. | [`std.x86_64.impl`](impl.md) |
| `cmova rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if above, unsigned. | [`std.x86_64.impl`](impl.md) |

### `cmovs`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovs rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the result was negative (SF = 1). | [`std.x86_64.impl`](impl.md) |
| `cmovs rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the result was negative (SF = 1). | [`std.x86_64.impl`](impl.md) |

### `cmovns`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovns rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the result wasn't negative (SF = 0). | [`std.x86_64.impl`](impl.md) |
| `cmovns rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the result wasn't negative (SF = 0). | [`std.x86_64.impl`](impl.md) |

### `cmovp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovp rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the parity flag is set (PF = 1). | [`std.x86_64.impl`](impl.md) |
| `cmovp rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the parity flag is set (PF = 1). | [`std.x86_64.impl`](impl.md) |

### `cmovnp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnp rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if the parity flag is clear (PF = 0). | [`std.x86_64.impl`](impl.md) |
| `cmovnp rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if the parity flag is clear (PF = 0). | [`std.x86_64.impl`](impl.md) |

### `cmovl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovl rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if less, signed. | [`std.x86_64.impl`](impl.md) |
| `cmovl rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if less, signed. | [`std.x86_64.impl`](impl.md) |

### `cmovge`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovge rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if greater or equal, signed. | [`std.x86_64.impl`](impl.md) |
| `cmovge rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if greater or equal, signed. | [`std.x86_64.impl`](impl.md) |

### `cmovle`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovle rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if less or equal, signed. | [`std.x86_64.impl`](impl.md) |
| `cmovle rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if less or equal, signed. | [`std.x86_64.impl`](impl.md) |

### `cmovg`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovg rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rs` if greater, signed. | [`std.x86_64.impl`](impl.md) |
| `cmovg rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Loads the value at `src` into `rd` if greater, signed. | [`std.x86_64.impl`](impl.md) |

### `cmovz`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovz rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmove`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovz rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmove`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnz`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnz rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovne`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnz rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovne`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovb`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovc rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovb`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnae`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnae rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovb`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnae rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovb`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnc rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovae`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnc rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovae`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnb`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovae`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovae`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnbe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnbe rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmova`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnbe rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmova`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovna`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovna rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovbe`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovna rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovbe`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnge`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnge rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovl`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnge rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovl`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnl rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovge`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnl rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovge`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovng`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovng rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovle`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovng rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovle`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovnle`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovnle rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovg`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovnle rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovg`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovpe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovpe rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovp`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovpe rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovp`, under another name. | [`std.x86_64.impl`](impl.md) |

### `cmovpo`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmovpo rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `cmovnp`, under another name. | [`std.x86_64.impl`](impl.md) |
| `cmovpo rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `cmovnp`, under another name. | [`std.x86_64.impl`](impl.md) |

### `seto`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `seto rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 on signed overflow (OF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `seto dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 on signed overflow (OF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `seto dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `seto dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setno`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setno rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 without signed overflow (OF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setno dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 without signed overflow (OF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setno dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setno dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setb`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setb rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if below, unsigned (CF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `setb dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if below, unsigned (CF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `setb dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setb dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setae`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setae rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if above or equal, unsigned (CF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setae dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if above or equal, unsigned (CF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setae dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setae dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `sete`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sete rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if equal (ZF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `sete dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if equal (ZF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `sete dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `sete dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setne`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setne rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if not equal (ZF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setne dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if not equal (ZF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setne dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setne dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setbe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setbe rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if below or equal, unsigned, else 0. | [`std.x86_64.impl`](impl.md) |
| `setbe dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if below or equal, unsigned, else 0. | [`std.x86_64.impl`](impl.md) |
| `setbe dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setbe dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `seta`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `seta rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if above, unsigned, else 0. | [`std.x86_64.impl`](impl.md) |
| `seta dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if above, unsigned, else 0. | [`std.x86_64.impl`](impl.md) |
| `seta dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `seta dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `sets`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sets rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the result was negative (SF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `sets dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the result was negative (SF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `sets dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `sets dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setns`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setns rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the result wasn't negative (SF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setns dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the result wasn't negative (SF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setns dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setns dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setp rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the parity flag is set (PF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `setp dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the parity flag is set (PF = 1), else 0. | [`std.x86_64.impl`](impl.md) |
| `setp dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setp dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnp rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if the parity flag is clear (PF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setnp dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if the parity flag is clear (PF = 0), else 0. | [`std.x86_64.impl`](impl.md) |
| `setnp dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnp dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setl rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if less, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setl dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if less, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setl dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setl dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setge`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setge rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if greater or equal, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setge dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if greater or equal, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setge dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setge dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setle`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setle rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if less or equal, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setle dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if less or equal, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setle dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setle dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setg`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setg rd` | `rd: Reg8` | emits `Bytes<...>` | Sets `rd` to 1 if greater, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setg dst` | `dst: MemOperand` | emits `Bytes<...>` | Sets the byte at `dst` to 1 if greater, signed, else 0. | [`std.x86_64.impl`](impl.md) |
| `setg dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setg dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setz`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setz rd` | `rd: Reg8` | emits `Bytes<...>` | `sete`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setz dst` | `dst: MemOperand` | emits `Bytes<...>` | `sete`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setz dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setz dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnz`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnz rd` | `rd: Reg8` | emits `Bytes<...>` | `setne`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnz dst` | `dst: MemOperand` | emits `Bytes<...>` | `setne`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnz dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnz dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setc rd` | `rd: Reg8` | emits `Bytes<...>` | `setb`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setc dst` | `dst: MemOperand` | emits `Bytes<...>` | `setb`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setc dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnae`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnae rd` | `rd: Reg8` | emits `Bytes<...>` | `setb`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnae dst` | `dst: MemOperand` | emits `Bytes<...>` | `setb`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnae dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnae dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnc rd` | `rd: Reg8` | emits `Bytes<...>` | `setae`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnc dst` | `dst: MemOperand` | emits `Bytes<...>` | `setae`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnc dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnb`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnb rd` | `rd: Reg8` | emits `Bytes<...>` | `setae`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnb dst` | `dst: MemOperand` | emits `Bytes<...>` | `setae`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnb dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnb dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnbe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnbe rd` | `rd: Reg8` | emits `Bytes<...>` | `seta`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnbe dst` | `dst: MemOperand` | emits `Bytes<...>` | `seta`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnbe dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnbe dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setna`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setna rd` | `rd: Reg8` | emits `Bytes<...>` | `setbe`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setna dst` | `dst: MemOperand` | emits `Bytes<...>` | `setbe`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setna dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setna dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnge`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnge rd` | `rd: Reg8` | emits `Bytes<...>` | `setl`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnge dst` | `dst: MemOperand` | emits `Bytes<...>` | `setl`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnge dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnge dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnl rd` | `rd: Reg8` | emits `Bytes<...>` | `setge`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnl dst` | `dst: MemOperand` | emits `Bytes<...>` | `setge`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnl dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnl dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setng`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setng rd` | `rd: Reg8` | emits `Bytes<...>` | `setle`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setng dst` | `dst: MemOperand` | emits `Bytes<...>` | `setle`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setng dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setng dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setnle`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnle rd` | `rd: Reg8` | emits `Bytes<...>` | `setg`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnle dst` | `dst: MemOperand` | emits `Bytes<...>` | `setg`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setnle dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setnle dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setpe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setpe rd` | `rd: Reg8` | emits `Bytes<...>` | `setp`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setpe dst` | `dst: MemOperand` | emits `Bytes<...>` | `setp`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setpe dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setpe dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `setpo`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setpo rd` | `rd: Reg8` | emits `Bytes<...>` | `setnp`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setpo dst` | `dst: MemOperand` | emits `Bytes<...>` | `setnp`, under another name. | [`std.x86_64.impl`](impl.md) |
| `setpo dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `setpo dst`; `size` must be `byte`. | [`std.x86_64.impl`](impl.md) |

### `jmp_far`

Jumps to the far pointer at `src`: a 64-bit offset, then a 16-bit
segment selector.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jmp far src` | `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `call_far`

Calls the far pointer at `src`: a 64-bit offset, then a 16-bit segment
selector.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call far src` | `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `retf`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `retf` |  | emits `Bytes<...>` | Returns from a far call. | [`std.x86_64.impl`](impl.md) |
| `retf n` | `n: int` | emits `Bytes<...>` | Returns from a far call, then pops `n` more bytes. | [`std.x86_64.impl`](impl.md) |

### `enter`

Makes a stack frame of `size` bytes, nested `level` deep.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `enter size, level` | `size: int`, `level: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `leave`

Undoes `enter`: `mov rsp, rbp`, then `pop rbp`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `leave` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `interrupt`

Calls interrupt handler `n`. The dialects spell it `int n`; `int` itself
names the integer type.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `int n` | `n: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `loop`

Decrements `rcx`, then jumps to the label `target` unless it's 0. `target`
must be within 127 bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `loop target` | `target: label` | emits `Rel8Instr` |  | [`std.x86_64.impl`](impl.md) |

### `loope`

Decrements `rcx`, then jumps to the label `target` if it isn't 0 and ZF = 1.
`target` must be within 127 bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `loope target` | `target: label` | emits `Rel8Instr` |  | [`std.x86_64.impl`](impl.md) |

### `loopz`

`loope`, under another name. `target` must be within 127 bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `loopz target` | `target: label` | emits `Rel8Instr` |  | [`std.x86_64.impl`](impl.md) |

### `loopne`

Decrements `rcx`, then jumps to the label `target` if it isn't 0 and ZF = 0.
`target` must be within 127 bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `loopne target` | `target: label` | emits `Rel8Instr` |  | [`std.x86_64.impl`](impl.md) |

### `loopnz`

`loopne`, under another name. `target` must be within 127 bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `loopnz target` | `target: label` | emits `Rel8Instr` |  | [`std.x86_64.impl`](impl.md) |

### `jrcxz`

Jumps to the label `target` if `rcx` is 0. `target` must be within 127 bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jrcxz target` | `target: label` | emits `Rel8Instr` |  | [`std.x86_64.impl`](impl.md) |

### `jecxz`

Jumps to the label `target` if `ecx` is 0. `target` must be within 127
bytes.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jecxz target` | `target: label` | emits `Rel8Instr2` |  | [`std.x86_64.impl`](impl.md) |

### `nop`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `nop` |  | emits `Bytes<...>` | Does nothing. | [`std.x86_64.impl`](impl.md) |
| `nop rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Does nothing, in a longer encoding that names `rd`: for padding. | [`std.x86_64.impl`](impl.md) |
| `nop dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Does nothing, in a longer encoding that names `dst`: for padding. | [`std.x86_64.impl`](impl.md) |

### `ud0`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ud0 rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Raises an invalid-opcode exception. | [`std.x86_64.impl`](impl.md) |
| `ud0 rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Raises an invalid-opcode exception. | [`std.x86_64.impl`](impl.md) |

### `ud1`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ud1 rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | Raises an invalid-opcode exception. | [`std.x86_64.impl`](impl.md) |
| `ud1 rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | Raises an invalid-opcode exception. | [`std.x86_64.impl`](impl.md) |

### `out`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `out port, rs` | `<R>`, `port: Reg16`, `rs: R` | emits `Bytes<...>` | Writes `rs` (`al`, `ax` or `eax`) to the I/O port in `port`, which must be `dx`. | [`std.x86_64.impl`](impl.md) |
| `out port, rs` | `<R>`, `port: int`, `rs: R` | emits `Bytes<...>` | Writes `rs` (`al`, `ax` or `eax`) to I/O port `port`, 0 to 255. | [`std.x86_64.impl`](impl.md) |

### `lfs`

Loads the far pointer at `src` into `fs` and `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lfs rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lgs`

Loads the far pointer at `src` into `gs` and `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lgs rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lss`

Loads the far pointer at `src` into `ss` and `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lss rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `clc`

Clears the carry flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clc` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `stc`

Sets the carry flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `stc` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cmc`

Flips the carry flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmc` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cld`

Clears the direction flag, so string instructions count up.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cld` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `std`

Sets the direction flag, so string instructions count down.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `std` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cli`

Clears the interrupt flag. Needs I/O privilege.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cli` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `sti`

Sets the interrupt flag. Needs I/O privilege.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sti` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cbw`

Sign-extends `al` into `ax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cbw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cwde`

Sign-extends `ax` into `eax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cwde` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cdqe`

Sign-extends `eax` into `rax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cdqe` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cwd`

Sign-extends `ax` into `dx:ax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cwd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cdq`

Sign-extends `eax` into `edx:eax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cdq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cqo`

Sign-extends `rax` into `rdx:rax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cqo` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lahf`

Loads the low byte of the flags into `ah`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lahf` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `sahf`

Stores `ah` into the low byte of the flags.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sahf` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `pushf`

Pushes the flags.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pushf` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `pushfq`

Pushes the flags.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pushfq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `popf`

Pops the flags.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `popf` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `popfq`

Pops the flags.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `popfq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `pause`

Hints that this is a spin-wait loop.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pause` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `ud2`

Raises an invalid-opcode exception.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ud2` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cpuid`

Reads processor identification and features, picked by `eax`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cpuid` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `int3`

Stops in a debugger.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `int3` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `iret`

Returns from an interrupt.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `iret` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `iretd`

Returns from an interrupt.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `iretd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `iretq`

Returns from an interrupt, with 64-bit stack values.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `iretq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `xlatb`

`al = [rbx + al]`: looks `al` up in a table.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `xlatb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `rep`

Repeats the next string instruction `rcx` times.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rep` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `repe`

Repeats the next `cmps` or `scas` while equal, up to `rcx` times.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repe` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `repz`

`repe`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repz` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `repne`

Repeats the next `cmps` or `scas` while not equal, up to `rcx` times.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repne` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `repnz`

`repne`, under another name.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repnz` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lock`

Makes the next instruction's memory access atomic.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lock` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `movsb`

Copies a byte from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `movsw`

Copies a word from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `movsd`

Copies a doubleword from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `movsq`

Copies a quadword from `[rsi]` to `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cmpsb`

Compares a byte at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpsb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cmpsw`

Compares a word at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpsw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cmpsd`

Compares a doubleword at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpsd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `cmpsq`

Compares a quadword at `[rsi]` and `[rdi]`, then steps both.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpsq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `scasb`

Compares the accumulator with a byte at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `scasb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `scasw`

Compares the accumulator with a word at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `scasw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `scasd`

Compares the accumulator with a doubleword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `scasd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `scasq`

Compares the accumulator with a quadword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `scasq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lodsb`

Loads a byte from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lodsb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lodsw`

Loads a word from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lodsw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lodsd`

Loads a doubleword from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lodsd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `lodsq`

Loads a quadword from `[rsi]` into the accumulator, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lodsq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `stosb`

Stores the accumulator's low byte at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `stosb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `stosw`

Stores the accumulator's low word at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `stosw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `stosd`

Stores the accumulator's low doubleword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `stosd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `stosq`

Stores the accumulator's low quadword at `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `stosq` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `insb`

Reads a byte from I/O port `dx` into `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `insb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `insw`

Reads a word from I/O port `dx` into `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `insw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `insd`

Reads a doubleword from I/O port `dx` into `[rdi]`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `insd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `outsb`

Writes a byte from `[rsi]` to I/O port `dx`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `outsb` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `outsw`

Writes a word from `[rsi]` to I/O port `dx`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `outsw` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `outsd`

Writes a doubleword from `[rsi]` to I/O port `dx`, then steps it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `outsd` |  | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `assert_valid_reg`

Fails to compile unless `r` is a register number, 0 to 15.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `assert_valid_reg r` | `r: Reg` |  |  | [`std.x86_64.intel`](intel.md) |

### `mem_base`

`[base]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[base]` | `base: Reg` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `mem_plus`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[base+disp]` | `base: Reg`, `disp: int` | returns `MemOperand` | `[base+disp]`. | [`std.x86_64.intel`](intel.md) |
| `[base+index]` | `base: Reg`, `index: Reg` | returns `MemOperand` | `[base+index]`, an index with scale 1. | [`std.x86_64.intel`](intel.md) |

### `mem_minus`

`[base-disp]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[base-disp]` | `base: Reg`, `disp: int` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `mem_scaled`

`[base+index*scale]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[base+index*scale]` | `base: Reg`, `index: Reg`, `scale: int` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `mem_indexed`

`[base+index*scale+disp]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[base+index*scale+disp]` | `base: Reg`, `index: Reg`, `scale: int`, `disp: int` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `mem_indexed_minus`

`[base+index*scale-disp]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[base+index*scale-disp]` | `base: Reg`, `index: Reg`, `scale: int`, `disp: int` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `mem_rip`

`[rip+disp]`: `disp` bytes past the end of the instruction.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[rip+disp]` | `disp: int` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `mem_rip_minus`

`[rip-disp]`: `disp` bytes before the end of the instruction.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `[rip-disp]` | `disp: int` | returns `MemOperand` |  | [`std.x86_64.intel`](intel.md) |

### `add_sized`

`add size [dst], src`, as in `add qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `add size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `or_sized`

`or size [dst], src`, as in `or qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `or size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `adc_sized`

`adc size [dst], src`, as in `adc qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `adc size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `sbb_sized`

`sbb size [dst], src`, as in `sbb qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbb size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `and_sized`

`and size [dst], src`, as in `and qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `and size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `sub_sized`

`sub size [dst], src`, as in `sub qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sub size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `xor_sized`

`xor size [dst], src`, as in `xor qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `xor size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `cmp_sized`

`cmp size [dst], src`, as in `cmp qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmp size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `test_sized`

`test size [dst], src`, as in `test qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `test size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `mov_sized`

`mov size [dst], src`, as in `mov qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mov size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `rol_sized`

`rol size [dst], src`, as in `rol qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rol size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `ror_sized`

`ror size [dst], src`, as in `ror qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ror size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `rcl_sized`

`rcl size [dst], src`, as in `rcl qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcl size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `rcr_sized`

`rcr size [dst], src`, as in `rcr qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcr size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `shl_sized`

`shl size [dst], src`, as in `shl qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shl size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `shr_sized`

`shr size [dst], src`, as in `shr qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `shr size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `sar_sized`

`sar size [dst], src`, as in `sar qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sar size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `bt_sized`

`bt size [dst], src`, as in `bt qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bt size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `bts_sized`

`bts size [dst], src`, as in `bts qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bts size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `btr_sized`

`btr size [dst], src`, as in `btr qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `btr size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `btc_sized`

`btc size [dst], src`, as in `btc qword [rbx], 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `btc size dst, src` | `<T>`, `size: OperandSize`, `dst: MemOperand`, `src: T` |  |  | [`std.x86_64.intel`](intel.md) |

### `not_sized`

`not size [dst]`, as in `not qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `not size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `neg_sized`

`neg size [dst]`, as in `neg qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `neg size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `mul_sized`

`mul size [dst]`, as in `mul qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mul size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `imul_sized`

`imul size [dst]`, as in `imul qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `imul size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `div_sized`

`div size [dst]`, as in `div qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `div size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `idiv_sized`

`idiv size [dst]`, as in `idiv qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `idiv size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `inc_sized`

`inc size [dst]`, as in `inc qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `inc size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `dec_sized`

`dec size [dst]`, as in `dec qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dec size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `push_sized`

`push size [dst]`, as in `push qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `push size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `pop_sized`

`pop size [dst]`, as in `pop qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pop size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `jmp_sized`

`jmp size [dst]`, as in `jmp qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jmp size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `call_sized`

`call size [dst]`, as in `call qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `nop_sized`

`nop size [dst]`, as in `nop qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `nop size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `cmpxchg8b_sized`

`cmpxchg8b size [dst]`, as in `cmpxchg8b qword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpxchg8b size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `cmpxchg16b_sized`

`cmpxchg16b size [dst]`, as in `cmpxchg16b oword [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cmpxchg16b size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `seto_sized`

`seto size [dst]`, as in `seto byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `seto size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setno_sized`

`setno size [dst]`, as in `setno byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setno size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setb_sized`

`setb size [dst]`, as in `setb byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setb size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setae_sized`

`setae size [dst]`, as in `setae byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setae size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `sete_sized`

`sete size [dst]`, as in `sete byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sete size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setne_sized`

`setne size [dst]`, as in `setne byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setne size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setbe_sized`

`setbe size [dst]`, as in `setbe byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setbe size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `seta_sized`

`seta size [dst]`, as in `seta byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `seta size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `sets_sized`

`sets size [dst]`, as in `sets byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sets size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setns_sized`

`setns size [dst]`, as in `setns byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setns size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setp_sized`

`setp size [dst]`, as in `setp byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setp size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnp_sized`

`setnp size [dst]`, as in `setnp byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnp size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setl_sized`

`setl size [dst]`, as in `setl byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setl size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setge_sized`

`setge size [dst]`, as in `setge byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setge size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setle_sized`

`setle size [dst]`, as in `setle byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setle size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setg_sized`

`setg size [dst]`, as in `setg byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setg size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setz_sized`

`setz size [dst]`, as in `setz byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setz size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnz_sized`

`setnz size [dst]`, as in `setnz byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnz size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setc_sized`

`setc size [dst]`, as in `setc byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setc size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnae_sized`

`setnae size [dst]`, as in `setnae byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnae size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnc_sized`

`setnc size [dst]`, as in `setnc byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnc size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnb_sized`

`setnb size [dst]`, as in `setnb byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnb size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnbe_sized`

`setnbe size [dst]`, as in `setnbe byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnbe size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setna_sized`

`setna size [dst]`, as in `setna byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setna size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnge_sized`

`setnge size [dst]`, as in `setnge byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnge size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnl_sized`

`setnl size [dst]`, as in `setnl byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnl size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setng_sized`

`setng size [dst]`, as in `setng byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setng size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setnle_sized`

`setnle size [dst]`, as in `setnle byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setnle size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setpe_sized`

`setpe size [dst]`, as in `setpe byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setpe size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `setpo_sized`

`setpo size [dst]`, as in `setpo byte [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `setpo size dst` | `size: OperandSize`, `dst: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `movzx_from_sized`

`movzx rd, size [src]`, as in `movzx eax, byte [rsi]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movzx rd, size src` | `<T>`, `rd: T`, `size: OperandSize`, `src: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `movsx_from_sized`

`movsx rd, size [src]`, as in `movsx rax, word [rsi]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsx rd, size src` | `<T>`, `rd: T`, `size: OperandSize`, `src: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `movsxd_from_sized`

`movsxd rd, size [src]`, as in `movsxd rax, dword [rsi]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movsxd rd, size src` | `<T>`, `rd: T`, `size: OperandSize`, `src: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `mov_from_sized`

`mov rd, size [src]`, as in `mov fs, word [rbx]`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mov rd, size src` | `<T>`, `rd: T`, `size: OperandSize`, `src: MemOperand` |  |  | [`std.x86_64.intel`](intel.md) |

### `rel`

`[rel label]`: the memory at `label`, addressed relative to the next
instruction, so the code works wherever it's loaded.

```basm
from std.x86_64.nasm import *

lea rsi, [rel msg]    # msg is 6 bytes past the end of this instruction
mov eax, [rel msg]    # and right after this one

msg:
    db "Hi", 10
```

```bytes
48 8d 35 06 00 00 00
8b 05 00 00 00 00
48 69 0a
```

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `[rel target]` | `target: label` | returns `RipLabel` |  |

### `data_int`

`value` as `width` little-endian bytes, truncated the way NASM does.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `data_int(value, width)` | `value: int`, `width: int` | returns `Bytes<...>` |  |

### `data_string`

A string literal's UTF-8 bytes, with no terminator.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `data_string(source)` | `<S>`, `source: S` | returns `Bytes<...>` |  |

### `db`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `db a` | `a: int` |  | Bytes. Each operand is an integer, stored in one byte, or a string literal, stored as its UTF-8 bytes with no terminator (add `, 0` yourself). Up to four operands; `db "Hello", 10` is the common case. |
| `db a` | `<S>`, `a: S` |  |  |
| `db a, b` | `<S>`, `a: S`, `b: int` |  |  |
| `db a, b, c` | `<S>`, `a: S`, `b: int`, `c: int` |  |  |
| `db a, b` | `a: int`, `b: int` |  |  |
| `db a, b, c` | `a: int`, `b: int`, `c: int` |  |  |
| `db a, b, c, d` | `a: int`, `b: int`, `c: int`, `d: int` |  |  |

### `dw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dw a` | `a: int` |  | 16-bit little-endian words, one or two per line. |
| `dw a, b` | `a: int`, `b: int` |  |  |

### `dd`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dd a` | `a: int` |  | 32-bit little-endian doublewords, one or two per line. |
| `dd a, b` | `a: int`, `b: int` |  |  |

### `dq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dq a` | `a: int` |  | 64-bit little-endian quadwords, one or two per line. |
| `dq a, b` | `a: int`, `b: int` |  |  |

## Re-exported

From [`std.x86_64.impl`](impl.md): `Reg`, `r0`, `r1`, `r2`, `r3`, `r4`, `r5`, `r6`, `r7`, `r8`, `r9`, `r10`, `r11`, `r12`, `r13`, `r14`, `r15`, `rax`, `rcx`, `rdx`, `rbx`, `rsp`, `rbp`, `rsi`, `rdi`, `Reg32`, `eax`, `ecx`, `edx`, `ebx`, `esp`, `ebp`, `esi`, `edi`, `r8d`, `r9d`, `r10d`, `r11d`, `r12d`, `r13d`, `r14d`, `r15d`, `Reg16`, `ax`, `cx`, `dx`, `bx`, `sp`, `bp`, `si`, `di`, `r8w`, `r9w`, `r10w`, `r11w`, `r12w`, `r13w`, `r14w`, `r15w`, `Reg8`, `al`, `cl`, `dl`, `bl`, `spl`, `bpl`, `sil`, `dil`, `r8b`, `r9b`, `r10b`, `r11b`, `r12b`, `r13b`, `r14b`, `r15b`, `ah`, `ch`, `dh`, `bh`, `SReg`, `es`, `cs`, `ss`, `ds`, `fs`, `gs`, `Gpr`, `Byte`, `Bytes`, `OperandSize`, `byte`, `word`, `dword`, `qword`, `oword`, `MemBase`, `MemSib`, `MemRip`, `MemOperand`, `Rel32Instr`, `Rel32Instr2`, `RipLabel`, `RipRelInstr`, `RipRelInstrRex`, `Rel8Instr`, `Rel8Instr2`.
