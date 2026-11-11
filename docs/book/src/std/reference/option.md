# `option`

[`std`](index.md) › `option`

`Option<T>`: a value that may be missing.

## Macros

### `is_some`

`1` if `o` holds a value, otherwise `0`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `is_some(o)` | `<T>`, `o: Option<T>` | returns `int` |  |

### `is_none`

`1` if `o` is `None`, otherwise `0`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `is_none(o)` | `<T>`, `o: Option<T>` | returns `int` |  |

### `unwrap`

The value in `o`. `None` is a compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `unwrap(o)` | `<T>`, `o: Option<T>` | returns `T` |  |

### `unwrap_or`

The value in `o`, or `default` if it's `None`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `unwrap_or(o, default)` | `<T>`, `o: Option<T>`, `default: T` | returns `T` |  |

### `mapped`

`f` of the value in `o`, or `None`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mapped(o, f)` | `<T, U>`, `o: Option<T>`, `f: macro(T) -> U` | returns `Option<U>` |  |

## Types

### `Option`

`enum Option<T>`

Either `Some` value of type `T`, or `None`.

| Variant | Payload | Description |
|---|---|---|
| `Some` | `T` | A value. |
| `None` |  | No value. |
