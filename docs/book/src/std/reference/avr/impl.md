# `impl`

[`std`](../index.md) › [`avr`](index.md) › `impl`

AVR: the 8-bit microcontrollers of the ATmega and ATtiny families. Their
32 registers, and every instruction of the ATmega328P (the `avr5`
family), each emitted as one or two little-endian 16-bit words.

Coverage: 131 of 131 operand forms (100%), through `std.avr.native`.

Import `std.avr.native` for the standard assembly syntax, which spells
pointer operands `X+`, `-Y` and `Z+5`. Here every instruction takes its
operands in order, and those are names: `ld r16, X_INC`,
`ldd r16, Y, 5`.

```basm
from std.avr.impl import *

    ldi r16, 10
loop:
    dec r16
    brne loop
```

```bytes
0a e0
0a 95
f1 f7
```

Flash holds 16-bit words, and every program address an instruction
takes is a word address: a branch's distance, a `jmp`'s target, `pm(l)`
for `ijmp`. Data memory is addressed in bytes. Branch and jump targets
are labels; `bitter` works out the distance once the program is laid
out. A label's address is its byte offset from the start of the output,
which is where it is in flash when the output is written to flash from
address 0.

Immediates are plain `int`s. One that doesn't fit its field is a compile
error, except that a negative 8-bit immediate becomes its two's
complement, as in `subi r16, -1`. A branch whose target is out of range
isn't caught: its distance wraps.

## Macros

### `incremented`

`p` incremented after the access: `X` becomes `X+`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `incremented(p)` | `p: Pointer` | returns `Pointer` |  |

### `decremented`

`p` decremented before the access: `X` becomes `-X`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `decremented(p)` | `p: Pointer` | returns `Pointer` |  |

### `flash_address`

Where `l` is in flash, in bytes: its offset from the start of the
output.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `flash_address(l)` | `l: label` | returns `Deferred` |  |

### `pm`

`l`'s word address in flash, what `ijmp` and `icall` jump to.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pm(l)` | `l: label` | returns `Deferred` |  |

### `lo8`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lo8(x)` | `x: int` | returns `int` | The low byte of `x`. |
| `lo8(l)` | `l: label` | returns `Deferred` | The low byte of `l`'s byte address in flash, for loading `Z` before `lpm`. |
| `lo8(x)` | `x: Deferred` | returns `Deferred` | The low byte of a value `bitter` works out later. |

### `hi8`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `hi8(x)` | `x: int` | returns `int` | The second byte of `x`. |
| `hi8(l)` | `l: label` | returns `Deferred` | The second byte of `l`'s byte address in flash. |
| `hi8(x)` | `x: Deferred` | returns `Deferred` | The second byte of a value `bitter` works out later. |

### `pm_lo8`

The low byte of `l`'s word address, for loading `Z` before `ijmp`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pm_lo8(l)` | `l: label` | returns `Deferred` |  |

### `pm_hi8`

The second byte of `l`'s word address.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pm_hi8(l)` | `l: label` | returns `Deferred` |  |

### `add`

`rd += rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `add rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `adc`

`rd += rr + C`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adc rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `sub`

`rd -= rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sub rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `sbc`

`rd -= rr + C`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbc rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `and`

`rd &= rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `and rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `or`

`rd |= rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `or rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `eor`

`rd ^= rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `eor rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `subi`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `subi rd, k` | `rd: Reg`, `k: int` |  | `rd -= k`, for r16 to r31. |
| `subi rd, k` | `rd: Reg`, `k: Deferred` |  | `rd -= k`, with `k` worked out later, like `lo8(-table)`. |

### `sbci`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbci rd, k` | `rd: Reg`, `k: int` |  | `rd -= k + C`, for r16 to r31. |
| `sbci rd, k` | `rd: Reg`, `k: Deferred` |  | `rd -= k + C`, with `k` worked out later. |

### `andi`

`rd &= k`, for r16 to r31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `andi rd, k` | `rd: Reg`, `k: int` |  |  |

### `ori`

