# `att`

[`std`](../index.md) › [`x86_64`](index.md) › `att`

AT&T-syntax x86-64 assembly, as GNU `as` reads it: source first,
destination second, `%` before registers and `$` before immediates.

```basm
from std.x86_64.att import *

mov %rbx, %rax
mov 8(%rsp), %rax
add $1, %rax
movl $2, %eax
movb %dl, 1(%rdi)
movb $124, 67(%r9)
movzbl (%r8,%rdx), %edx
```

```bytes
48 89 d8
48 8b 44 24 08
48 83 c0 01
b8 02 00 00 00
88 57 01
41 c6 41 43 7c
41 0f b6 14 10
```

Every operand is a value: `%rax` is a `Register`, `$1` an `Imm`, and each
memory form a `Memory`: `(%rbx)`, `8(%rbx)`, `(%rbx,%rcx)`,
`8(%rbx,%rcx)`, `(%rbx,%rcx,4)`, `8(%rbx,%rcx,4)` and `8(%rip)`. A
displacement can be any expression, like `-4(%rbp)`.

`label(%rip)` is the address of `label`, relative to the next
instruction, the way GNU `as` reads it; a number before `(%rip)` is a
plain displacement. A label is a `label`, not a number, which is how the
two are told apart.

A size suffix (`b`, `w`, `l` or `q`, as in `movl` or `cmpq`) is optional
when a register gives the size, and must match it. With only an
immediate and memory, nothing else says the size, so the suffix is
required: `movb $1, (%rdi)`. `movzbl`-style mnemonics extend a byte or
word into a larger register.

A string instruction can take its prefix on the same line: `rep stosb`.
`*%rax` and `*8(%rsp)` are indirect `jmp` and `call` targets. Data is
`db`, `dw`, `dd` and `dq`, from `std.x86_64.data`; labels, sections and
`pub` work as everywhere in BitterASM rather than as GNU directives.
Every instruction of `std.x86_64.impl` stays available in its explicit
form too: `mov rax, rbx, 0` for a 32-bit move.

Re-exports [`std.x86_64.impl`](impl.md) and [`std.x86_64.data`](data.md).

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
| `mov rd, imm, w` | `rd: Reg`, `imm: int`, `w: int` | emits `Bytes<...>` | `rd = imm`. With `w` = 1 it takes a full 64-bit `imm`, encoded as a sign-extended 32-bit one when it fits. | [`std.x86_64.impl`](impl.md) |
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
| `mov rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = imm`. A 64-bit `rd` takes a full 64-bit `imm`, encoded as a sign-extended 32-bit one when it fits. | [`std.x86_64.impl`](impl.md) |
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
| `mov src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = src`. |  |
| `mov src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `mov src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `mov src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `mov src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = src`. |  |
| `mov src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = src`. |  |
| `mov src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = src`. |  |
| `mov _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `mov _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `add src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst + src`. |  |
| `add src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `add src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `add src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `add src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst + src`. |  |
| `add src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst + src`. |  |
| `add src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst + src`. |  |
| `add _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `add _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `or src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst \| src`. |  |
| `or src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `or src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `or src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `or src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst \| src`. |  |
| `or src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst \| src`. |  |
| `or src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst \| src`. |  |
| `or _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `or _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `and src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst & src`. |  |
| `and src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `and src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `and src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `and src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst & src`. |  |
| `and src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst & src`. |  |
| `and src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst & src`. |  |
| `and _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `and _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `sub src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst - src`. |  |
| `sub src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `sub src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `sub src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `sub src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst - src`. |  |
| `sub src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst - src`. |  |
| `sub src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst - src`. |  |
| `sub _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `sub _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `xor src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst ^ src`. |  |
| `xor src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `xor src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `xor src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `xor src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst ^ src`. |  |
| `xor src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst ^ src`. |  |
| `xor src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst ^ src`. |  |
| `xor _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `xor _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `cmp src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | Sets the flags from `dst - src`, without storing it. |  |
| `cmp src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `cmp src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `cmp src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `cmp src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | Sets the flags from `dst - src`, without storing it. |  |
| `cmp src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | Sets the flags from `dst - src`, without storing it. |  |
| `cmp src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | Sets the flags from `dst - src`, without storing it. |  |
| `cmp _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `cmp _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `test src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | Sets the flags from `dst & src`, without storing it. |  |
| `test src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `test src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `test src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `test src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | Sets the flags from `dst & src`, without storing it. |  |
| `test src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | Sets the flags from `dst & src`, without storing it. |  |
| `test src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | Sets the flags from `dst & src`, without storing it. |  |
| `test _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `test _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `jmp target` | `target: Indirect<Register<Reg>>` |  | Jumps to the address in a register: `jmp *%rax`. |  |
| `jmp target` | `target: Indirect<Memory<MemOperand>>` |  | Jumps to the address stored in memory: `jmp *8(%rsp)`. |  |

