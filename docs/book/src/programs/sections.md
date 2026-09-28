# Sections

A section is a named region of the output, such as code or read-only data.

## Syntax

```basm,ignore
section name
```

Everything emitted after a `section` statement belongs to that section, until
the next `section` statement. A name can be reopened any number of times.

The name means nothing to the compiler, or to `bitter`: `.text`, `.rodata`
and `code` are all just names.

## Sections are grouped

When `bitter` packs a program, it groups each section's values together.
Sections appear in the order each was first opened, and anything emitted
before the first `section` statement comes before all of them:

```basm
from std.binary import bits

macro db(value: int) {
    @emit value as bits<8>
}

    db 1
section .data
    db 0xaa
section .text
    db 2
section .data
    db 0xbb
```

```bytes
01 aa bb 02
```

With several input files, `bitter build` also joins same-named sections
across files. See [Linking multiple files](linking.md).

## Sections inside macros

A `section` statement inside a macro only lasts until the macro returns. The
caller's section is then restored, so calling a macro can't move the
caller's code by accident:

```basm
from std.binary import bits

macro db(value: int) {
    @emit value as bits<8>
}

macro stash(value: int) {
    section .data
    db value
}

section .text
    db 1
    stash 0xaa
    db 2
```

```bytes
01 02 aa
```

A macro whose job is to switch the caller's section, like a shorthand for
`section .data`, declares the `leaks_section` facet:

```basm
from std.binary import bits

macro db(value: int) {
    @emit value as bits<8>
}

macro data()
    | leaks_section
{
    section .data
}

section .text
    db 1
data
    db 0xaa
section .text
    db 2
```

```bytes
01 02 aa
```
