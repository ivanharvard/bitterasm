# BitterASM

Making Assembly Sweeter.

## What BitterASM is

BitterASM is a metalanguage for constructing assembly languages. It defines the semantics and metaprogramming machinery needed to build an ISA, while assuming essentially nothing about the target architecture itself.

Traditional assemblers bake in a specific architecture: registers, instructions, opcodes, addressing modes, and binary encoding are all privileged, built-in concepts. That works fine until you want something the assembler's authors didn't anticipate — a pseudo-instruction, an alternate syntax, a non-binary target, a research architecture. Then you're stuck extending the assembler itself.

BitterASM inverts this. The language core knows nothing about:

```text
registers, instructions, opcodes, operands, immediates,
addresses, word sizes, endianness, calling conventions,
sections, labels, binary
```

Every one of those is defined in ordinary BitterASM code — as libraries, architecture packages, and evaluators. The complexity of a real ISA lives where it belongs: in the code describing that ISA, not in the language interpreting it.

## The core ideas

**Instructions are macros, not language features.** `mov` isn't a built-in instruction declaration — it's a macro someone wrote and exposed publicly. There's no fundamental distinction between a "real" instruction and a "pseudo" one at the language level; both are just macros that expand into some encoding.

**Syntax is an interface, not part of the ISA.** Because instructions are macros with their own invocation syntax, the same underlying x86 machinery could support `mov rax, 3`, an arrow-style `rax <- 3`, or something else entirely — all sharing one architecture implementation underneath.

**Abstraction does not imply optimization.** What you expand is what you get. If a macro expands `clear rax` into `xor rax, rax`, that's the macro author's explicit choice. BitterASM never second-guesses it by picking a "faster" sequence on its own. A macro can implement sophisticated code selection, but that logic belongs to the macro author, not the language.

**Even bits are a library concept.** The only thing BitterASM assumes is `Int` — an architecture-neutral, arbitrary-precision integer. Binary (`bit`, `bits<N>`) is defined in terms of `Int` in the standard library, not built into the language. `3` is just the mathematical integer three until some library gives it a binary interpretation. This leaves room for architectures that aren't binary at all.

**Evaluators own the output contract, not the language.** BitterASM source describes behavior; an evaluator decides what running that behavior produces. A binary evaluator turns assembly into machine code. A different evaluator could target something else entirely. Language semantics (`Int`, `struct`, `type`, `macro`, pattern matching, imports, generics) mean the same thing everywhere — only the output effect changes.

**Imports compose modules; they don't grant magic.** `from x86_64.native import *` works because that package defines `mov` and `rax` — not because the language has special knowledge of x86. Importing a binary library doesn't secretly switch the language into "binary mode"; binary is just one ordinary abstraction among many.

## Why bother

The payoff for assuming almost nothing is:

- **Portability** — the same metaprogramming core builds tiny pedagogical ISAs, real architectures like RISC-V, and pathologically complex ones like x86-64, without special-casing any of them.
- **Auditable, structured assembly** — instructions and their expansions are ordinary, inspectable code rather than opaque tables inside an assembler binary.
- **Alternate syntaxes for free** — because syntax lives with the macros that define it, an architecture can offer multiple front-ends (native, AT&T-style, "pretty," even something Pythonic) over one shared implementation.
- **Room for the unknown** — future architectures that aren't neatly binary, register-based, or von Neumann at all don't require changes to BitterASM itself.

The tradeoff is symmetric: the fewer assumptions the language makes, the more an architecture package has to define for itself. That's the bitter part. The resulting portability, auditability, and extensibility are the sweet part.

## Status

BitterASM is an early-stage project (lexer, parser, and resolver in Rust; a growing `std` library in BitterASM itself). Expect the language and standard library to change as the design settles.