### `call`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call target` | `target: label` | emits `Rel32Instr` | Pushes the address of the next instruction and jumps to the label `target`. | [`std.x86_64.impl`](impl.md) |
| `call r` | `r: Reg` | emits `Bytes<...>` | Calls the address in `r`. | [`std.x86_64.impl`](impl.md) |
| `call src` | `src: MemOperand` | emits `Bytes<...>` | Calls the address stored at `src`. | [`std.x86_64.impl`](impl.md) |
| `call src, size` | `src: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `call src`; `size` must be `qword`. | [`std.x86_64.impl`](impl.md) |
| `call target` | `target: Indirect<Register<Reg>>` |  | Calls the address in a register: `call *%rax`. |  |
| `call target` | `target: Indirect<Memory<MemOperand>>` |  | Calls the address stored in memory: `call *8(%rsp)`. |  |

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
| `shl count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | `dst = dst << count`. |  |
| `shl count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | `dst = dst << count`. `count` must be `%cl`. |  |
| `shl _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `shl _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

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
| `shr count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | `dst = dst >> count`, shifting in zeros. |  |
| `shr count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | `dst = dst >> count`, shifting in zeros. `count` must be `%cl`. |  |
| `shr _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `shr _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

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
| `sar count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | `dst = dst >> count`, shifting in copies of the sign bit. |  |
| `sar count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | `dst = dst >> count`, shifting in copies of the sign bit. `count` must be `%cl`. |  |
| `sar _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `sar _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

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
| `lea src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = the address of src`, without reading memory. |  |

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
| `push src` | `src: Register<Reg>` |  | Pushes `src`. |  |
| `push src` | `src: Register<Reg16>` |  | Pushes `src`. |  |
| `push src` | `src: Imm` |  | Pushes `src`, sign-extended to 64 bits. |  |
| `push src` | `src: Memory<MemOperand>` |  | Pushes the quadword at `src`. |  |

### `pop`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pop rd` | `rd: Reg` | emits `Bytes<...>` | Pops 64 bits off the stack into `rd`. | [`std.x86_64.impl`](impl.md) |
| `pop rd` | `rd: Reg16` | emits `Bytes<...>` | Pops 16 bits into `rd`. | [`std.x86_64.impl`](impl.md) |
| `pop dst` | `dst: MemOperand` | emits `Bytes<...>` | Pops 64 bits into the memory at `dst`. | [`std.x86_64.impl`](impl.md) |
| `pop dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Pops a `size`-wide value into the memory at `dst`: `qword` or `word`. | [`std.x86_64.impl`](impl.md) |
| `pop sr` | `sr: SReg` | emits `Bytes<...>` | Pops `fs` or `gs`, the only segment registers 64-bit mode can pop. | [`std.x86_64.impl`](impl.md) |
| `pop dst` | `dst: Register<Reg>` |  | Pops into `dst`. |  |
| `pop dst` | `dst: Register<Reg16>` |  | Pops into `dst`. |  |
| `pop dst` | `dst: Memory<MemOperand>` |  | Pops a quadword into `dst`. |  |

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
| `adc src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst + src + CF`. |  |
| `adc src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `adc src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `adc src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `adc src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst + src + CF`. |  |
| `adc src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst + src + CF`. |  |
| `adc src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst + src + CF`. |  |
| `adc _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `adc _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

