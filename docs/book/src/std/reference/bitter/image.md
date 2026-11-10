# `image`

[`std`](../index.md) › [`bitter`](index.md) › `image`

What a link stage builds the output from. A program's `@link` macro
runs once everything else has, and what it emits is the output file:
its own values, like an executable's headers, with `write_section`
putting a section's contents in, and `place_section` saying where a
section is in its space.

```basm
from std.binary import bits
from std.bitter.image import write_section

macro db(value: int) {
    @emit value as bits<8>
}

# Data first, then code, whatever order the file has them in.
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

A section the link stage doesn't write takes no room in the file. A
placed one's labels are at its address, and the distance between two
placed positions is the distance between their addresses, so a branch
from one section to another is right wherever they're placed.

## Macros

### `write_section`

Writes `sec`'s contents into the output here.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `write_section sec` | `sec: section` | emits `Write` |  |

### `place_section`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `place_section sec, address` | `sec: section`, `address: int` | emits `Place` | Places `sec` at `address` in its space. |
| `place_section sec, address` | `sec: section`, `address: Deferred` | emits `Place` | Places `sec` at an address only known once the output is laid out, like one that depends on where the section is in the file. |

## Types

### `Write`

`struct Write`

A section's contents, from `start` up to `end`, written into the
output here. `write_section` makes one.

| Field | Type | Description |
|---|---|---|
| `start` | `Deferred` | The section's first position. |
| `end` | `Deferred` | Just past its last. |

### `Place`

`struct Place`

A section placed at `address` in its space: its first value is there,
and the rest follow. `place_section` makes one.

| Field | Type | Description |
|---|---|---|
| `start` | `Deferred` | The section's first position. |
| `end` | `Deferred` | Just past its last. |
| `address` | `Deferred` | Where its first value goes. |
