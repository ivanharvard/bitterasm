# Summary

[Introduction](introduction.md)

# Getting started

- [Getting started](getting-started/index.md)
    - [Installation](getting-started/installation.md)
    - [Your first program](getting-started/first-program.md)
    - [The toolchain](getting-started/toolchain.md)

# Philosophy

- [`Int` is the only emitted type]()
- [Instructions are macros]()
- [Syntax is an interface]()
- [Evaluators own the output]()

# Tutorial

- [Building a toy ISA from nothing]()

# The language

- [Basics](basics/index.md)
    - [Integers](basics/integers.md)
    - [Operators](basics/operators.md)
    - [Ranges and `in`](basics/ranges.md)
    - [Characters and strings](basics/strings.md)
    - [Constants](basics/constants.md)
    - [Doc comments](basics/doc-comments.md)
- [Macros](macros/index.md)
    - [Parameters and defaults](macros/parameters.md)
    - [Returning values](macros/returning.md)
    - [Overloading](macros/overloading.md)
    - [Recursion](macros/recursion.md)
    - [Hooks: `before` and `after`](macros/hooks.md)
    - [Custom syntax](macros/syntax.md)
    - [Generating declarations](macros/generating.md)
- [Meta keywords](meta/index.md)
    - [`@emit`](meta/emit.md)
    - [`@return`](meta/return.md)
    - [`@assert`](meta/assert.md)
    - [`@if` and `@else`](meta/if.md)
    - [`@match`](meta/match.md)
    - [`@for`](meta/for.md)
    - [`@fold` and `@next`](meta/fold.md)
- [Types](types/index.md)
    - [Structs](types/structs.md)
    - [Struct fields: `pub` and `skip`](types/fields.md)
    - [Enums](types/enums.md)
    - [Type aliases](types/aliases.md)
    - [Invariants](types/invariants.md)
    - [Conversions: `as`, `to` and `from`](types/conversions.md)
- [Generics](generics/index.md)
    - [Const parameters](generics/const.md)
    - [Wildcards: `...`](generics/wildcards.md)
    - [Macros as parameters](generics/macro-parameters.md)
- [Splicing](splicing/index.md)
    - [Spliced names](splicing/names.md)
- [Modules](modules/index.md)
    - [Imports](modules/imports.md)
    - [Visibility and re-exports](modules/visibility.md)
- [Programs](programs/index.md)
    - [Labels](programs/labels.md)
    - [Sections](programs/sections.md)
    - [Linking multiple files](programs/linking.md)
    - [Executables](programs/executables.md)

# Output

- [Evaluators](output/index.md)
    - [The `.em` format](output/em-format.md)
    - [Packing bytes with `bitter`](output/bitter.md)

# Tools

- [Tools](tools/index.md)
    - [Formatting](tools/formatting.md)
    - [Diagnostics and lints](tools/diagnostics.md)
    - [Generating docs](tools/docs.md)

# Standard library

- [The standard library](std/index.md)
- [std reference](std/reference/index.md)
    <!-- bitterasm doc: begin -->
    - [`array`](std/reference/array.md)
    - [`binary`](std/reference/binary.md)
    - [`bitfield`](std/reference/bitfield.md)
    - [`bitter`](std/reference/bitter/index.md)
        - [`byte_order`](std/reference/bitter/byte_order.md)
        - [`deferred`](std/reference/bitter/deferred.md)
        - [`layout`](std/reference/bitter/layout.md)
        - [`link`](std/reference/bitter/link.md)
    - [`ctypes`](std/reference/ctypes.md)
    - [`decimal`](std/reference/decimal.md)
    - [`enumerated`](std/reference/enumerated.md)
    - [`formats`](std/reference/formats/index.md)
        - [`elf`](std/reference/formats/elf.md)
        - [`macho`](std/reference/formats/macho.md)
        - [`pe`](std/reference/formats/pe.md)
    - [`iter`](std/reference/iter.md)
    - [`math`](std/reference/math.md)
    - [`option`](std/reference/option.md)
    - [`pdp10`](std/reference/pdp10/index.md)
        - [`impl`](std/reference/pdp10/impl.md)
        - [`tops10`](std/reference/pdp10/tops10.md)
    - [`riscv`](std/reference/riscv/index.md)
        - [`c_like`](std/reference/riscv/c_like.md)
        - [`impl`](std/reference/riscv/impl.md)
        - [`native`](std/reference/riscv/native.md)
    - [`string`](std/reference/string.md)
    - [`unsigned`](std/reference/unsigned.md)
    - [`wasm`](std/reference/wasm/index.md)
        - [`impl`](std/reference/wasm/impl.md)
        - [`leb128`](std/reference/wasm/leb128.md)
        - [`module`](std/reference/wasm/module.md)
    - [`x86_64`](std/reference/x86_64/index.md)
        - [`att`](std/reference/x86_64/att.md)
        - [`impl`](std/reference/x86_64/impl.md)
        - [`intel`](std/reference/x86_64/intel.md)
        - [`nasm`](std/reference/x86_64/nasm.md)
    <!-- bitterasm doc: end -->

---

[Writing the docs](contributing/writing-docs.md)
