# `@match`

`@match` compares a value against a list of patterns and runs the first arm
that matches.

## Syntax

```basm,ignore
@match value {
    pattern => { ... }
    pattern => { ... }
    _ => { ... }
}
```

Arms are tried from top to bottom. `_` matches anything. If no arm matches,
nothing happens. Commas between arms are optional.

## Matching values

A pattern that's an ordinary expression matches if it equals the value:

```basm
macro name_length(n: int) {
    @match n {
        0 => { @emit 4 }        # "zero"
        1 => { @emit 3 }        # "one"
        1 + 1 => { @emit 3 }    # "two"
        _ => { @emit -1 }
    }
}

name_length 0
name_length 2
name_length 7
```

```emits
4 3 -1
```

## Matching enums

Matching is most useful with [enums](../types/enums.md). Name a variant by
itself, or qualified by its enum:

```basm
enum Color {
    Red,
    Green,
    Blue,
}

macro code(c: Color) {
    @match c {
        Red => { @emit 1 }
        Color.Green => { @emit 2 }
        _ => { @emit 3 }
    }
}

code Color.Red
code Color.Green
code Color.Blue
```

```emits
1 2 3
```

For a variant with a payload, `Variant(name)` binds the payload to `name` for
that arm. `Variant(_)` matches any payload without binding it.

```basm
enum Shape {
    Circle: int,
    Square: int,
    Empty,
}

macro area(s: Shape) {
    @match s {
        Shape.Circle(r) => { @emit 3 * r * r }
        Square(w) => { @emit w * w }
        Empty => { @emit 0 }
    }
}

area Shape.Circle(2)
area Shape.Square(5)
area Shape.Empty
```

```emits
12 25 0
```

`Variant(expression)`, where the expression isn't a plain name, matches only
when the payload equals it. Qualified forms work the same way:
`Shape.Circle(r)`, and for a generic enum, `Option<int>.Some(v)`.

A pattern that isn't a variant, such as a constant holding an enum value, is
compared with `==`:

```basm
enum Color {
    Red,
    Green,
    Blue,
}

const FAVORITE = Color.Blue

macro is_favorite(c: Color) {
    @match c {
        FAVORITE => { @emit 1 }
        _ => { @emit 0 }
    }
}

is_favorite Color.Blue
is_favorite Color.Red
```

```emits
1 0
```

## Returning from a match

`@return` inside an arm returns from the whole macro:

```basm
from std.option import Option

macro unwrap_or(o: Option<int>, fallback: int) -> int {
    @match o {
        Option<int>.Some(v) => { @return v }
        None => { @return fallback }
    }
}

macro show(value: int) {
    @emit value
}

show unwrap_or(Option<int>.Some(42), 0)
show unwrap_or(Option<int>.None, 7)
```

```emits
42 7
```
