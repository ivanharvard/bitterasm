# The "official encoder" tests/x86_64/run_tests.py checks bitter's raw
# machine-code output against: GNU binutils' x86-64 assembler, packaged and
# maintained independently of this project, sharing no code with
# std/x86_64/native.basm or with bitter's own packer (bitter/src/pack.rs).
#
# Unlike std/riscv's oracle, this needs no cross-toolchain package — plain
# Debian's `binutils` already targets x86-64 natively. Built with
# `--platform linux/amd64` (see run_tests.py) so this is genuine x86-64
# binutils regardless of the host machine's own architecture (e.g. an
# Apple Silicon Mac, which would otherwise get arm64 binutils from the
# same `apt-get install binutils`).
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends binutils \
    && rm -rf /var/lib/apt/lists/*

COPY assemble.sh /usr/local/bin/assemble.sh
RUN chmod +x /usr/local/bin/assemble.sh

ENTRYPOINT ["/usr/local/bin/assemble.sh"]
