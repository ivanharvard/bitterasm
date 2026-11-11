# `native`

[`std`](../index.md) › [`avr`](index.md) › `native`

AVR in its standard assembly syntax, the one GNU `as` and Atmel's
assembler read: `ld r16, X+`, `st -Y, r0`, `ldd r16, Z+5`,
`in r16, 0x3f`.

```basm
from std.avr.native import *

ld r16, X+
st -Y, r16
ldd r17, Z+5
in r18, 0x3f
```

```bytes
0d 91
0a 93
15 81
2f b7
```

The instructions themselves, and their registers, come from
`std.avr.impl`, where `in` is spelled `in_`, since `in` is a keyword.

Re-exports [`std.avr.impl`](impl.md).

## Macros

### `incremented`

`p` incremented after the access: `X` becomes `X+`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `incremented(p)` | `p: Pointer` | returns `Pointer` |  | [`std.avr.impl`](impl.md) |

### `decremented`

`p` decremented before the access: `X` becomes `-X`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `decremented(p)` | `p: Pointer` | returns `Pointer` |  | [`std.avr.impl`](impl.md) |

### `flash_address`

Where `l` is in flash, in bytes: its offset from the start of the
output.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `flash_address(l)` | `l: label` | returns `Deferred` |  | [`std.avr.impl`](impl.md) |

### `pm`

`l`'s word address in flash, what `ijmp` and `icall` jump to.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pm(l)` | `l: label` | returns `Deferred` |  | [`std.avr.impl`](impl.md) |

### `lo8`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lo8(x)` | `x: int` | returns `int` | The low byte of `x`. | [`std.avr.impl`](impl.md) |
| `lo8(l)` | `l: label` | returns `Deferred` | The low byte of `l`'s byte address in flash, for loading `Z` before `lpm`. | [`std.avr.impl`](impl.md) |
| `lo8(x)` | `x: Deferred` | returns `Deferred` | The low byte of a value `bitter` works out later. | [`std.avr.impl`](impl.md) |

### `hi8`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `hi8(x)` | `x: int` | returns `int` | The second byte of `x`. | [`std.avr.impl`](impl.md) |
| `hi8(l)` | `l: label` | returns `Deferred` | The second byte of `l`'s byte address in flash. | [`std.avr.impl`](impl.md) |
| `hi8(x)` | `x: Deferred` | returns `Deferred` | The second byte of a value `bitter` works out later. | [`std.avr.impl`](impl.md) |

### `pm_lo8`

The low byte of `l`'s word address, for loading `Z` before `ijmp`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pm_lo8(l)` | `l: label` | returns `Deferred` |  | [`std.avr.impl`](impl.md) |

### `pm_hi8`

The second byte of `l`'s word address.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pm_hi8(l)` | `l: label` | returns `Deferred` |  | [`std.avr.impl`](impl.md) |

### `add`

`rd += rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `add rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `adc`

`rd += rr + C`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `adc rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `sub`

`rd -= rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sub rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `sbc`

`rd -= rr + C`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbc rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `and`

`rd &= rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `and rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `or`

`rd |= rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `or rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `eor`

`rd ^= rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `eor rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `subi`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `subi rd, k` | `rd: Reg`, `k: int` |  | `rd -= k`, for r16 to r31. | [`std.avr.impl`](impl.md) |
| `subi rd, k` | `rd: Reg`, `k: Deferred` |  | `rd -= k`, with `k` worked out later, like `lo8(-table)`. | [`std.avr.impl`](impl.md) |

### `sbci`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbci rd, k` | `rd: Reg`, `k: int` |  | `rd -= k + C`, for r16 to r31. | [`std.avr.impl`](impl.md) |
| `sbci rd, k` | `rd: Reg`, `k: Deferred` |  | `rd -= k + C`, with `k` worked out later. | [`std.avr.impl`](impl.md) |

### `andi`

`rd &= k`, for r16 to r31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `andi rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `ori`

`rd |= k`, for r16 to r31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ori rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `sbr`

Sets the bits of `k` in `rd`: `ori`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbr rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `cbr`

Clears the bits of `k` in `rd`: `andi` with `k`'s complement.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cbr rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `adiw`

Adds `k`, 0 to 63, to a register pair: r24, r26, r28 or r30 and the
one above it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `adiw rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `sbiw`

