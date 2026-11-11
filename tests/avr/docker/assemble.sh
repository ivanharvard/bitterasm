#!/bin/bash
# Reads a whole AVR .s file on stdin, real labels included, and writes its
# assembled flash image to stdout, via GNU as + ld (binutils-avr).
#
# Assembled for the ATmega328P (`-mmcu=atmega328p`, the avr5 family, which
# has every instruction std.avr encodes), and linked with .text at address
# 0, so a label's address counts from the top of the program the way
# std.avr's does. Linked rather than only assembled so ld resolves every
# label, and without --relax, so a `jmp` or `call` stays the 32-bit form
# that was written.
set -euo pipefail

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

cat >"$workdir/in.s"

if ! avr-as -mmcu=atmega328p -o "$workdir/out.o" "$workdir/in.s" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "official encoder failed to assemble" >&2
    exit 1
fi

if ! avr-ld -mavr5 -Ttext=0x0 -e 0 -o "$workdir/out.elf" "$workdir/out.o" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "official encoder failed to link" >&2
    exit 1
fi

avr-objcopy -O binary --only-section=.text "$workdir/out.elf" "$workdir/out.bin"

cat "$workdir/out.bin"