### `sbb`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbb rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd - rs - CF`: subtract with borrow. | [`std.x86_64.impl`](impl.md) |
| `sbb rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | `rd = rd - imm - CF`: subtract with borrow; a 64-bit `rd` takes a 32-bit `imm`, sign-extended. | [`std.x86_64.impl`](impl.md) |
| `sbb rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd - [src] - CF`: subtract with borrow. | [`std.x86_64.impl`](impl.md) |
| `sbb dst, rs` | `<R>`, `dst: MemOperand`, `rs: R` | emits `Bytes<...>` | `[dst] = [dst] - rs - CF`: subtract with borrow. | [`std.x86_64.impl`](impl.md) |
| `sbb dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - imm - CF`: subtract with borrow, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `sbb _dst, _imm` | `_dst: MemOperand`, `_imm: int` |  | Fails: `sbb [dst], imm` needs a size, like `sbb qword [rbx], 1`. | [`std.x86_64.impl`](impl.md) |
| `sbb src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst - src - CF`. |  |
| `sbb src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `sbb src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `sbb src, dst` | `src: Register<Reg8>`, `dst: Register<Reg8>` |  |  |  |
| `sbb src, dst` | `<R>`, `src: Imm`, `dst: Register<R>` |  | `dst = dst - src - CF`. |  |
| `sbb src, dst` | `<M, R>`, `src: Memory<M>`, `dst: Register<R>` |  | `dst = dst - src - CF`. |  |
| `sbb src, dst` | `<R, M>`, `src: Register<R>`, `dst: Memory<M>` |  | `dst = dst - src - CF`. |  |
| `sbb _src, _dst` | `_src: Imm`, `_dst: Memory<MemOperand>` |  | Fails: an immediate and a memory operand need a size suffix, like `movq $1, (%rdi)`. |  |
| `sbb _src, _dst` | `_src: Imm`, `_dst: Memory<RipLabel>` |  | Fails: an immediate can't be stored at a `label(%rip)` operand. |  |

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
| `not dst` | `dst: Register<Reg>` |  | `dst = ~dst`. |  |
| `not dst` | `dst: Register<Reg32>` |  |  |  |
| `not dst` | `dst: Register<Reg16>` |  |  |  |
| `not dst` | `dst: Register<Reg8>` |  |  |  |
| `not _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `neg`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `neg rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = -rd`. | [`std.x86_64.impl`](impl.md) |
| `neg dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = -[dst]`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `neg dst` | `dst: Register<Reg>` |  | `dst = -dst`. |  |
| `neg dst` | `dst: Register<Reg32>` |  |  |  |
| `neg dst` | `dst: Register<Reg16>` |  |  |  |
| `neg dst` | `dst: Register<Reg8>` |  |  |  |
| `neg _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `mul`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mul rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Unsigned multiply: `rdx:rax = rax * rd` (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `mul dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Unsigned multiply: `rdx:rax = rax * [dst]` (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |
| `mul dst` | `dst: Register<Reg>` |  | Unsigned multiply: `rdx:rax = rax * src` (or the smaller registers for a smaller operand). |  |
| `mul dst` | `dst: Register<Reg32>` |  |  |  |
| `mul dst` | `dst: Register<Reg16>` |  |  |  |
| `mul dst` | `dst: Register<Reg8>` |  |  |  |
| `mul _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `imul`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `imul rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Signed multiply: `rdx:rax = rax * rd` (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `imul dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Signed multiply: `rdx:rax = rax * [dst]` (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |
| `imul rd, rs` | `<R>`, `rd: R`, `rs: R` | emits `Bytes<...>` | `rd = rd * rs`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul rd, src` | `<R>`, `rd: R`, `src: MemOperand` | emits `Bytes<...>` | `rd = rd * [src]`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul rd, rs, imm` | `<R>`, `rd: R`, `rs: R`, `imm: int` | emits `Bytes<...>` | `rd = rs * imm`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul rd, src, imm` | `<R>`, `rd: R`, `src: MemOperand`, `imm: int` | emits `Bytes<...>` | `rd = [src] * imm`, signed, keeping the low half. | [`std.x86_64.impl`](impl.md) |
| `imul src` | `src: Register<Reg>` |  | Signed multiply: `rdx:rax = rax * src` (or the smaller registers). |  |
| `imul src` | `src: Register<Reg32>` |  |  |  |
| `imul src` | `src: Register<Reg16>` |  |  |  |
| `imul src` | `src: Register<Reg8>` |  |  |  |
| `imul src, dst` | `src: Register<Reg>`, `dst: Register<Reg>` |  | `dst = dst * src`, signed. |  |
| `imul src, dst` | `src: Register<Reg32>`, `dst: Register<Reg32>` |  |  |  |
| `imul src, dst` | `src: Register<Reg16>`, `dst: Register<Reg16>` |  |  |  |
| `imul src, dst` | `<R>`, `src: Memory<MemOperand>`, `dst: Register<R>` |  | `dst = dst * src`, signed. |  |
| `imul imm, src, dst` | `<R>`, `imm: Imm`, `src: Register<R>`, `dst: Register<R>` |  | `dst = src * imm`, signed. |  |
| `imul imm, src, dst` | `<R>`, `imm: Imm`, `src: Memory<MemOperand>`, `dst: Register<R>` |  | `dst = [src] * imm`, signed. |  |

### `div`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `div rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Unsigned divide: `rax = rdx:rax / rd`, `rdx` = the remainder (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `div dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Unsigned divide: `rax = rdx:rax / [dst]`, `rdx` = the remainder (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |
| `div dst` | `dst: Register<Reg>` |  | Unsigned divide: `rax = rdx:rax / src`, `rdx` = the remainder. |  |
| `div dst` | `dst: Register<Reg32>` |  |  |  |
| `div dst` | `dst: Register<Reg16>` |  |  |  |
| `div dst` | `dst: Register<Reg8>` |  |  |  |
| `div _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `idiv`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `idiv rd` | `<R>`, `rd: R` | emits `Bytes<...>` | Signed divide: `rax = rdx:rax / rd`, `rdx` = the remainder (or the smaller registers for a smaller operand). | [`std.x86_64.impl`](impl.md) |
| `idiv dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | Signed divide: `rax = rdx:rax / [dst]`, `rdx` = the remainder (or the smaller registers for a smaller operand), `size` wide. | [`std.x86_64.impl`](impl.md) |
| `idiv dst` | `dst: Register<Reg>` |  | Signed divide: `rax = rdx:rax / src`, `rdx` = the remainder. |  |
| `idiv dst` | `dst: Register<Reg32>` |  |  |  |
| `idiv dst` | `dst: Register<Reg16>` |  |  |  |
| `idiv dst` | `dst: Register<Reg8>` |  |  |  |
| `idiv _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `inc`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `inc rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = rd + 1`, leaving CF alone. | [`std.x86_64.impl`](impl.md) |
| `inc dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] + 1`, leaving CF alone, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `inc dst` | `dst: Register<Reg>` |  | `dst = dst + 1`, leaving CF alone. |  |
| `inc dst` | `dst: Register<Reg32>` |  |  |  |
| `inc dst` | `dst: Register<Reg16>` |  |  |  |
| `inc dst` | `dst: Register<Reg8>` |  |  |  |
| `inc _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `dec`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dec rd` | `<R>`, `rd: R` | emits `Bytes<...>` | `rd = rd - 1`, leaving CF alone. | [`std.x86_64.impl`](impl.md) |
| `dec dst, size` | `dst: MemOperand`, `size: OperandSize` | emits `Bytes<...>` | `[dst] = [dst] - 1`, leaving CF alone, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `dec dst` | `dst: Register<Reg>` |  | `dst = dst - 1`, leaving CF alone. |  |
| `dec dst` | `dst: Register<Reg32>` |  |  |  |
| `dec dst` | `dst: Register<Reg16>` |  |  |  |
| `dec dst` | `dst: Register<Reg8>` |  |  |  |
| `dec _dst` | `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `rol`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rol rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` left by `imm`. | [`std.x86_64.impl`](impl.md) |
| `rol rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` left by `cl`. | [`std.x86_64.impl`](impl.md) |
| `rol dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` left by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rol dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` left by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rol count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | Rotates `dst` left by `count` bits. |  |
| `rol count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | Rotates `dst` left by `count` bits. `count` must be `%cl`. |  |
| `rol _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `rol _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `ror`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ror rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` right by `imm`. | [`std.x86_64.impl`](impl.md) |
| `ror rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` right by `cl`. | [`std.x86_64.impl`](impl.md) |
| `ror dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` right by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `ror dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` right by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `ror count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | Rotates `dst` right by `count` bits. |  |
| `ror count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | Rotates `dst` right by `count` bits. `count` must be `%cl`. |  |
| `ror _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `ror _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `rcl`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcl rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` and CF left by `imm`. | [`std.x86_64.impl`](impl.md) |
| `rcl rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` and CF left by `cl`. | [`std.x86_64.impl`](impl.md) |
| `rcl dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF left by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rcl dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF left by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rcl count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | Rotates `dst` and CF left by `count` bits. |  |
| `rcl count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | Rotates `dst` and CF left by `count` bits. `count` must be `%cl`. |  |
| `rcl _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `rcl _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

### `rcr`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcr rd, imm` | `<R>`, `rd: R`, `imm: int` | emits `Bytes<...>` | Rotates `rd` and CF right by `imm`. | [`std.x86_64.impl`](impl.md) |
| `rcr rd, count` | `<R>`, `rd: R`, `count: Reg8` | emits `Bytes<...>` | Rotates `rd` and CF right by `cl`. | [`std.x86_64.impl`](impl.md) |
| `rcr dst, imm, size` | `dst: MemOperand`, `imm: int`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF right by `imm`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rcr dst, count, size` | `dst: MemOperand`, `count: Reg8`, `size: OperandSize` | emits `Bytes<...>` | Rotates `[dst]` and CF right by `cl`, `size` wide. | [`std.x86_64.impl`](impl.md) |
| `rcr count, dst` | `<R>`, `count: Imm`, `dst: Register<R>` |  | Rotates `dst` and CF right by `count` bits. |  |
| `rcr count, dst` | `<R>`, `count: Register<Reg8>`, `dst: Register<R>` |  | Rotates `dst` and CF right by `count` bits. `count` must be `%cl`. |  |
| `rcr _count, _dst` | `_count: Imm`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |
| `rcr _count, _dst` | `_count: Register<Reg8>`, `_dst: Memory<MemOperand>` |  | Fails: a memory operand alone needs a size suffix, like `incq (%rdi)`. |  |

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
| `jmp_far src` | `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

### `call_far`

Calls the far pointer at `src`: a 64-bit offset, then a 16-bit segment
selector.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call_far src` | `src: MemOperand` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

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
| `interrupt n` | `n: int` | emits `Bytes<...>` |  | [`std.x86_64.impl`](impl.md) |

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

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rep` |  | emits `Bytes<...>` | Repeats the next string instruction `rcx` times. | [`std.x86_64.impl`](impl.md) |
| `rep instruction` | `instruction: macro()` |  | Repeats `instruction` `rcx` times: `rep stosb`. |  |

### `repe`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repe` |  | emits `Bytes<...>` | Repeats the next `cmps` or `scas` while equal, up to `rcx` times. | [`std.x86_64.impl`](impl.md) |
| `repe instruction` | `instruction: macro()` |  | Repeats `instruction` while equal, up to `rcx` times: `repe cmpsb`. |  |

### `repz`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repz` |  | emits `Bytes<...>` | `repe`, under another name. | [`std.x86_64.impl`](impl.md) |
| `repz instruction` | `instruction: macro()` |  | `repe`, under another name. |  |

### `repne`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repne` |  | emits `Bytes<...>` | Repeats the next `cmps` or `scas` while not equal, up to `rcx` times. | [`std.x86_64.impl`](impl.md) |
| `repne instruction` | `instruction: macro()` |  | Repeats `instruction` while not equal, up to `rcx` times: `repne scasb`. |  |

### `repnz`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `repnz` |  | emits `Bytes<...>` | `repne`, under another name. | [`std.x86_64.impl`](impl.md) |
| `repnz instruction` | `instruction: macro()` |  | `repne`, under another name. |  |

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

### `data_int`

`value` as `width` little-endian bytes, truncated the way NASM does.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `data_int(value, width)` | `value: int`, `width: int` | returns `Bytes<...>` |  | [`std.x86_64.data`](data.md) |

### `data_string`

A string literal's UTF-8 bytes, with no terminator.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `data_string(source)` | `<S>`, `source: S` | returns `Bytes<...>` |  | [`std.x86_64.data`](data.md) |

### `resb`

`count` zero bytes, as one value: room for variables, like NASM's
`resb`. In a section that takes no room in the file, like ELF's `.bss`,
a link stage leaves them out of the file.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `resb count` | `count: int` |  |  | [`std.x86_64.data`](data.md) |

### `db`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `db a` | `a: int` |  | Bytes. Each operand is an integer, stored in one byte, or a string literal, stored as its UTF-8 bytes with no terminator (add `, 0` yourself). Up to four operands; `db "Hello", 10` is the common case. | [`std.x86_64.data`](data.md) |
| `db a` | `<S>`, `a: S` |  |  | [`std.x86_64.data`](data.md) |
| `db a, b` | `<S>`, `a: S`, `b: int` |  |  | [`std.x86_64.data`](data.md) |
| `db a, b, c` | `<S>`, `a: S`, `b: int`, `c: int` |  |  | [`std.x86_64.data`](data.md) |
| `db a, b` | `a: int`, `b: int` |  |  | [`std.x86_64.data`](data.md) |
| `db a, b, c` | `a: int`, `b: int`, `c: int` |  |  | [`std.x86_64.data`](data.md) |
| `db a, b, c, d` | `a: int`, `b: int`, `c: int`, `d: int` |  |  | [`std.x86_64.data`](data.md) |

### `dw`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dw a` | `a: int` |  | 16-bit little-endian words, one or two per line. | [`std.x86_64.data`](data.md) |
| `dw a, b` | `a: int`, `b: int` |  |  | [`std.x86_64.data`](data.md) |

### `dd`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dd a` | `a: int` |  | 32-bit little-endian doublewords, one or two per line. | [`std.x86_64.data`](data.md) |
| `dd a, b` | `a: int`, `b: int` |  |  | [`std.x86_64.data`](data.md) |

### `dq`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dq a` | `a: int` |  | 64-bit little-endian quadwords, one or two per line. | [`std.x86_64.data`](data.md) |
| `dq a, b` | `a: int`, `b: int` |  |  | [`std.x86_64.data`](data.md) |

### `register`

`%r`: a register.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `%r` | `<R>`, `r: R` | returns `Register<R>` |  |

### `immediate`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `$value` | `value: int` | returns `Imm` | `$value`: an immediate. |
| `$_value` | `_value: label` | returns `Imm` | `$label` would be the label's absolute address, which isn't known until the program is loaded. Fails; use `lea label(%rip), %reg` instead. |

### `memory_base`

`(%base)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `(%base)` | `base: Reg` | returns `Memory<MemOperand>` |  |

### `memory_disp`

`disp(%base)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `disp(%base)` | `disp: int`, `base: Reg` | returns `Memory<MemOperand>` |  |

### `memory_indexed`

`(%base,%index)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `(%base,%index)` | `base: Reg`, `index: Reg` | returns `Memory<MemOperand>` |  |

### `memory_disp_indexed`

`disp(%base,%index)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `disp(%base,%index)` | `disp: int`, `base: Reg`, `index: Reg` | returns `Memory<MemOperand>` |  |

### `memory_scaled`

`(%base,%index,scale)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `(%base,%index,scale)` | `base: Reg`, `index: Reg`, `scale: int` | returns `Memory<MemOperand>` |  |

### `memory_disp_scaled`

`disp(%base,%index,scale)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `disp(%base,%index,scale)` | `disp: int`, `base: Reg`, `index: Reg`, `scale: int` | returns `Memory<MemOperand>` |  |

### `memory_rip`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `disp(%rip)` | `disp: int` | returns `Memory<MemOperand>` | `disp(%rip)`: `disp` bytes past the end of the instruction. |
| `disp(%rip)` | `disp: label` | returns `Memory<RipLabel>` | `label(%rip)`: `label`, addressed relative to the next instruction, so the code works wherever it's loaded. |

### `indirect`

`*target`: an indirect `jmp` or `call` through a register or memory.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `*through` | `<T>`, `through: T` | returns `Indirect<T>` |  |

### `movb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `mov` on byte operands. |
| `movb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `mov` on byte operands. |

### `movw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `mov` on word operands. |
| `movw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `mov` on word operands. |

### `movl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `mov` on long operands. |
| `movl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `mov` on long operands. |

### `movq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `mov` on quadword operands. |
| `movq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `mov` on quadword operands. |

### `addb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `addb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `add` on byte operands. |
| `addb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `add` on byte operands. |

### `addw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `addw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `add` on word operands. |
| `addw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `add` on word operands. |

### `addl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `addl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `add` on long operands. |
| `addl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `add` on long operands. |

### `addq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `addq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `add` on quadword operands. |
| `addq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `add` on quadword operands. |

### `adcb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adcb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `adc` on byte operands. |
| `adcb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `adc` on byte operands. |

### `adcw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adcw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `adc` on word operands. |
| `adcw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `adc` on word operands. |

### `adcl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adcl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `adc` on long operands. |
| `adcl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `adc` on long operands. |

### `adcq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adcq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `adc` on quadword operands. |
| `adcq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `adc` on quadword operands. |

### `subb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `subb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sub` on byte operands. |
| `subb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sub` on byte operands. |

### `subw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `subw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sub` on word operands. |
| `subw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sub` on word operands. |

### `subl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `subl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sub` on long operands. |
| `subl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sub` on long operands. |

### `subq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `subq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sub` on quadword operands. |
| `subq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sub` on quadword operands. |

### `sbbb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbbb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sbb` on byte operands. |
| `sbbb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sbb` on byte operands. |

### `sbbw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbbw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sbb` on word operands. |
| `sbbw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sbb` on word operands. |

### `sbbl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbbl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sbb` on long operands. |
| `sbbl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sbb` on long operands. |

### `sbbq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbbq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `sbb` on quadword operands. |
| `sbbq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `sbb` on quadword operands. |

### `andb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `andb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `and` on byte operands. |
| `andb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `and` on byte operands. |

### `andw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `andw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `and` on word operands. |
| `andw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `and` on word operands. |

### `andl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `andl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `and` on long operands. |
| `andl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `and` on long operands. |

### `andq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `andq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `and` on quadword operands. |
| `andq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `and` on quadword operands. |

### `orb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `orb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `or` on byte operands. |
| `orb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `or` on byte operands. |

### `orw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `orw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `or` on word operands. |
| `orw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `or` on word operands. |

### `orl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `orl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `or` on long operands. |
| `orl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `or` on long operands. |

### `orq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `orq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `or` on quadword operands. |
| `orq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `or` on quadword operands. |

### `xorb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xorb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `xor` on byte operands. |
| `xorb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `xor` on byte operands. |

### `xorw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xorw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `xor` on word operands. |
| `xorw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `xor` on word operands. |

### `xorl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xorl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `xor` on long operands. |
| `xorl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `xor` on long operands. |

### `xorq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `xorq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `xor` on quadword operands. |
| `xorq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `xor` on quadword operands. |

### `cmpb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `cmp` on byte operands. |
| `cmpb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `cmp` on byte operands. |

### `cmpw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `cmp` on word operands. |
| `cmpw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `cmp` on word operands. |

### `cmpl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `cmp` on long operands. |
| `cmpl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `cmp` on long operands. |

### `cmpq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `cmp` on quadword operands. |
| `cmpq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `cmp` on quadword operands. |

### `testb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `testb src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `test` on byte operands. |
| `testb src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `test` on byte operands. |

### `testw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `testw src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `test` on word operands. |
| `testw src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `test` on word operands. |

### `testl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `testl src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `test` on long operands. |
| `testl src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `test` on long operands. |

### `testq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `testq src, dst` | `<S, D>`, `src: S`, `dst: D` |  | `test` on quadword operands. |
| `testq src, dst` | `src: Imm`, `dst: Memory<MemOperand>` |  | `test` on quadword operands. |

### `incb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `incb dst` | `dst: Register<Reg8>` |  | `inc` on a byte. |
| `incb dst` | `<M>`, `dst: Memory<M>` |  | `inc` on a byte. |

### `incw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `incw dst` | `dst: Register<Reg16>` |  | `inc` on a word. |
| `incw dst` | `<M>`, `dst: Memory<M>` |  | `inc` on a word. |

### `incl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `incl dst` | `dst: Register<Reg32>` |  | `inc` on a long. |
| `incl dst` | `<M>`, `dst: Memory<M>` |  | `inc` on a long. |

### `incq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `incq dst` | `dst: Register<Reg>` |  | `inc` on a quadword. |
| `incq dst` | `<M>`, `dst: Memory<M>` |  | `inc` on a quadword. |

### `decb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decb dst` | `dst: Register<Reg8>` |  | `dec` on a byte. |
| `decb dst` | `<M>`, `dst: Memory<M>` |  | `dec` on a byte. |

### `decw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decw dst` | `dst: Register<Reg16>` |  | `dec` on a word. |
| `decw dst` | `<M>`, `dst: Memory<M>` |  | `dec` on a word. |

### `decl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decl dst` | `dst: Register<Reg32>` |  | `dec` on a long. |
| `decl dst` | `<M>`, `dst: Memory<M>` |  | `dec` on a long. |

### `decq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decq dst` | `dst: Register<Reg>` |  | `dec` on a quadword. |
| `decq dst` | `<M>`, `dst: Memory<M>` |  | `dec` on a quadword. |

### `negb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `negb dst` | `dst: Register<Reg8>` |  | `neg` on a byte. |
| `negb dst` | `<M>`, `dst: Memory<M>` |  | `neg` on a byte. |

### `negw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `negw dst` | `dst: Register<Reg16>` |  | `neg` on a word. |
| `negw dst` | `<M>`, `dst: Memory<M>` |  | `neg` on a word. |

### `negl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `negl dst` | `dst: Register<Reg32>` |  | `neg` on a long. |
| `negl dst` | `<M>`, `dst: Memory<M>` |  | `neg` on a long. |

### `negq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `negq dst` | `dst: Register<Reg>` |  | `neg` on a quadword. |
| `negq dst` | `<M>`, `dst: Memory<M>` |  | `neg` on a quadword. |

### `notb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `notb dst` | `dst: Register<Reg8>` |  | `not` on a byte. |
| `notb dst` | `<M>`, `dst: Memory<M>` |  | `not` on a byte. |

### `notw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `notw dst` | `dst: Register<Reg16>` |  | `not` on a word. |
| `notw dst` | `<M>`, `dst: Memory<M>` |  | `not` on a word. |

### `notl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `notl dst` | `dst: Register<Reg32>` |  | `not` on a long. |
| `notl dst` | `<M>`, `dst: Memory<M>` |  | `not` on a long. |

### `notq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `notq dst` | `dst: Register<Reg>` |  | `not` on a quadword. |
| `notq dst` | `<M>`, `dst: Memory<M>` |  | `not` on a quadword. |

### `mulb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mulb dst` | `dst: Register<Reg8>` |  | `mul` on a byte. |
| `mulb dst` | `<M>`, `dst: Memory<M>` |  | `mul` on a byte. |

### `mulw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mulw dst` | `dst: Register<Reg16>` |  | `mul` on a word. |
| `mulw dst` | `<M>`, `dst: Memory<M>` |  | `mul` on a word. |

### `mull`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mull dst` | `dst: Register<Reg32>` |  | `mul` on a long. |
| `mull dst` | `<M>`, `dst: Memory<M>` |  | `mul` on a long. |

### `mulq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mulq dst` | `dst: Register<Reg>` |  | `mul` on a quadword. |
| `mulq dst` | `<M>`, `dst: Memory<M>` |  | `mul` on a quadword. |

### `divb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `divb dst` | `dst: Register<Reg8>` |  | `div` on a byte. |
| `divb dst` | `<M>`, `dst: Memory<M>` |  | `div` on a byte. |

### `divw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `divw dst` | `dst: Register<Reg16>` |  | `div` on a word. |
| `divw dst` | `<M>`, `dst: Memory<M>` |  | `div` on a word. |

### `divl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `divl dst` | `dst: Register<Reg32>` |  | `div` on a long. |
| `divl dst` | `<M>`, `dst: Memory<M>` |  | `div` on a long. |

### `divq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `divq dst` | `dst: Register<Reg>` |  | `div` on a quadword. |
| `divq dst` | `<M>`, `dst: Memory<M>` |  | `div` on a quadword. |

### `idivb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `idivb dst` | `dst: Register<Reg8>` |  | `idiv` on a byte. |
| `idivb dst` | `<M>`, `dst: Memory<M>` |  | `idiv` on a byte. |

### `idivw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `idivw dst` | `dst: Register<Reg16>` |  | `idiv` on a word. |
| `idivw dst` | `<M>`, `dst: Memory<M>` |  | `idiv` on a word. |

### `idivl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `idivl dst` | `dst: Register<Reg32>` |  | `idiv` on a long. |
| `idivl dst` | `<M>`, `dst: Memory<M>` |  | `idiv` on a long. |

### `idivq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `idivq dst` | `dst: Register<Reg>` |  | `idiv` on a quadword. |
| `idivq dst` | `<M>`, `dst: Memory<M>` |  | `idiv` on a quadword. |

### `shlb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shlb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `shl` on a byte. |
| `shlb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `shl` on a byte. `count` must be `%cl`. |
| `shlb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shl` on a byte. |
| `shlb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shl` on a byte. `count` must be `%cl`. |

### `shlw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shlw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `shl` on a word. |
| `shlw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `shl` on a word. `count` must be `%cl`. |
| `shlw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shl` on a word. |
| `shlw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shl` on a word. `count` must be `%cl`. |

### `shll`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shll count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `shl` on a long. |
| `shll count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `shl` on a long. `count` must be `%cl`. |
| `shll count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shl` on a long. |
| `shll count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shl` on a long. `count` must be `%cl`. |

### `shlq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shlq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `shl` on a quadword. |
| `shlq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `shl` on a quadword. `count` must be `%cl`. |
| `shlq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shl` on a quadword. |
| `shlq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shl` on a quadword. `count` must be `%cl`. |

### `shrb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shrb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `shr` on a byte. |
| `shrb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `shr` on a byte. `count` must be `%cl`. |
| `shrb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shr` on a byte. |
| `shrb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shr` on a byte. `count` must be `%cl`. |

### `shrw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shrw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `shr` on a word. |
| `shrw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `shr` on a word. `count` must be `%cl`. |
| `shrw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shr` on a word. |
| `shrw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shr` on a word. `count` must be `%cl`. |

### `shrl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shrl count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `shr` on a long. |
| `shrl count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `shr` on a long. `count` must be `%cl`. |
| `shrl count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shr` on a long. |
| `shrl count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shr` on a long. `count` must be `%cl`. |

### `shrq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shrq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `shr` on a quadword. |
| `shrq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `shr` on a quadword. `count` must be `%cl`. |
| `shrq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `shr` on a quadword. |
| `shrq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `shr` on a quadword. `count` must be `%cl`. |

### `sarb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sarb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `sar` on a byte. |
| `sarb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `sar` on a byte. `count` must be `%cl`. |
| `sarb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `sar` on a byte. |
| `sarb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `sar` on a byte. `count` must be `%cl`. |

### `sarw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sarw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `sar` on a word. |
| `sarw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `sar` on a word. `count` must be `%cl`. |
| `sarw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `sar` on a word. |
| `sarw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `sar` on a word. `count` must be `%cl`. |

### `sarl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sarl count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `sar` on a long. |
| `sarl count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `sar` on a long. `count` must be `%cl`. |
| `sarl count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `sar` on a long. |
| `sarl count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `sar` on a long. `count` must be `%cl`. |

### `sarq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sarq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `sar` on a quadword. |
| `sarq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `sar` on a quadword. `count` must be `%cl`. |
| `sarq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `sar` on a quadword. |
| `sarq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `sar` on a quadword. `count` must be `%cl`. |

### `rolb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rolb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `rol` on a byte. |
| `rolb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `rol` on a byte. `count` must be `%cl`. |
| `rolb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rol` on a byte. |
| `rolb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rol` on a byte. `count` must be `%cl`. |

### `rolw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rolw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `rol` on a word. |
| `rolw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `rol` on a word. `count` must be `%cl`. |
| `rolw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rol` on a word. |
| `rolw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rol` on a word. `count` must be `%cl`. |

### `roll`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `roll count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `rol` on a long. |
| `roll count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `rol` on a long. `count` must be `%cl`. |
| `roll count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rol` on a long. |
| `roll count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rol` on a long. `count` must be `%cl`. |

### `rolq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rolq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `rol` on a quadword. |
| `rolq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `rol` on a quadword. `count` must be `%cl`. |
| `rolq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rol` on a quadword. |
| `rolq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rol` on a quadword. `count` must be `%cl`. |

### `rorb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rorb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `ror` on a byte. |
| `rorb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `ror` on a byte. `count` must be `%cl`. |
| `rorb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `ror` on a byte. |
| `rorb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `ror` on a byte. `count` must be `%cl`. |

### `rorw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rorw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `ror` on a word. |
| `rorw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `ror` on a word. `count` must be `%cl`. |
| `rorw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `ror` on a word. |
| `rorw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `ror` on a word. `count` must be `%cl`. |

### `rorl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rorl count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `ror` on a long. |
| `rorl count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `ror` on a long. `count` must be `%cl`. |
| `rorl count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `ror` on a long. |
| `rorl count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `ror` on a long. `count` must be `%cl`. |

### `rorq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rorq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `ror` on a quadword. |
| `rorq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `ror` on a quadword. `count` must be `%cl`. |
| `rorq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `ror` on a quadword. |
| `rorq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `ror` on a quadword. `count` must be `%cl`. |

### `rclb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rclb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `rcl` on a byte. |
| `rclb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `rcl` on a byte. `count` must be `%cl`. |
| `rclb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcl` on a byte. |
| `rclb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcl` on a byte. `count` must be `%cl`. |

### `rclw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rclw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `rcl` on a word. |
| `rclw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `rcl` on a word. `count` must be `%cl`. |
| `rclw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcl` on a word. |
| `rclw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcl` on a word. `count` must be `%cl`. |

### `rcll`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcll count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `rcl` on a long. |
| `rcll count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `rcl` on a long. `count` must be `%cl`. |
| `rcll count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcl` on a long. |
| `rcll count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcl` on a long. `count` must be `%cl`. |

### `rclq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rclq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `rcl` on a quadword. |
| `rclq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `rcl` on a quadword. `count` must be `%cl`. |
| `rclq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcl` on a quadword. |
| `rclq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcl` on a quadword. `count` must be `%cl`. |

### `rcrb`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcrb count, dst` | `count: Imm`, `dst: Register<Reg8>` |  | `rcr` on a byte. |
| `rcrb count, dst` | `count: Register<Reg8>`, `dst: Register<Reg8>` |  | `rcr` on a byte. `count` must be `%cl`. |
| `rcrb count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcr` on a byte. |
| `rcrb count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcr` on a byte. `count` must be `%cl`. |

### `rcrw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcrw count, dst` | `count: Imm`, `dst: Register<Reg16>` |  | `rcr` on a word. |
| `rcrw count, dst` | `count: Register<Reg8>`, `dst: Register<Reg16>` |  | `rcr` on a word. `count` must be `%cl`. |
| `rcrw count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcr` on a word. |
| `rcrw count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcr` on a word. `count` must be `%cl`. |

### `rcrl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcrl count, dst` | `count: Imm`, `dst: Register<Reg32>` |  | `rcr` on a long. |
| `rcrl count, dst` | `count: Register<Reg8>`, `dst: Register<Reg32>` |  | `rcr` on a long. `count` must be `%cl`. |
| `rcrl count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcr` on a long. |
| `rcrl count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcr` on a long. `count` must be `%cl`. |

### `rcrq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcrq count, dst` | `count: Imm`, `dst: Register<Reg>` |  | `rcr` on a quadword. |
| `rcrq count, dst` | `count: Register<Reg8>`, `dst: Register<Reg>` |  | `rcr` on a quadword. `count` must be `%cl`. |
| `rcrq count, dst` | `<M>`, `count: Imm`, `dst: Memory<M>` |  | `rcr` on a quadword. |
| `rcrq count, dst` | `<M>`, `count: Register<Reg8>`, `dst: Memory<M>` |  | `rcr` on a quadword. `count` must be `%cl`. |

### `movzbw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movzbw src, dst` | `src: Register<Reg8>`, `dst: Register<Reg16>` |  | `dst = src`, a byte zero-extended into a word. |
| `movzbw src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg16>` |  | `dst = src`, a byte zero-extended into a word. |

### `movzbl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movzbl src, dst` | `src: Register<Reg8>`, `dst: Register<Reg32>` |  | `dst = src`, a byte zero-extended into a long. |
| `movzbl src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg32>` |  | `dst = src`, a byte zero-extended into a long. |

### `movzbq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movzbq src, dst` | `src: Register<Reg8>`, `dst: Register<Reg>` |  | `dst = src`, a byte zero-extended into a quadword. |
| `movzbq src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg>` |  | `dst = src`, a byte zero-extended into a quadword. |

### `movzwl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movzwl src, dst` | `src: Register<Reg16>`, `dst: Register<Reg32>` |  | `dst = src`, a word zero-extended into a long. |
| `movzwl src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg32>` |  | `dst = src`, a word zero-extended into a long. |

### `movzwq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movzwq src, dst` | `src: Register<Reg16>`, `dst: Register<Reg>` |  | `dst = src`, a word zero-extended into a quadword. |
| `movzwq src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg>` |  | `dst = src`, a word zero-extended into a quadword. |

### `movsbw`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsbw src, dst` | `src: Register<Reg8>`, `dst: Register<Reg16>` |  | `dst = src`, a byte sign-extended into a word. |
| `movsbw src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg16>` |  | `dst = src`, a byte sign-extended into a word. |

### `movsbl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsbl src, dst` | `src: Register<Reg8>`, `dst: Register<Reg32>` |  | `dst = src`, a byte sign-extended into a long. |
| `movsbl src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg32>` |  | `dst = src`, a byte sign-extended into a long. |

### `movsbq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsbq src, dst` | `src: Register<Reg8>`, `dst: Register<Reg>` |  | `dst = src`, a byte sign-extended into a quadword. |
| `movsbq src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg>` |  | `dst = src`, a byte sign-extended into a quadword. |

### `movswl`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movswl src, dst` | `src: Register<Reg16>`, `dst: Register<Reg32>` |  | `dst = src`, a word sign-extended into a long. |
| `movswl src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg32>` |  | `dst = src`, a word sign-extended into a long. |

### `movswq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movswq src, dst` | `src: Register<Reg16>`, `dst: Register<Reg>` |  | `dst = src`, a word sign-extended into a quadword. |
| `movswq src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg>` |  | `dst = src`, a word sign-extended into a quadword. |

### `movslq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movslq src, dst` | `src: Register<Reg32>`, `dst: Register<Reg>` |  | `dst = src`, a long sign-extended into a quadword. |
| `movslq src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg>` |  | `dst = src`, a long sign-extended into a quadword. |

### `leaq`

`lea` into a quadword register.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `leaq src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg>` |  |  |

### `leal`

`lea` into a long register.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `leal src, dst` | `<M>`, `src: Memory<M>`, `dst: Register<Reg32>` |  |  |

### `pushq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pushq src` | `src: Register<Reg>` |  | Pushes `src`. |
| `pushq src` | `src: Imm` |  | Pushes `src`, sign-extended to 64 bits. |
| `pushq src` | `src: Memory<MemOperand>` |  | Pushes the quadword at `src`. |

### `popq`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `popq dst` | `dst: Register<Reg>` |  | Pops into `dst`. |
| `popq dst` | `dst: Memory<MemOperand>` |  | Pops a quadword into `dst`. |

### `stosl`

`stosd`, under its AT&T name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `stosl` |  |  |  |

### `lodsl`

`lodsd`, under its AT&T name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lodsl` |  |  |  |

### `movsl`

`movsd` (the string move), under its AT&T name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movsl` |  |  |  |

### `cmpsl`

`cmpsd` (the string compare), under its AT&T name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cmpsl` |  |  |  |

### `scasl`

`scasd`, under its AT&T name.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `scasl` |  |  |  |

## Types

### `Register`

`struct Register<R>`

A register operand, `%rax`.

| Field | Type | Description |
|---|---|---|
| `r` | `R` | The register. |

### `Imm`

`struct Imm`

An immediate operand, `$1`.

| Field | Type | Description |
|---|---|---|
| `value` | `int` | The value. |

### `Memory`

`struct Memory<M>`

A memory operand. `at` is a `MemOperand`, or a `RipLabel` for
`label(%rip)`.

| Field | Type | Description |
|---|---|---|
| `at` | `M` | Where it is. |

### `Indirect`

`struct Indirect<T>`

The target of an indirect `jmp` or `call`: `*%rax`, or `*8(%rsp)`.

| Field | Type | Description |
|---|---|---|
| `through` | `T` | The register or memory holding the target. |

## Re-exported

From [`std.x86_64.impl`](impl.md): `Reg`, `r0`, `r1`, `r2`, `r3`, `r4`, `r5`, `r6`, `r7`, `r8`, `r9`, `r10`, `r11`, `r12`, `r13`, `r14`, `r15`, `rax`, `rcx`, `rdx`, `rbx`, `rsp`, `rbp`, `rsi`, `rdi`, `Reg32`, `eax`, `ecx`, `edx`, `ebx`, `esp`, `ebp`, `esi`, `edi`, `r8d`, `r9d`, `r10d`, `r11d`, `r12d`, `r13d`, `r14d`, `r15d`, `Reg16`, `ax`, `cx`, `dx`, `bx`, `sp`, `bp`, `si`, `di`, `r8w`, `r9w`, `r10w`, `r11w`, `r12w`, `r13w`, `r14w`, `r15w`, `Reg8`, `al`, `cl`, `dl`, `bl`, `spl`, `bpl`, `sil`, `dil`, `r8b`, `r9b`, `r10b`, `r11b`, `r12b`, `r13b`, `r14b`, `r15b`, `ah`, `ch`, `dh`, `bh`, `SReg`, `es`, `cs`, `ss`, `ds`, `fs`, `gs`, `Gpr`, `Byte`, `Bytes`, `OperandSize`, `byte`, `word`, `dword`, `qword`, `oword`, `MemBase`, `MemSib`, `MemRip`, `MemOperand`, `Rel32Instr`, `Rel32Instr2`, `RipLabel`, `RipRelInstr`, `RipRelInstrRex`, `Rel8Instr`, `Rel8Instr2`.
