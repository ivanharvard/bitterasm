# Struct fields: `pub` and `skip`

Two keywords change how a field can be used: `pub` and `skip`.

| Field | Read or named from other files | Visited by `@for` and `in` |
|---|---|---|
| `name: T` | No | No |
| `pub name: T` | Yes | Yes |
| `pub skip name: T` | Yes | No |

Every field is part of the value either way: all of them are emitted, and
all of them are packed by `bitter`.

## `pub`: visibility

A field without `pub` is private to the file (module) that declares the
struct. Other files can't read it with `.field`, or supply it by name when
constructing the struct.

Say `shapes.basm` declares a struct with one public and one private field:

```basm,file=shapes.basm
pub struct Circle {
    pub radius: int,
    area_cache: int,
}

pub macro circle(r: int) -> Circle {
    @return Circle(r, 3 * r * r)
}
```

Another file can read `radius`, but not `area_cache`:

```basm
from .shapes import circle

macro show(value: int) {
    @emit value
}

show circle(2).radius
```

```emits
2
```

```basm,fail
from .shapes import circle

macro show(value: int) {
    @emit value
}

show circle(2).area_cache
```

```error
field `area_cache` of `Circle` is private to the module that declared it
```

A few things still work with private fields:

- **Positional construction**, like `Circle(2, 12)`, from any file, since it
  names no fields.
- **The struct's own code**, meaning its [invariants](invariants.md), field
  defaults and [conversions](conversions.md). These always run as part of the
  declaring file, whoever triggered them.

## `pub`: iteration

[`@for x in value`](../meta/for.md) and [`x in value`](../basics/ranges.md)
only see `pub` fields. A private field is treated as internal bookkeeping,
not as one of the struct's elements.

## `skip`

`pub skip` marks a field that's fully public, but isn't one of the
struct's elements. `@for` and `in` pass over it.

`std.array`'s `Array<T, N>` is the example. Its `len` must be readable by
anyone, but a loop over an array should see only its elements:

```basm
pub struct Array<T, const N: int>
    | invariant N >= 0
{
    @for i in 0..N {
        pub __el`i`: T,
    }

    pub skip len: int = N,
}

macro walk<const N: int>(arr: Array<int, N>) {
    @for x in arr {
        @emit x
    }
    @emit arr.len
}

walk Array<int, 2> { __el0: 7, __el1: 8 }
```

```emits
7 8 2
```

Without `skip`, the loop would visit `len` as a third element. String
literals work the same way: their `len` is `pub skip`.

`skip` on a private field is allowed, but does nothing, since private fields
are already left out.
