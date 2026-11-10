# Link stages

Without one, a program's output is its sections, one after another, in
the order each was first used. A **link stage** lays the output out
instead: which sections go in the file and in what order, what comes
before them, like an executable's headers, and where each section is in
its space.

```basm,ignore
@link name(arguments)
```

`@link` runs the macro once everything else in the program has run, so
every section is complete. What it emits is the output. It goes at the top
level of the program's own file, once. A library's `@link` only runs when
the library is the program, the way its other top-level code does.

## Writing sections

`std.bitter.image`'s `write_section` puts a section's contents in the
output at that point. A section that isn't written takes no room in the
file:

```basm
from std.binary import bits
from std.bitter.image import write_section

macro db(value: int) {
    @emit value as bits<8>
}

macro data_first(code: section, data: section) {
    db 0xff
    write_section data
    write_section code
}

const CODE = section .text
const DATA = section .data

@link data_first(CODE, DATA)

@into CODE
db 1
@into DATA
db 2
```

```bytes
ff 02 01
```

The link macro takes its sections as arguments, so a program says which
sections it has and in what order. There's no list of every section for it
to find: the program's own argument is the list.

## Placing sections

`place_section sec, address` says where `sec` is in its space. Its labels are
there, and the distance between two placed positions, the `span` a branch
or a RIP-relative operand is encoded from, is the distance between their
addresses. So code in one section can reach data in another wherever the
two are placed.

`std.bitter.deferred` reads the layout back, for headers that describe it:

| | |
|---|---|
| `offset(l)` | Where `l` is in the output file, in bytes |
| `address(l)` | Where `l` is in its space |
| `size_of(sec)` | How many bytes `sec` holds |

A section value's `start` and `end` are labels at its first value and just
past its last, so `offset(sec.start)` is where a written section begins.

## Executables

`std.formats.elf`'s `elf64_link` is a link stage for ELF executables. Each
section becomes its own segment, with the permissions its facets give it,
and a `nobits` section, like `.bss`, takes no room in the file and starts
zeroed:

```basm,ignore
from std.array import Array
from std.formats.elf import *
from std.x86_64.nasm import *

@link elf64_link(EM_X86_64, _start, Array<section, 3>(TEXT, RODATA, BSS))

@into TEXT
pub _start:
    mov eax, 60
    mov edi, [rel status]   # 0: `.bss` starts zeroed
    syscall

@into RODATA
greeting:
    db "hi"

@into BSS
status:
    resb 4
```

`examples/x86_64/hexdump.basm` is a whole program built this way.

## Limits

A program with a link stage is built from one file for now. With several,
a section written from one file wouldn't include the same section's values
from another.
