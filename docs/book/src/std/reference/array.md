# `array`

[`std`](index.md) › `array`

`Array<T, N>`: `N` values of type `T`, and ways to build new arrays from
old ones. Every value is immutable, so each operation returns a new array.

```basm
from std.array import *

macro show_all<const N: int>(values: Array<int, N>) {
    @for v in values {
        @emit v
    }
}

macro double(x: int) -> int {
    @return x * 2
}

const values = Array<int, 3>(10, 20, 30)

show_all reversed(values)
show_all mapped(appended(values, 40), double)
```

```emits
30 20 10
20 40 60 80
```

An array is built from its elements in order, as above, or by an `@for`
inside a brace construction, which names them `__el0`, `__el1`, ....
`@for v in values` visits the elements in order.

## Macros

### `get`

The element at `index`, counting from 0. An index outside the array is
a compile error.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `get(arr, index)` | `<T>`, `arr: Array<T, ...>`, `index: int` | returns `T` |  |

### `updated`

A copy of `arr` with the element at `index` replaced by `value`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `updated(arr, index, value)` | `<T>`, `arr: Array<T, ...>`, `index: int`, `value: T` | returns `Array<T, ...>` |  |

### `reversed`

A copy of `arr` in reverse order.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `reversed(arr)` | `<T>`, `arr: Array<T, ...>` | returns `Array<T, ...>` |  |

### `popped`

A copy of `arr` without its last element. `arr` can't be empty.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `popped(arr)` | `<T, const N: int>`, `arr: Array<T, N>` | returns `Array<T, (N - 1)>` |  |

### `first`

The first element. `arr` can't be empty.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `first(arr)` | `<T>`, `arr: Array<T, ...>` | returns `T` |  |

### `last`

The last element. `arr` can't be empty.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `last(arr)` | `<T>`, `arr: Array<T, ...>` | returns `T` |  |

### `mapped`

A new array holding `f` of each element, in order. `f` is any macro
that takes a `T` and returns a `U`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `mapped(arr, f)` | `<T, U>`, `arr: Array<T, ...>`, `f: macro(T) -> U` | returns `Array<U, ...>` |  |

### `array_from_struct`

An `Array<int, N>` of a struct's `__el0`..`__el{len - 1}` fields, such
as a `String`'s characters or a `Range`'s values.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `array_from_struct source` | `<S>`, `source: S` |  |  |

### `enumerate`

Each element paired with its index, as an `Enumerated<T>`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `enumerate(arr)` | `<T>`, `arr: Array<T, ...>` | returns `Array<Enumerated<T>, ...>` |  |

### `appended`

A copy of `arr` with `value` added at the end.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `appended(arr, value)` | `<T, const N: int>`, `arr: Array<T, N>`, `value: T` | returns `Array<T, (N + 1)>` |  |

### `find`

Iterates over `arr`'s elements in order until it finds the first instance of
`value`, then returns its index. If `value` isn't found, returns `-1`. If
`arr` is empty, errors at compile time.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `find(arr, value)` | `<T>`, `arr: Array<T, ...>`, `value: T` | returns `int` |  |

### `contains`

Returns `1` if `value` is in `arr`, otherwise returns `0` or errors at
compile time if `arr` is empty.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `contains(arr, value)` | `<T>`, `arr: Array<T, ...>`, `value: T` | returns `int` |  |

### `remove`

A copy of `arr` with the first instance of `value` removed. If `value`
isn't found, returns a copy of `arr`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `remove(arr, value)` | `<T, const N: int>`, `arr: Array<T, N>`, `value: T` | returns `Array<T, ...>` |  |

### `filled`

`N` copies of `value`. Name both types: `filled<int, 4>(0)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `filled(value)` | `<T, const N: int>`, `value: T` | returns `Array<T, N>` |  |

### `concatenated`

`a`'s elements, then `b`'s.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `concatenated(a, b)` | `<T, const M: int, const N: int>`, `a: Array<T, M>`, `b: Array<T, N>` | returns `Array<T, (M + N)>` |  |

### `sum`

The sum of `arr`'s elements, 0 for an empty array.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sum(arr)` | `arr: Array<int, ...>` | returns `int` |  |

### `any`

`1` if `f` of any element is nonzero, otherwise `0`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `any(arr, f)` | `<T>`, `arr: Array<T, ...>`, `f: macro(T) -> int` | returns `int` |  |

### `all`

`1` if `f` of every element is nonzero, otherwise `0`. An empty array
gives `1`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `all(arr, f)` | `<T>`, `arr: Array<T, ...>`, `f: macro(T) -> int` | returns `int` |  |

### `sorted`

A copy of `arr` with its elements in increasing order. Equal elements
keep their order.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `sorted(arr)` | `arr: Array<int, ...>` | returns `Array<int, ...>` |  |

## Types

### `Array`

`struct Array<T, const N: int>`

`N` values of type `T`, in order.

| Field | Type | Description |
|---|---|---|
| `len` | `int` | The number of elements, `N`. `skip`, so `@for` doesn't visit it. |

Some of its fields are generated by `@for` or `@if`.