Subtracts `k`, 0 to 63, from a register pair: r24, r26, r28 or r30 and
the one above it.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbiw rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `com`

`rd = ~rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `com rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `neg`

`rd = -rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `neg rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `inc`

`rd += 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `inc rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `dec`

`rd -= 1`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dec rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `tst`

Sets the flags from `rd`: `and rd, rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `tst rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `clr`

`rd = 0`: `eor rd, rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clr rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `ser`

`rd = 0xFF`, for r16 to r31: `ldi rd, 0xFF`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ser rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `mul`

`r1:r0 = rd * rr`, unsigned.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mul rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `muls`

`r1:r0 = rd * rr`, signed, for r16 to r31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `muls rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `mulsu`

`r1:r0 = rd * rr`, `rd` signed and `rr` unsigned, for r16 to r23.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mulsu rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `fmul`

Fractional unsigned multiply, for r16 to r23.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `fmul rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `fmuls`

Fractional signed multiply, for r16 to r23.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `fmuls rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `fmulsu`

Fractional multiply, `rd` signed and `rr` unsigned, for r16 to r23.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `fmulsu rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `lsl`

Shifts `rd` left one bit: `add rd, rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lsl rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `lsr`

Shifts `rd` right one bit.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lsr rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `rol`

Rotates `rd` left through carry: `adc rd, rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rol rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `ror`

Rotates `rd` right through carry.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ror rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `asr`

Shifts `rd` right one bit, keeping its sign.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `asr rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `swap`

Swaps `rd`'s two nibbles.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `swap rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `bset`

Sets SREG bit `s`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bset s` | `s: int` |  |  | [`std.avr.impl`](impl.md) |

### `bclr`

Clears SREG bit `s`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bclr s` | `s: int` |  |  | [`std.avr.impl`](impl.md) |

### `bst`

Stores bit `b` of `rd` in the `T` flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bst rd, b` | `rd: Reg`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `bld`

Loads the `T` flag into bit `b` of `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `bld rd, b` | `rd: Reg`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `sbi`

Sets bit `b` of I/O register `a`, 0 to 31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbi a, b` | `a: int`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `cbi`

Clears bit `b` of I/O register `a`, 0 to 31.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cbi a, b` | `a: int`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `sec`

Sets carry.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sec` |  |  |  | [`std.avr.impl`](impl.md) |

### `clc`

Clears carry.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clc` |  |  |  | [`std.avr.impl`](impl.md) |

### `sez`

Sets the zero flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sez` |  |  |  | [`std.avr.impl`](impl.md) |

### `clz`

Clears the zero flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clz` |  |  |  | [`std.avr.impl`](impl.md) |

### `sen`

Sets the negative flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sen` |  |  |  | [`std.avr.impl`](impl.md) |

### `cln`

Clears the negative flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cln` |  |  |  | [`std.avr.impl`](impl.md) |

### `sev`

Sets the overflow flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sev` |  |  |  | [`std.avr.impl`](impl.md) |

### `clv`

Clears the overflow flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clv` |  |  |  | [`std.avr.impl`](impl.md) |

### `ses`

Sets the sign flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ses` |  |  |  | [`std.avr.impl`](impl.md) |

### `cls`

Clears the sign flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cls` |  |  |  | [`std.avr.impl`](impl.md) |

### `seh`

Sets the half carry flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `seh` |  |  |  | [`std.avr.impl`](impl.md) |

### `clh`

Clears the half carry flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clh` |  |  |  | [`std.avr.impl`](impl.md) |

### `set`

Sets the `T` flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `set` |  |  |  | [`std.avr.impl`](impl.md) |

### `clt`

Clears the `T` flag.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `clt` |  |  |  | [`std.avr.impl`](impl.md) |

### `sei`

Enables interrupts.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sei` |  |  |  | [`std.avr.impl`](impl.md) |

### `cli`

Disables interrupts.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cli` |  |  |  | [`std.avr.impl`](impl.md) |

### `rjmp`

Jumps to `target`, up to 2048 words away.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rjmp target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `rcall`

Calls `target`, up to 2048 words away.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `rcall target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `jmp`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `jmp target` | `target: label` |  | Jumps to `target`, anywhere in flash. | [`std.avr.impl`](impl.md) |
| `jmp address` | `address: int` |  | Jumps to byte address `address` in flash, which must be even. | [`std.avr.impl`](impl.md) |