`rd |= k`, for r16 to r31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ori rd, k` | `rd: Reg`, `k: int` |  |  |

### `sbr`

Sets the bits of `k` in `rd`: `ori`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbr rd, k` | `rd: Reg`, `k: int` |  |  |

### `cbr`

Clears the bits of `k` in `rd`: `andi` with `k`'s complement.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cbr rd, k` | `rd: Reg`, `k: int` |  |  |

### `adiw`

Adds `k`, 0 to 63, to a register pair: r24, r26, r28 or r30 and the
one above it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `adiw rd, k` | `rd: Reg`, `k: int` |  |  |

### `sbiw`

Subtracts `k`, 0 to 63, from a register pair: r24, r26, r28 or r30 and
the one above it.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbiw rd, k` | `rd: Reg`, `k: int` |  |  |

### `com`

`rd = ~rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `com rd` | `rd: Reg` |  |  |

### `neg`

`rd = -rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `neg rd` | `rd: Reg` |  |  |

### `inc`

`rd += 1`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `inc rd` | `rd: Reg` |  |  |

### `dec`

`rd -= 1`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dec rd` | `rd: Reg` |  |  |

### `tst`

Sets the flags from `rd`: `and rd, rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `tst rd` | `rd: Reg` |  |  |

### `clr`

`rd = 0`: `eor rd, rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clr rd` | `rd: Reg` |  |  |

### `ser`

`rd = 0xFF`, for r16 to r31: `ldi rd, 0xFF`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ser rd` | `rd: Reg` |  |  |

### `mul`

`r1:r0 = rd * rr`, unsigned.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mul rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `muls`

`r1:r0 = rd * rr`, signed, for r16 to r31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `muls rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `mulsu`

`r1:r0 = rd * rr`, `rd` signed and `rr` unsigned, for r16 to r23.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mulsu rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `fmul`

Fractional unsigned multiply, for r16 to r23.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fmul rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `fmuls`

Fractional signed multiply, for r16 to r23.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fmuls rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `fmulsu`

Fractional multiply, `rd` signed and `rr` unsigned, for r16 to r23.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `fmulsu rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `lsl`

Shifts `rd` left one bit: `add rd, rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lsl rd` | `rd: Reg` |  |  |

### `lsr`

Shifts `rd` right one bit.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lsr rd` | `rd: Reg` |  |  |

### `rol`

Rotates `rd` left through carry: `adc rd, rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rol rd` | `rd: Reg` |  |  |

### `ror`

Rotates `rd` right through carry.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ror rd` | `rd: Reg` |  |  |

### `asr`

Shifts `rd` right one bit, keeping its sign.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `asr rd` | `rd: Reg` |  |  |

### `swap`

Swaps `rd`'s two nibbles.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `swap rd` | `rd: Reg` |  |  |

### `bset`

Sets SREG bit `s`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bset s` | `s: int` |  |  |

### `bclr`

Clears SREG bit `s`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bclr s` | `s: int` |  |  |

### `bst`

Stores bit `b` of `rd` in the `T` flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bst rd, b` | `rd: Reg`, `b: int` |  |  |

### `bld`

Loads the `T` flag into bit `b` of `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `bld rd, b` | `rd: Reg`, `b: int` |  |  |

### `sbi`

Sets bit `b` of I/O register `a`, 0 to 31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbi a, b` | `a: int`, `b: int` |  |  |

### `cbi`

Clears bit `b` of I/O register `a`, 0 to 31.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cbi a, b` | `a: int`, `b: int` |  |  |

### `sec`

Sets carry.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sec` |  |  |  |

### `clc`

Clears carry.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clc` |  |  |  |

### `sez`

Sets the zero flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sez` |  |  |  |

### `clz`

Clears the zero flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clz` |  |  |  |

### `sen`

Sets the negative flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sen` |  |  |  |

### `cln`

Clears the negative flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cln` |  |  |  |

### `sev`

Sets the overflow flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sev` |  |  |  |

### `clv`

Clears the overflow flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clv` |  |  |  |

### `ses`

Sets the sign flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ses` |  |  |  |

### `cls`

Clears the sign flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cls` |  |  |  |

### `seh`

Sets the half carry flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `seh` |  |  |  |

