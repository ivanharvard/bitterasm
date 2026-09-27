# Characters and strings

## Characters

A character literal like `'a'` is just an `int`: the character's Unicode
code point. `'€'` is `0x20AC`, whatever machine you target. Characters carry
no encoding and no byte order.

```basm
macro show(value: int) {
    @emit value
}

show 'A'
show '€'
show '\n'
```

```emits
65 8364 10
```

A character literal holds exactly one character. The escapes are `\n`, `\r`,
`\t`, `\0`, `\\` and `\'`.

## Strings

A string literal like `"abc"` is a struct with one `int` field per character
(its code point) and a `len` field:

| Field | Value for `"hé€"` |
|---|---|
| `__el0` | `'h'` (104) |
| `__el1` | `'é'` (233) |
| `__el2` | `'€'` (8364) |
| `len` | `3` |

`len` counts characters, not bytes. Strings accept the same escapes as
characters, with `\"` in place of `\'`.

```basm
macro show(value: int) {
    @emit value
}

const text = "hé€"
show text.len
show text.__el1
```

```emits
3 233
```

[`@for`](../meta/for.md) visits a string's characters, but not its `len`,
which is declared `pub skip` (see [Struct fields](../types/fields.md)):

```basm
macro each_char() {
    @for c in "ab" {
        @emit c
    }
}

each_char
```

```emits
97 98
```

## Strings as bytes

A string literal has no encoding until a library gives it one:

- `std.array`'s `array_from_struct("abc")` turns it into an
  `Array<int, 3>`.
- `std.string`'s `string_from_struct("abc")` encodes it as UTF-8, packed
  into one integer, with a `len` in bytes.

```basm
from std.string import string_from_struct

macro packed() {
    const s = string_from_struct("hi")
    @emit s.value
    @emit s.len
}

packed
```

```emits
26729 2
```

`26729` is `0x6869`: the bytes `h` (`0x68`) and `i` (`0x69`).

To put a string's bytes in a program, an architecture package provides a
directive for it. For example, `std.x86_64.nasm` has NASM's `db`:

```basm,ignore
msg:
    db "Hello, World!\n"
```
