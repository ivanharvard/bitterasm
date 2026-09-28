# Enums

An enum is a value that is exactly one of a fixed list of **variants**.

## Declaring an enum

```basm,ignore
enum Name {
    Variant,
    Variant: PayloadType,
}
```

A variant can carry one value of a given type, its **payload**, or nothing.
`pub enum` lets other files import it.

```basm
enum Endian {
    Little,
    Big,
}

enum Operand {
    Register: int,
    Immediate: int,
    None,
}

macro emit_it(o: Operand) {
    @emit o
}

emit_it Operand.Register(3)
emit_it Operand.None
```

```emits
Operand.Register(3)
Operand.None
```

## Making a value

Write the enum's name, a dot and the variant. Add the payload in
parentheses:

| Expression | Value |
|---|---|
| `Endian.Big` | the `Big` variant |
| `Operand.Immediate(42)` | the `Immediate` variant, carrying `42` |

## Using a value

Compare with `==`, or take it apart with [`@match`](../meta/match.md):

```basm
enum Operand {
    Register: int,
    Immediate: int,
    None,
}

macro describe(o: Operand) {
    @match o {
        Register(n) => { @emit 100 + n }
        Immediate(v) => { @emit v }
        None => { @emit -1 }
    }
}

describe Operand.Register(3)
describe Operand.Immediate(42)
describe Operand.None
```

```emits
103 42 -1
```

## Generic enums

An enum can be [generic](../generics/index.md). `std.option` declares:

```basm,ignore
pub enum Option<T> {
    Some: T,
    None
}
```

Write the type arguments when making a value: `Option<int>.Some(42)` and
`Option<int>.None`. They aren't inferred:

```basm,fail
from std.option import Option

macro emit_option(o: Option<int>) {
    @emit o
}

emit_option Option.Some(42)
```

```error
`Option` expects 1 generic argument(s), but 0 were supplied
```

## Enums as settings

An enum can be a [const generic parameter](../generics/const.md), which
makes it a good fit for options that are fixed at compile time. `std.string`
uses `Endian` this way: `Utf8String<5, Endian.Little>`.

## Enums and output

An evaluator decides what an emitted enum means. `bitter` has no layout for
enums in general, so emitting one to `bitter` is an error, with one
exception: `std.bitter.deferred`'s `Deferred`, which it resolves to a number.
To put an enum in machine code, convert it to a `bits<N>` first.
