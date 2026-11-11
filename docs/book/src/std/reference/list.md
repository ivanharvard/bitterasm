# `list`

[`std`](index.md) › `list`

Linked lists: a `List<T>` is either empty (`Nil`) or a value followed
by the rest of the list (`Cons`). Every value is immutable, so each
operation returns a new list.

```basm
from std.list import *

macro show_all<T>(xs: List<T>) {
    @match xs {
        Nil => {}
        Cons(node) => {
            @emit node.head
            show_all node.tail
        }
    }
}

macro show(value: int) {
    @emit value
}

struct Point {
    pub x: int,
    pub y: int,
}

const xs = list(1, list(2, list(3, nil<int>())))

show_all xs
show_all reversed(xs)
show_all struct_as_list<Point, int>(Point(4, 5))
show length(xs)
```

```emits
1 2 3
3 2 1
4 5
3
```

An empty list still has an element type, so `nil` takes it as a generic
argument: `nil<int>()`.

## Macros

### `list`

The list `head` followed by `tail`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `list(head, tail)` | `<T>`, `head: T`, `tail: List<T>` | returns `List<T>` |  |

### `nil`

The empty list of `T`s.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `nil()` | `<T>` | returns `List<T>` |  |

### `length`

How many values `xs` holds.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `length(xs)` | `<T>`, `xs: List<T>` | returns `int` |  |

### `reversed`

`xs` in reverse order.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `reversed(xs)` | `<T>`, `xs: List<T>` | returns `List<T>` |  |

### `struct_as_list`

The `pub` fields of `s`, a struct of type `S`, as a list in order. Every
one must be a `T`. Both types are named: `struct_as_list<Point, int>(p)`.

| Syntax | Parameters | Result | Description |
|---|---|---|---|
| `struct_as_list(s)` | `<S, T>`, `s: S` | returns `List<T>` |  |

## Types

### `Node`

`struct Node<T>`

A list's first value and the rest of the list.

| Field | Type | Description |
|---|---|---|
| `head` | `T` | The first value. |
| `tail` | `List<T>` | Everything after it. |

### `List`

`enum List<T>`

Either empty, or a value followed by the rest of the list.

| Variant | Payload | Description |
|---|---|---|
| `Nil` |  | The empty list. |
| `Cons` | `Node<T>` | A value followed by the rest of the list. |
