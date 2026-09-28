# `byte_order`

[`std`](../index.md) › [`bitter`](index.md) › `byte_order`

Byte order for `bitter`. Its default is most significant byte first;
`LittleEndian` asks for the reverse.

```basm
from std.binary import bits
from std.bitter.byte_order import LittleEndian

macro dw_le(value: int) {
    @emit LittleEndian<bits<16>, 16> { value: value as bits<16> }
}

dw_le 0x1234
```

```bytes
34 12
```

## Types

### `LittleEndian`

`struct LittleEndian<T, const width: int>`

`value`, packed with its least significant byte first. `width` is
`value`'s width in bits, and must be a multiple of 8.

| Field | Type | Description |
|---|---|---|
| `value` | `T` | The value to reverse, such as a whole instruction. |