### `call`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `call target` | `target: label` |  | Calls `target`, anywhere in flash. | [`std.avr.impl`](impl.md) |
| `call address` | `address: int` |  | Calls byte address `address` in flash, which must be even. | [`std.avr.impl`](impl.md) |

### `ijmp`

Jumps to the word address in `Z`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ijmp` |  |  |  | [`std.avr.impl`](impl.md) |

### `icall`

Calls the word address in `Z`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `icall` |  |  |  | [`std.avr.impl`](impl.md) |

### `ret`

Returns from a call.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ret` |  |  |  | [`std.avr.impl`](impl.md) |

### `reti`

Returns from an interrupt, enabling interrupts.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `reti` |  |  |  | [`std.avr.impl`](impl.md) |

### `brbs`

Branches to `target`, up to 64 words away, if SREG bit `s` is set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brbs s, target` | `s: int`, `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brbc`

Branches to `target`, up to 64 words away, if SREG bit `s` is clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brbc s, target` | `s: int`, `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `breq`

Branches if equal: `Z` set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `breq target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brne`

Branches if not equal: `Z` clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brne target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brcs`

Branches if carry is set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brcs target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brcc`

Branches if carry is clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brcc target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brlo`

Branches if lower, unsigned: carry set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brlo target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brsh`

Branches if the same or higher, unsigned: carry clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brsh target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brmi`

Branches if negative.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brmi target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brpl`

Branches if not negative.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brpl target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brlt`

Branches if less than, signed: `S` set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brlt target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brge`

Branches if greater or equal, signed: `S` clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brge target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brhs`

Branches if half carry is set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brhs target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brhc`

Branches if half carry is clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brhc target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brts`

Branches if the `T` flag is set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brts target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brtc`

Branches if the `T` flag is clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brtc target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brvs`

Branches if overflow is set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brvs target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brvc`

Branches if overflow is clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brvc target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brie`

Branches if interrupts are enabled.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brie target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `brid`

Branches if interrupts are disabled.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `brid target` | `target: label` |  |  | [`std.avr.impl`](impl.md) |

### `cp`

Compares `rd` with `rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cp rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `cpc`

Compares `rd` with `rr` and carry.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cpc rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `cpi`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cpi rd, k` | `rd: Reg`, `k: int` |  | Compares `rd` with `k`, for r16 to r31. | [`std.avr.impl`](impl.md) |
| `cpi rd, k` | `rd: Reg`, `k: Deferred` |  | Compares `rd` with `k` worked out later. | [`std.avr.impl`](impl.md) |

### `cpse`

Skips the next instruction if `rd` equals `rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `cpse rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `sbrc`

Skips the next instruction if bit `b` of `rr` is clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbrc rr, b` | `rr: Reg`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `sbrs`

Skips the next instruction if bit `b` of `rr` is set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbrs rr, b` | `rr: Reg`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `sbic`

Skips the next instruction if bit `b` of I/O register `a`, 0 to 31, is
clear.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbic a, b` | `a: int`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `sbis`

Skips the next instruction if bit `b` of I/O register `a`, 0 to 31, is
set.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sbis a, b` | `a: int`, `b: int` |  |  | [`std.avr.impl`](impl.md) |

### `mov`

`rd = rr`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `mov rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `movw`

Copies the pair starting at `rr` to the pair starting at `rd`. Both
are even.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `movw rd, rr` | `rd: Reg`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `ldi`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ldi rd, k` | `rd: Reg`, `k: int` |  | `rd = k`, for r16 to r31. | [`std.avr.impl`](impl.md) |
| `ldi rd, k` | `rd: Reg`, `k: Deferred` |  | `rd = k`, with `k` worked out later: `ldi r30, lo8(table)`. | [`std.avr.impl`](impl.md) |

### `ld`

Loads `rd` from the data address in `p`: `X`, `Y`, `Z`, or one of them
incremented after or decremented before.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ld rd, p` | `rd: Reg`, `p: Pointer` |  |  | [`std.avr.impl`](impl.md) |

### `ldd`

Loads `rd` from `q` bytes, 0 to 63, past the address in `Y` or `Z`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `ldd rd, p+q` | `rd: Reg`, `p: Pointer`, `q: int` |  |  | [`std.avr.impl`](impl.md) |

### `st`

Stores `rr` at the data address in `p`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `st p, rr` | `p: Pointer`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `std`

Stores `rr` `q` bytes, 0 to 63, past the address in `Y` or `Z`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `std p+q, rr` | `p: Pointer`, `q: int`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `lds`

Loads `rd` from data address `k`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lds rd, k` | `rd: Reg`, `k: int` |  |  | [`std.avr.impl`](impl.md) |

