# Modules

Every `.basm` file is a **module**. A module sees its own declarations, plus
exactly what it imports, and nothing else.

## Module names

A module is named by its path, with `.` in place of `/` and without the
extension:

| File | Module |
|---|---|
| `std/binary.basm` | `std.binary` |
| `std/riscv/native.basm` | `std.riscv.native` |
| `helpers.basm`, next to the importing file | `.helpers` |

A leading `.` makes the path relative to the importing file's directory.
Without it, the path is looked up in the [search path](imports.md#the-search-path).

## A first import

`shapes.basm`:

```basm,file=shapes.basm
pub struct Point {
    pub x: int,
    pub y: int,
}

pub macro emit_point(p: Point) {
    @emit p
}
```

`main.basm`, in the same directory:

```basm
from .shapes import Point, emit_point

emit_point Point(1, 2)
```

```emits
Point { x: 1, y: 2 }
```

## Every module has its own namespace

Two modules can declare the same name without conflict. A name always means
what it meant in the file that wrote it: a macro imported from a library
keeps using the library's helpers, even if the importing file declares
something with the same name.

## Libraries and architecture packages

The standard library, `std`, is made of ordinary modules. So is every
architecture: `std.riscv.native` is a module that imports
`std.riscv.impl`, and `std.x86_64.nasm` builds on `std.x86_64.intel`.
There's no special kind of module for an instruction set.

## In this chapter

- [Imports](imports.md): the forms of `from ... import`, and where modules
  are found.
- [Visibility and re-exports](visibility.md): `pub`, `pub from`, and what
  happens when names collide.
