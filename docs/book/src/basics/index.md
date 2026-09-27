# Basics

A BitterASM program is a text file, usually ending in `.basm`. This chapter
covers what every program is made of: its layout, and the values it
computes with.

## One statement per line

A newline ends a statement, just as in a traditional assembler:

```basm
macro show(value: int) {
    @emit value
}

show 1
show 2
```

```emits
1 2
```

Inside `()`, `[]` and `{}`, a statement can continue onto the next line, so a
long call can be split:

```basm
macro add3(a: int, b: int, c: int) -> int {
    @return a + b + c
}

macro show(value: int) {
    @emit value
}

show add3(
    1,
    2,
    3
)
```

```emits
6
```

## Comments

`#` starts a comment that runs to the end of the line. There are no block
comments.

```basm
# A whole-line comment.
macro show(value: int) {
    @emit value    # a trailing comment
}

show 7
```

```emits
7
```

## Names

Names may contain letters, digits and underscores, and don't start with a
digit. Any Unicode letter counts, so `π` is a valid name. A name that starts
with an underscore marks a parameter as deliberately unused (see
[Diagnostics and lints](../tools/diagnostics.md)).

The only reserved words are `from`, `import`, `as`, `in`, `pub`, `skip`,
`macro`, `type`, `struct`, `enum`, `const` and `section`. Everything else,
including `int` and every instruction mnemonic, is an ordinary name.

## Order doesn't matter

A file's declarations are visible throughout the file, so you can use a
macro or constant above the line that declares it:

```basm
show LIMIT

const LIMIT = 9

macro show(value: int) {
    @emit value
}
```

```emits
9
```

What *does* follow source order is the output: calls emit their values in the
order they appear.

## What a file can contain

| Statement | Example | Chapter |
|---|---|---|
| Import | `from std.binary import bits` | [Modules](../modules/index.md) |
| Constant | `const WIDTH = 8` | [Constants](constants.md) |
| Macro | `macro nop() { ... }` | [Macros](../macros/index.md) |
| Struct, enum, type alias | `struct Point { ... }` | [Types](../types/index.md) |
| Macro call | `add a0, a1, a2` | [Macros](../macros/index.md) |
| Label | `loop:` | [Labels](../programs/labels.md) |
| Section | `section .text` | [Sections](../programs/sections.md) |
| Syntax override | `syntax add(a, b) = { ... }` | [Custom syntax](../macros/syntax.md) |
| `@for`, `@if`, `@fold` | `@for i in 0..4 { ... }` | [Meta keywords](../meta/index.md) |

## In this chapter

- [Integers](integers.md): the one built-in type.
- [Operators](operators.md): arithmetic, bitwise, comparison and logic.
- [Ranges and `in`](ranges.md): `0..n`, and testing membership.
- [Characters and strings](strings.md).
- [Constants](constants.md): naming values with `const`.
