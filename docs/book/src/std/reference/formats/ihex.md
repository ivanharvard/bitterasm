# `ihex`

[`std`](../index.md) › [`formats`](index.md) › `ihex`

Intel HEX: an image as lines of text, the format microcontroller
programmers like avrdude read. Each line is a record: `:`, its length,
address and type, up to 16 data bytes, and a checksum, all in hex.

A program's bytes are only known once `bitter` has packed them, so a
HEX file takes two builds: one makes the image, and a second program
embeds it and writes it out as text:

```text
bitterasm compile blink.basm -o blink.em && bitter encode blink.em -o blink.bin
bitterasm compile to_hex.basm -o blink_hex.em && bitter encode blink_hex.em -o blink.hex
```

where `to_hex.basm` is

```text
from std.formats.ihex import ihex

ihex @embed "blink.bin", 0
```

`examples/avr/to_hex.basm` is that program.

This writes what GNU `objcopy -O ihex` does for a raw image: lines end
in CR LF, a record never crosses a 64 KiB boundary, and an extended
address record starts each 64 KiB past the first. It writes no start
address record, which only says where an x86 program starts running.

## Macros

### `ihex`

Writes `raw`, from `@embed`, as Intel HEX, its first byte at `address`.
Every unit must be a byte, 0 to 255.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `ihex raw, address` | `<S>`, `raw: S`, `address: int` |  |  |
