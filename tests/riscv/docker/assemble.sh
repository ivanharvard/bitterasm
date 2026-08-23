#!/bin/bash
# Reads a whole RV32I .s file on stdin — real labels included — and writes
# its assembled machine-code bytes to stdout, via GNU as + ld
# (binutils-riscv64-linux-gnu).
#
# Assembled and linked as one program, with .text placed at address 0
# (`-Ttext=0x0`) so the first instruction sits at PC 0 — matching
# `bitterasm`'s own convention that `@here`/a label's position starts
# counting from the top of the program (see std/riscv/native.basm's
# branch/jump macros). Whole-file, not per-instruction: earlier versions of
# this script assembled one bare-literal-offset instruction at a time,
# synthesizing a fake local label via `.org` so GNU as had *something* to
# resolve a bare number against — but bitterasm's branch/jump macros no
# longer take a raw byte offset at all (see native.basm's own doc comment:
# their operand is now the target *instruction*, converted to a byte delta
# internally via `@here`). GNU as has no equivalent "raw already-computed
# delta" mode for a bare literal — it always treats a branch/jump operand
# as a target address — so a bare-literal test case can no longer agree
# with this oracle at all; real labels, which both sides resolve
# independently and are expected to agree on by the whole design of the
# feature, are the only case left to test against it. That's also what
# lets this script drop the old `.org`/synthetic-label machinery entirely:
# a real label needs none of it, GNU as/ld resolve it natively.
set -euo pipefail

AS=riscv64-linux-gnu-as
LD=riscv64-linux-gnu-ld
OBJCOPY=riscv64-linux-gnu-objcopy

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

cat >"$workdir/in.s"

if ! "$AS" -march=rv32i -mabi=ilp32 -o "$workdir/out.o" "$workdir/in.s" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "official encoder failed to assemble" >&2
    exit 1
fi

# -m elf32lriscv: riscv64-linux-gnu-ld's default emulation is the 64-bit
# one, which refuses to link the ELF32 object `as -mabi=ilp32` just
# produced ("ABI is incompatible with that of the selected emulation").
# --relax: lets `ld` collapse an out-of-range-safe branch/jump sequence
# back down to its minimal real encoding once every label's final address
# is known.
if ! "$LD" -m elf32lriscv --relax -Ttext=0x0 -e 0 -o "$workdir/out.elf" "$workdir/out.o" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "official encoder failed to link" >&2
    exit 1
fi

"$OBJCOPY" -O binary --only-section=.text "$workdir/out.elf" "$workdir/out.bin"

cat "$workdir/out.bin"
