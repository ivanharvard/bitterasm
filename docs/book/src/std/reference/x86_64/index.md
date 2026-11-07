# `x86_64`

[`std`](../index.md) › `x86_64`

| Name | Summary |
|---|---|
| [`att`](att.md) | AT&T-syntax x86-64 assembly, as GNU `as` reads it: source first, destination second, `%` before registers and `$` before immediates. |
| [`data`](data.md) | Data for x86-64 programs: NASM's `db`, `dw`, `dd` and `dq`, shared by `std.x86_64.nasm` and `std.x86_64.att`, since data is spelled the same whichever syntax the instructions use. |
| [`impl`](impl.md) | The x86-64 base instruction set: general-purpose registers, memory operands, and the core integer instructions, each emitted as its machine code. |
| [`intel`](intel.md) | Intel-syntax x86-64 assembly: destination first, registers by name, and memory operands in brackets. |
| [`nasm`](nasm.md) | NASM-flavored x86-64 assembly: everything `std.x86_64.intel` provides, plus the NASM spellings it doesn't have. |