### `clh`

Clears the half carry flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clh` |  |  |  |

### `set`

Sets the `T` flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `set` |  |  |  |

### `clt`

Clears the `T` flag.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `clt` |  |  |  |

### `sei`

Enables interrupts.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sei` |  |  |  |

### `cli`

Disables interrupts.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cli` |  |  |  |

### `rjmp`

Jumps to `target`, up to 2048 words away.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rjmp target` | `target: label` |  |  |

### `rcall`

Calls `target`, up to 2048 words away.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `rcall target` | `target: label` |  |  |

### `jmp`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `jmp target` | `target: label` |  | Jumps to `target`, anywhere in flash. |
| `jmp address` | `address: int` |  | Jumps to byte address `address` in flash, which must be even. |

### `call`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `call target` | `target: label` |  | Calls `target`, anywhere in flash. |
| `call address` | `address: int` |  | Calls byte address `address` in flash, which must be even. |

### `ijmp`

Jumps to the word address in `Z`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ijmp` |  |  |  |

### `icall`

Calls the word address in `Z`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `icall` |  |  |  |

### `ret`

Returns from a call.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ret` |  |  |  |

### `reti`

Returns from an interrupt, enabling interrupts.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `reti` |  |  |  |

### `brbs`

Branches to `target`, up to 64 words away, if SREG bit `s` is set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brbs s, target` | `s: int`, `target: label` |  |  |

### `brbc`

Branches to `target`, up to 64 words away, if SREG bit `s` is clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brbc s, target` | `s: int`, `target: label` |  |  |

### `breq`

Branches if equal: `Z` set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `breq target` | `target: label` |  |  |

### `brne`

Branches if not equal: `Z` clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brne target` | `target: label` |  |  |

### `brcs`

Branches if carry is set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brcs target` | `target: label` |  |  |

### `brcc`

Branches if carry is clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brcc target` | `target: label` |  |  |

### `brlo`

Branches if lower, unsigned: carry set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brlo target` | `target: label` |  |  |

### `brsh`

Branches if the same or higher, unsigned: carry clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brsh target` | `target: label` |  |  |

### `brmi`

Branches if negative.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brmi target` | `target: label` |  |  |

### `brpl`

Branches if not negative.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brpl target` | `target: label` |  |  |

### `brlt`

Branches if less than, signed: `S` set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brlt target` | `target: label` |  |  |

### `brge`

Branches if greater or equal, signed: `S` clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brge target` | `target: label` |  |  |

### `brhs`

Branches if half carry is set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brhs target` | `target: label` |  |  |

### `brhc`

Branches if half carry is clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brhc target` | `target: label` |  |  |

### `brts`

Branches if the `T` flag is set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brts target` | `target: label` |  |  |

### `brtc`

Branches if the `T` flag is clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brtc target` | `target: label` |  |  |

### `brvs`

Branches if overflow is set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brvs target` | `target: label` |  |  |

### `brvc`

Branches if overflow is clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brvc target` | `target: label` |  |  |

### `brie`

Branches if interrupts are enabled.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brie target` | `target: label` |  |  |

### `brid`

Branches if interrupts are disabled.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `brid target` | `target: label` |  |  |

### `cp`

Compares `rd` with `rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cp rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `cpc`

Compares `rd` with `rr` and carry.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cpc rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `cpi`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cpi rd, k` | `rd: Reg`, `k: int` |  | Compares `rd` with `k`, for r16 to r31. |
| `cpi rd, k` | `rd: Reg`, `k: Deferred` |  | Compares `rd` with `k` worked out later. |

### `cpse`

Skips the next instruction if `rd` equals `rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `cpse rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `sbrc`

Skips the next instruction if bit `b` of `rr` is clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbrc rr, b` | `rr: Reg`, `b: int` |  |  |

### `sbrs`

Skips the next instruction if bit `b` of `rr` is set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbrs rr, b` | `rr: Reg`, `b: int` |  |  |

### `sbic`

