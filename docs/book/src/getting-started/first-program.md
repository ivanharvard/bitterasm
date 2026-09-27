# Your first program

## Assembling real instructions

Save this as `first.basm`:

```basm
from std.riscv.native import *

add a0, a1, a2
addi a0, a0, 1
```

```bytes
33 85 c5 00 13 05 15 00
```

Then compile it and pack the result:

```sh
bitterasm compile first.basm     # writes first.em
bitter encode first.em           # writes first.bin
```

`first.bin` holds the eight bytes above: two RV32I instructions,
little-endian, exactly what a RISC-V assembler would produce.

Nothing in the language itself knows what `add` or `a0` is. Both are
ordinary declarations in the standard library: `add` is a
[macro](../macros/index.md) and `a0` is a [constant](../basics/constants.md).
The import brings them into scope, like importing a library in any other
language.

## Emitting plain values

A program's output is whatever its macros **emit**. Here is a program with no
architecture at all:

```basm
macro show(value: int) {
    @emit value
}

show 1
show 2 + 3
show 'A'
```

```emits
1 5 65
```

- `macro show(value: int) { ... }` declares a macro named `show` that takes
  one integer.
- [`@emit`](../meta/emit.md) adds a value to the program's output.
- `show 1` calls the macro. A line that starts with a macro's name followed by
  its arguments is a call, just like an instruction in a traditional
  assembler.

Compile it and look at the `.em` file:

```sh
bitterasm compile show.basm
cat show.em
```

```json
{
  "version": 1,
  "requires": [],
  "module": "show",
  "exports": {},
  "entries": [
    { "kind": "Int", "value": "1" },
    { "kind": "Int", "value": "5" },
    { "kind": "Int", "value": "65" }
  ]
}
```

`bitter encode show.em` fails, and that's on purpose. A bare integer has no
width, so `bitter` can't tell how many bits `5` should take up. Giving it a
width is a library's job: `std.binary` defines `bits<N>`, and the RISC-V
package builds its instructions out of `bits<N>` fields. See
[Packing bytes with `bitter`](../output/bitter.md).

Next: [The toolchain](toolchain.md).
