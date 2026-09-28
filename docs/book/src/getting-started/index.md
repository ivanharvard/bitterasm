# Getting started

BitterASM comes as two programs:

- **`bitterasm`**, the compiler. It reads `.basm` source, expands every
  macro, and writes the values the program emits to a `.em` file.
- **`bitter`**, an evaluator. It reads a `.em` file and packs its values into
  machine-code bytes.

Keeping them separate is deliberate. `bitterasm` knows nothing about bits,
bytes or machine code: it only produces a stream of values. What those values
*mean* is up to whichever evaluator reads them, and `bitter` is the one that
turns them into binary. See [Evaluators](../output/index.md).

This chapter covers:

- [Installation](installation.md): building and installing both programs and
  the standard library.
- [Your first program](first-program.md): assembling two RISC-V instructions,
  and then a program that emits plain numbers.
- [The toolchain](toolchain.md): every `bitterasm` and `bitter` command.