Skips the next instruction if bit `b` of I/O register `a`, 0 to 31, is
clear.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbic a, b` | `a: int`, `b: int` |  |  |

### `sbis`

Skips the next instruction if bit `b` of I/O register `a`, 0 to 31, is
set.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sbis a, b` | `a: int`, `b: int` |  |  |

### `mov`

`rd = rr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mov rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `movw`

Copies the pair starting at `rr` to the pair starting at `rd`. Both
are even.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `movw rd, rr` | `rd: Reg`, `rr: Reg` |  |  |

### `ldi`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ldi rd, k` | `rd: Reg`, `k: int` |  | `rd = k`, for r16 to r31. |
| `ldi rd, k` | `rd: Reg`, `k: Deferred` |  | `rd = k`, with `k` worked out later: `ldi r30, lo8(table)`. |

### `ld`

Loads `rd` from the data address in `p`: `X`, `Y`, `Z`, or one of them
incremented after or decremented before.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ld rd, p` | `rd: Reg`, `p: Pointer` |  |  |

### `ldd`

Loads `rd` from `q` bytes, 0 to 63, past the address in `Y` or `Z`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ldd rd, p, q` | `rd: Reg`, `p: Pointer`, `q: int` |  |  |

### `st`

Stores `rr` at the data address in `p`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `st p, rr` | `p: Pointer`, `rr: Reg` |  |  |

### `std`

Stores `rr` `q` bytes, 0 to 63, past the address in `Y` or `Z`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `std p, q, rr` | `p: Pointer`, `q: int`, `rr: Reg` |  |  |

### `lds`

Loads `rd` from data address `k`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lds rd, k` | `rd: Reg`, `k: int` |  |  |

### `sts`

Stores `rr` at data address `k`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sts k, rr` | `k: int`, `rr: Reg` |  |  |

### `lpm`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `lpm` |  |  | Loads r0 from the flash byte address in `Z`. |
| `lpm rd, p` | `rd: Reg`, `p: Pointer` |  | Loads `rd` from the flash byte address in `p`: `Z`, or `Z` incremented after. |

### `spm`

Writes flash from r1:r0 at the address in `Z`, under the rules of the
chip's self-programming mode.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `spm` |  |  |  |

### `in_`

Reads I/O register `a`, 0 to 63, into `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `in_ rd, a` | `rd: Reg`, `a: int` |  |  |

### `out`

Writes `rr` to I/O register `a`, 0 to 63.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `out a, rr` | `a: int`, `rr: Reg` |  |  |

### `push`

Pushes `rr` onto the stack.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `push rr` | `rr: Reg` |  |  |

### `pop`

Pops the stack into `rd`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `pop rd` | `rd: Reg` |  |  |

### `nop`

Does nothing for one cycle.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `nop` |  |  |  |

### `sleep`

Enters the sleep mode set in the chip's control register.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sleep` |  |  |  |

### `wdr`

Resets the watchdog timer.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `wdr` |  |  |  |

### `break`

Stops for an on-chip debugger.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `break` |  |  |  |

### `db`

One byte of data in flash.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `db value` | `value: int` |  |  |

### `dw`

One 16-bit word of data in flash, least significant byte first.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `dw value` | `value: int` |  |  |

## Types

### `Reg`

`type Reg = bits<5>`

A register number, 0 to 31.

### `Pointer`

`struct Pointer`

A pointer register pair as an `ld` or `st` operand: `X`, `Y` or `Z`,
used as it is, incremented after, or decremented before.

| Field | Type | Description |
|---|---|---|
| `base` | `int` |  |
| `step` | `int` |  |

### `Word`

`type Word = LittleEndian<bits<16>, 16>`

One instruction word, least significant byte first, as flash stores it.

### `LongWord`

`struct LongWord`

A 32-bit instruction: two words, each least significant byte first.

### `RelWord`

`struct RelWord`

`rjmp` and `rcall`: a 12-bit distance in words.

### `BranchWord`

`struct BranchWord`

A conditional branch: a 7-bit distance in words, and an SREG bit.

### `ImmWord`

`struct ImmWord`

An instruction with an 8-bit immediate that may be worked out later,
like `ldi r30, lo8(table)`.

### `JumpWords`

