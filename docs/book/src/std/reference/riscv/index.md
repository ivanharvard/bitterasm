# `riscv`

[`std`](../index.md) › `riscv`

| Name | Summary |
|---|---|
| [`c_like`](c_like.md) | RISC-V RV32I written as expressions instead of mnemonics: `a0 = a1 + a2`, `a1 = mem[sp + 8]`, `if (a0 != zero) goto loop`. |
| [`impl`](impl.md) | RISC-V RV32I: its registers, instruction formats, and every base instruction, each emitted as a little-endian 32-bit word. |
| [`native`](native.md) | RISC-V RV32I in its standard assembly syntax: `add a0, a1, a2`, `lw a0, 8(sp)`, `beq a0, zero, done`. |
