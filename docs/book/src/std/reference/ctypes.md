# `ctypes`

[`std`](index.md) › `ctypes`

C's fixed-width integer names. The signed ones are `signed<N>`, stored
in two's complement; the unsigned ones are `bits<N>` that can't be
negative.

```basm
from std.ctypes import *

macro db(value: int8_t) {
    @emit value
}

macro dw(value: uint16_t) {
    @emit value
}

db (-2) as int8_t
dw 0x1234 as uint16_t
db (((-2) as int8_t) as int + 5) as int8_t
```

```bytes
fe 12 34 03
```

## Types

### `int8_t`

`type int8_t = signed<8>`

An 8-bit signed value, -128 to 127.

### `uint8_t`

`type uint8_t = bits<8>`

An 8-bit unsigned value, 0 to 255.

### `int16_t`

`type int16_t = signed<16>`

A 16-bit signed value.

### `uint16_t`

`type uint16_t = bits<16>`

A 16-bit unsigned value.

### `int32_t`

`type int32_t = signed<32>`

A 32-bit signed value.

### `uint32_t`

`type uint32_t = bits<32>`

A 32-bit unsigned value.

### `int64_t`

`type int64_t = signed<64>`

A 64-bit signed value.

### `uint64_t`

`type uint64_t = bits<64>`

A 64-bit unsigned value.
