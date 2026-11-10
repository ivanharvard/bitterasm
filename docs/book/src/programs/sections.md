# Sections

A section is a named region of the output, such as code or read-only data.

## Syntax

```basm,ignore
section name
section name in space | facet ...
@into value
```

Everything emitted after a `section` statement belongs to that section, until
the next `section` statement. A name can be reopened any number of times.

The name means nothing to the compiler, or to `bitter`: `.text`, `.rodata`
and `code` are all just names. What a section *is* comes from the second
form, which gives the [space](spaces.md) it goes in and its facets.

## Sections are values

Written where a value goes, `section name in space | facet ...` is a
section, the way `"abc"` is a string: a `const` can hold it, and a macro
can take one. `@into` switches to a section value:

```basm
from std.binary import bits

macro db(value: int) {
    @emit value as bits<8>
}

const DATA = section .data

section .text
    db 1
@into DATA
    db 0xaa
```

```bytes
01 aa
```

A `section` line is shorthand for `@into` on the section it names, so
inside a macro, `section s` is a section named `s`, not the parameter `s`:
switching to a parameter is `@into s`. In a section value, a facet's
arguments go in parentheses, `section .text | tagged(3)`.

A library shares a section as a const, like any other value:

```basm,ignore
pub const TEXT = section .text | progbits | alloc | exec
```

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