### `sts`

Stores `rr` at data address `k`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sts k, rr` | `k: int`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `lpm`

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `lpm` |  |  | Loads r0 from the flash byte address in `Z`. | [`std.avr.impl`](impl.md) |
| `lpm rd, p` | `rd: Reg`, `p: Pointer` |  | Loads `rd` from the flash byte address in `p`: `Z`, or `Z` incremented after. | [`std.avr.impl`](impl.md) |

### `spm`

Writes flash from r1:r0 at the address in `Z`, under the rules of the
chip's self-programming mode.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `spm` |  |  |  | [`std.avr.impl`](impl.md) |

### `in_`

Reads I/O register `a`, 0 to 63, into `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `in_ rd, a` | `rd: Reg`, `a: int` |  |  | [`std.avr.impl`](impl.md) |

### `out`

Writes `rr` to I/O register `a`, 0 to 63.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `out a, rr` | `a: int`, `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `push`

Pushes `rr` onto the stack.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `push rr` | `rr: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `pop`

Pops the stack into `rd`.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `pop rd` | `rd: Reg` |  |  | [`std.avr.impl`](impl.md) |

### `nop`

Does nothing for one cycle.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `nop` |  |  |  | [`std.avr.impl`](impl.md) |

### `sleep`

Enters the sleep mode set in the chip's control register.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `sleep` |  |  |  | [`std.avr.impl`](impl.md) |

### `wdr`

Resets the watchdog timer.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `wdr` |  |  |  | [`std.avr.impl`](impl.md) |

### `break`

Stops for an on-chip debugger.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `break` |  |  |  | [`std.avr.impl`](impl.md) |

### `db`

One byte of data in flash.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `db value` | `value: int` |  |  | [`std.avr.impl`](impl.md) |

### `dw`

One 16-bit word of data in flash, least significant byte first.

| Syntax | Parameters | Result | Description | From |
|---|---|---|---|---|
| `dw value` | `value: int` |  |  | [`std.avr.impl`](impl.md) |

### `pre_decrement_x`

`-X`: `X`, decremented before the access.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `-X` |  | returns `Pointer` |  |

### `pre_decrement_y`

`-Y`: `Y`, decremented before the access.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `-Y` |  | returns `Pointer` |  |

### `pre_decrement_z`

`-Z`: `Z`, decremented before the access.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `-Z` |  | returns `Pointer` |  |

### `ld_incremented`

`ld rd, X+`: loads, then increments the pointer.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ld rd, p+` | `rd: Reg`, `p: Pointer` |  |  |

### `st_incremented`

`st X+, rr`: stores, then increments the pointer.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `st p+, rr` | `p: Pointer`, `rr: Reg` |  |  |

### `lpm_incremented`

`lpm rd, Z+`: loads from flash, then increments `Z`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lpm rd, p+` | `rd: Reg`, `p: Pointer` |  |  |

### `in_native`

`in rd, a`: reads I/O register `a`, 0 to 63, into `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `in rd, a` | `rd: Reg`, `a: int` |  |  |

## Re-exported

From [`std.avr.impl`](impl.md): `Reg`, `r0`, `r1`, `r2`, `r3`, `r4`, `r5`, `r6`, `r7`, `r8`, `r9`, `r10`, `r11`, `r12`, `r13`, `r14`, `r15`, `r16`, `r17`, `r18`, `r19`, `r20`, `r21`, `r22`, `r23`, `r24`, `r25`, `r26`, `r27`, `r28`, `r29`, `r30`, `r31`, `Pointer`, `X`, `X_INC`, `X_DEC`, `Y`, `Y_INC`, `Y_DEC`, `Z`, `Z_INC`, `Z_DEC`, `FLAG_C`, `FLAG_Z`, `FLAG_N`, `FLAG_V`, `FLAG_S`, `FLAG_H`, `FLAG_T`, `FLAG_I`, `Word`, `LongWord`, `RelWord`, `BranchWord`, `ImmWord`, `JumpWords`, `JumpHigh`.