`struct JumpWords`

`jmp` and `call` to a label: a 22-bit word address split across two
words.

### `JumpHigh`

`struct JumpHigh`

The first word of `JumpWords`.

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `r0` |  | `Reg(0)` | A general-purpose register. |
| `r1` |  | `Reg(1)` | A general-purpose register. |
| `r2` |  | `Reg(2)` | A general-purpose register. |
| `r3` |  | `Reg(3)` | A general-purpose register. |
| `r4` |  | `Reg(4)` | A general-purpose register. |
| `r5` |  | `Reg(5)` | A general-purpose register. |
| `r6` |  | `Reg(6)` | A general-purpose register. |
| `r7` |  | `Reg(7)` | A general-purpose register. |
| `r8` |  | `Reg(8)` | A general-purpose register. |
| `r9` |  | `Reg(9)` | A general-purpose register. |
| `r10` |  | `Reg(10)` | A general-purpose register. |
| `r11` |  | `Reg(11)` | A general-purpose register. |
| `r12` |  | `Reg(12)` | A general-purpose register. |
| `r13` |  | `Reg(13)` | A general-purpose register. |
| `r14` |  | `Reg(14)` | A general-purpose register. |
| `r15` |  | `Reg(15)` | A general-purpose register. |
| `r16` |  | `Reg(16)` | A general-purpose register. |
| `r17` |  | `Reg(17)` | A general-purpose register. |
| `r18` |  | `Reg(18)` | A general-purpose register. |
| `r19` |  | `Reg(19)` | A general-purpose register. |
| `r20` |  | `Reg(20)` | A general-purpose register. |
| `r21` |  | `Reg(21)` | A general-purpose register. |
| `r22` |  | `Reg(22)` | A general-purpose register. |
| `r23` |  | `Reg(23)` | A general-purpose register. |
| `r24` |  | `Reg(24)` | A general-purpose register. |
| `r25` |  | `Reg(25)` | A general-purpose register. |
| `r26` |  | `Reg(26)` | A general-purpose register. |
| `r27` |  | `Reg(27)` | A general-purpose register. |
| `r28` |  | `Reg(28)` | A general-purpose register. |
| `r29` |  | `Reg(29)` | A general-purpose register. |
| `r30` |  | `Reg(30)` | A general-purpose register. |
| `r31` |  | `Reg(31)` | A general-purpose register. |
| `X` |  | `Pointer(load = 0x900C, base = 26, step = 0)` | `X`, the pair r27:r26. |
| `X_INC` |  | `Pointer(load = 0x900D, base = 26, step = 1)` | `X+`: `X`, incremented after the access. |
| `X_DEC` |  | `Pointer(load = 0x900E, base = 26, step = -1)` | `-X`: `X`, decremented before the access. |
| `Y` |  | `Pointer(load = 0x8008, base = 28, step = 0)` | `Y`, the pair r29:r28. |
| `Y_INC` |  | `Pointer(load = 0x9009, base = 28, step = 1)` | `Y+`: `Y`, incremented after the access. |
| `Y_DEC` |  | `Pointer(load = 0x900A, base = 28, step = -1)` | `-Y`: `Y`, decremented before the access. |
| `Z` |  | `Pointer(load = 0x8000, base = 30, step = 0)` | `Z`, the pair r31:r30. |
| `Z_INC` |  | `Pointer(load = 0x9001, base = 30, step = 1)` | `Z+`: `Z`, incremented after the access. |
| `Z_DEC` |  | `Pointer(load = 0x9002, base = 30, step = -1)` | `-Z`: `Z`, decremented before the access. |
| `FLAG_C` |  | `0` | Carry. |
| `FLAG_Z` |  | `1` | Zero. |
| `FLAG_N` |  | `2` | Negative. |
| `FLAG_V` |  | `3` | Two's complement overflow. |
| `FLAG_S` |  | `4` | Sign: `N` xor `V`. |
| `FLAG_H` |  | `5` | Half carry. |
| `FLAG_T` |  | `6` | The bit `bst` stores and `bld` loads. |
| `FLAG_I` |  | `7` | Interrupts enabled. |
