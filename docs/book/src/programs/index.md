# Programs

A program's result is the list of values it emits, in order. Everything in
this chapter is about arranging that list:

- [Labels](labels.md) name positions in it.
- [Sections](sections.md) group it into regions, like code and data.
- [Linking](linking.md) combines the lists of several files.
- [Executables](executables.md) put a header in front, so the result can run.

## A complete program

This is `examples/x86_64/hello.basm` from the repository, a "Hello, world!"
for Linux on x86-64:

```basm
from std.x86_64.nasm import *
from std.formats.elf import *

elf64_executable EM_X86_64, _start

const text = "Hello, World!\n"

section .rodata
msg:
    db text

section .text
pub _start:
    mov eax, 1          # sys_write
    mov edi, 1          # stdout
    lea rsi, [rel msg]  # buffer
    mov edx, text.len   # length
    syscall

    mov eax, 60         # sys_exit
    xor edi, edi        # status 0
    syscall
```

Every piece of it is ordinary BitterASM:

- `elf64_executable` is a macro from `std.formats.elf` that emits an ELF
  header. See [Executables](executables.md).
- `section .rodata` and `section .text` put the string and the code in
  separate [sections](sections.md).
- [`const text`](../basics/constants.md) names the string, so `db text` can emit it and
  `text.len` gives its length.
- `msg:` and `_start:` are [labels](labels.md). `pub` exports `_start`.
- `lea`, `mov`, `syscall` and the rest are macros from
  `std.x86_64.nasm`, and `db` is NASM's data directive.

Build and run it:

```sh
bitter build examples/x86_64/hello.basm -o hello
./hello
```

## Top-level statements run in order

The statements at the top level of a file run from top to bottom. Each call
appends whatever it emits. Declarations can appear anywhere, since they're
visible throughout the file.
