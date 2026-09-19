#!/bin/bash
# Reads a whole x86-64 .s file on stdin (Intel syntax — a leading
# `.intel_syntax noprefix` directive is prepended here, not something each
# cases/*.s file needs to spell itself, matching how those files are also
# fed directly to bitterasm as native-dialect source) and writes its
# assembled machine-code bytes to stdout, via GNU as + ld (binutils).
#
# Whole-file, not per-instruction, and real labels only — same reasoning
# tests/riscv/docker/assemble.sh's own doc gives: bitterasm's own
# jmp/jcc/call take a target *instruction*, resolved to a byte delta via
# here()/Deferred at pack time, so a bare already-computed literal offset
# has nothing for this oracle to independently agree with; a real label,
# resolved natively by GNU as/ld, is the only comparable case.
#
# .text is placed at address 0 (`-Ttext=0x0`), matching bitterasm's own
# convention that a label's position starts counting from the top of the
# program.
set -euo pipefail

AS=as
LD=ld
OBJCOPY=objcopy

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

{
    echo ".intel_syntax noprefix"
    cat
} >"$workdir/in.s"

if ! "$AS" --64 -o "$workdir/out.o" "$workdir/in.s" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "official encoder failed to assemble" >&2
    exit 1
fi

if ! "$LD" -Ttext=0x0 -e 0 -o "$workdir/out.elf" "$workdir/out.o" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "official encoder failed to link" >&2
    exit 1
fi

"$OBJCOPY" -O binary --only-section=.text "$workdir/out.elf" "$workdir/out.bin"

cat "$workdir/out.bin"
