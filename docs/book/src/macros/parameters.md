# Parameters and defaults

## Parameters

Every parameter has a name and a type:

```basm
macro show_sum(a: int, b: int) {
    @emit a + b
}

show_sum 2, 3
```

```emits
5
```

Arguments are matched to parameters by position. Named arguments like
`f(b = 1)` are only for constructing [structs](../types/structs.md), not for
calling macros.

## Arguments are type-checked

Each argument must already have its parameter's type. Nothing is converted
automatically, even when a conversion exists:

```basm,fail
from std.binary import bits

macro emit_byte(b: bits<8>) {
    @emit b
}

emit_byte 65
```

```error
type mismatch for `b`: expected `bits<8>`, found `int`
```

Convert explicitly with [`as`](../types/conversions.md):

```basm
from std.binary import bits

macro emit_byte(b: bits<8>) {
    @emit b
}

emit_byte 65 as bits<8>
```

```emits
bits<8> { value: 65 }
```

## Defaults

Parameters at the end of the list can have a default value, used when the
call leaves them out:

```basm
macro encode(value: int, width: int = 8, mask: int = (1 << width) - 1) {
    @emit value & mask
}

encode 0x1ff
encode 0x1ff, 4
```

```emits
255 15
```

- A default can use earlier parameters. Defaults are evaluated left to right
  at each call, so `mask` above sees whichever `width` the call ended up with.
- Once one parameter has a default, every parameter after it needs one too.

## Unused parameters

An unused parameter triggers the `unused_parameter` warning. If it's unused
on purpose, for example because only its type matters for
[overloading](overloading.md), start its name with an underscore:

```basm
macro kind(_value: int) {
    @emit 1
}

kind 42
```

```emits
1
```
