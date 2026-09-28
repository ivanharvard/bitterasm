# `deferred`

[`std`](../index.md) › [`bitter`](index.md) › `deferred`

Values that depend on where things end up, such as the distance to a
branch target. They're built as `Deferred` expressions that `bitter`
resolves once the image is laid out, and given a width by
`Positioned<N>`.

```basm
from std.binary import bits
from std.bitter.deferred import *

macro db(value: int) {
    @emit value as bits<8>
}

macro offset_to(target: int) {
    @emit Positioned<8> { value: span(here(), target) }
}

offset_to end
db 0xAA
db 0xBB
end:
```

```bytes
03 aa bb
```

`add`, `sub`, `mul`, `shr` and `band` work on `int`s, giving an `int`,
and on `Deferred`s, giving a `Deferred`, so an encoder is written the
same way whether its operand is known yet or not.

## Macros

### `here`

The position of the value being packed. Use it only inside the `span`
of the value being emitted: stored and used by a different value, it
refers to that value's position instead.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `here()` |  | returns `Deferred` |  |

### `add`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `add(a, b)` | `a: int`, `b: int` | returns `int` | `a + b`: an `int` when both are `int`s, else a `Deferred`. |
| `add(a, b)` | `a: Deferred`, `b: int` | returns `Deferred` |  |
| `add(a, b)` | `a: int`, `b: Deferred` | returns `Deferred` |  |
| `add(a, b)` | `a: Deferred`, `b: Deferred` | returns `Deferred` |  |

### `sub`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sub(a, b)` | `a: int`, `b: int` | returns `int` | `a - b`: an `int` when both are `int`s, else a `Deferred`. |
| `sub(a, b)` | `a: Deferred`, `b: int` | returns `Deferred` |  |
| `sub(a, b)` | `a: int`, `b: Deferred` | returns `Deferred` |  |
| `sub(a, b)` | `a: Deferred`, `b: Deferred` | returns `Deferred` |  |

### `mul`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mul(a, b)` | `a: int`, `b: int` | returns `int` | `a * b`: an `int` when both are `int`s, else a `Deferred`. |
| `mul(a, b)` | `a: Deferred`, `b: int` | returns `Deferred` |  |
| `mul(a, b)` | `a: int`, `b: Deferred` | returns `Deferred` |  |
| `mul(a, b)` | `a: Deferred`, `b: Deferred` | returns `Deferred` |  |

### `shr`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `shr(a, b)` | `a: int`, `b: int` | returns `int` | `a >> b`: an `int` when `a` is an `int`, else a `Deferred`. |
| `shr(a, b)` | `a: Deferred`, `b: int` | returns `Deferred` |  |

### `band`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `band(a, b)` | `a: int`, `b: int` | returns `int` | `a & b`: an `int` when `a` is an `int`, else a `Deferred`. |
| `band(a, b)` | `a: Deferred`, `b: int` | returns `Deferred` |  |

### `span`

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `span(start, end)` | `start: int`, `end: int` | returns `Deferred` | The number of bytes from position `start` to position `end`, negative if `end` comes first. Either may be `here()`. Labels are positions, so `span(here(), target)` is a relative branch offset. |
| `span(start, end)` | `start: Deferred`, `end: int` | returns `Deferred` |  |
| `span(start, end)` | `start: int`, `end: Deferred` | returns `Deferred` |  |

## Types

### `Op`

`enum Op`

The operation of a `BinOp`.

| Variant | Payload | Description |
|---|---|---|
| `Add` |  | `left + right`. |
| `Sub` |  | `left - right`. |
| `Mul` |  | `left * right`. |
| `Shr` |  | `left >> right`. |
| `Band` |  | `left & right`. |
| `Span` |  | Bytes from position `left` to position `right`, as in `span`. |

### `BinOp`

`struct BinOp`

An operation on two `Deferred` values.

| Field | Type | Description |
|---|---|---|
| `op` | `Op` | What to do. |
| `left` | `Deferred` | The first operand. |
| `right` | `Deferred` | The second operand. |

### `Deferred`

`enum Deferred`

A value `bitter` works out after laying out the image. Build one with
`here`, `span` and the arithmetic macros rather than by hand.

| Variant | Payload | Description |
|---|---|---|
| `Leaf` | `int` | A plain number. |
| `Pos` | `int` | A position, such as a label, as a number of entries. |
| `Here` |  | The position of the value being packed. |
| `Node` | `BinOp` | An operation on two `Deferred` values. |

### `Positioned`

`struct Positioned<const width: int>`

A `Deferred` value, packed into `width` bits once `bitter` resolves it:
what `bits<N>` is to an `int`. A negative value is packed in two's
complement.

| Field | Type | Description |
|---|---|---|
| `value` | `Deferred` | The value to resolve. |
