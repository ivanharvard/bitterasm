# The "official encoder" tests/riscv/run_tests.py checks bitter's raw
# machine-code output against: GNU binutils' RISC-V assembler, packaged
# and maintained independently of this project, sharing no code with
# std/riscv/native.basm or with bitter's own packer (bitter/src/pack.rs).
#
# Debian doesn't package a standalone riscv32 cross-binutils, so this uses
# the riscv64-linux-gnu one restricted to the plain base ISA via
# `-march=rv32i -mabi=ilp32` (see assemble.sh) — RISC-V binutils' single
# backend covers both XLENs, so this still produces genuine RV32I
# encoding, not anything derived from a 64-bit assumption.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends binutils-riscv64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

COPY assemble.sh /usr/local/bin/assemble.sh
RUN chmod +x /usr/local/bin/assemble.sh

ENTRYPOINT ["/usr/local/bin/assemble.sh"]
