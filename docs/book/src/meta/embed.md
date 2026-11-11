# `@embed`

`@embed "path"` is a file's contents, as a value. It's shaped like a
[string](../basics/strings.md): one `int` field per unit of the file,
`__el0`, `__el1`, ..., and a `len` field.

```basm,ignore
const raw = @embed "logo.bin"

raw.len     # how many units the file holds
raw.__el0   # the first one
```

The path starts from the directory of the file the `@embed` is written in,
so a library's macro finds a file beside the library, wherever it's called
from.

## Nothing is interpreted

`@embed` doesn't decode anything. Each unit is a number for what the file
holds, the way the machine running the compiler stores it. Today's
machines store files as bytes, so a unit is 0 to 255. A text file isn't
read as characters: `é` in a UTF-8 file is two units, `0xC3` and `0xA9`,
where the string literal `"é"` is one character, 233.

What the units mean is up to the program. `std.embed` has the macros for
a file of bytes:

| Macro | Does |
|---|---|
| `emit_bytes(raw)` | Emits each unit as a byte: pastes the file into the output |
| `as_bytes(raw)` | The units as an `Array<bits<8>, ...>` |

Both fail to compile on a unit that doesn't fit in a byte.

```basm,ignore
from std.binary import bits
from std.embed import emit_bytes

macro db(value: int) {
    @emit value as bits<8>
}

# A length byte, then the image, untouched.
db (@embed "image.bin").len
emit_bytes @embed "image.bin"
```

A format that needs to read a file's contents, not just paste them, loops
over the units. `std.formats.ihex` writes an image as Intel HEX that way:

```basm,ignore
from std.formats.ihex import ihex

ihex @embed "blink.bin", 0
```

## Size

Every unit is a field, so a large file is a large struct. Pasting a
70 KiB file takes half a second in a release build, and writing it as
Intel HEX a few seconds. A file of more than 1,000,000 units is an error.
