#!/usr/bin/env python3
"""Wraps a flat machine-code blob (as `bitter encode` produces) in a minimal,
non-PIE, statically-linked ELF64 executable so it can actually be run.

`bitter` only ever emits raw instruction bytes with no ELF header, section
table, or load address (see docs/reference.md) -- turning that into a
runnable file is a job for the surrounding toolchain, the same way a real
`as`/`ld` pipeline would, not something BitterASM itself needs to know
about. This script builds the smallest ELF64 executable that will do:
one ELF header, one PT_LOAD program header covering the whole file, and the
code appended right after them, entered at its very first byte.

Usage: elf_wrap.py <input.bin> <output>
"""

import struct
import sys

LOAD_ADDR = 0x400000
EHDR_SIZE = 64
PHDR_SIZE = 56


def build_elf(code: bytes) -> bytes:
    entry = LOAD_ADDR + EHDR_SIZE + PHDR_SIZE
    file_size = EHDR_SIZE + PHDR_SIZE + len(code)

    ehdr = struct.pack(
        "<4sBBBBB7xHHIQQQIHHHHHH",
        b"\x7fELF",
        2,  # ELFCLASS64
        1,  # ELFDATA2LSB
        1,  # EI_VERSION
        0,  # ELFOSABI_SYSV
        0,  # ABI version
        2,  # e_type = ET_EXEC
        0x3E,  # e_machine = EM_X86_64
        1,  # e_version
        entry,  # e_entry
        EHDR_SIZE,  # e_phoff
        0,  # e_shoff
        0,  # e_flags
        EHDR_SIZE,  # e_ehsize
        PHDR_SIZE,  # e_phentsize
        1,  # e_phnum
        0,  # e_shentsize
        0,  # e_shnum
        0,  # e_shstrndx
    )

    phdr = struct.pack(
        "<IIQQQQQQ",
        1,  # p_type = PT_LOAD
        5,  # p_flags = R + X
        0,  # p_offset
        LOAD_ADDR,  # p_vaddr
        LOAD_ADDR,  # p_paddr
        file_size,  # p_filesz
        file_size,  # p_memsz
        0x1000,  # p_align
    )

    return ehdr + phdr + code


def main() -> None:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <input.bin> <output>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1], "rb") as f:
        code = f.read()

    with open(sys.argv[2], "wb") as f:
        f.write(build_elf(code))


if __name__ == "__main__":
    main()
