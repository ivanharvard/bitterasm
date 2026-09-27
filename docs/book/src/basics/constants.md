# Constants

`const` gives a value a name:

```basm
const WIDTH = 8
const MASK = (1 << WIDTH) - 1

macro show(value: int) {
    @emit value
}

show MASK
```

```emits
255
```

A constant can hold any value: an integer, a struct, an enum variant, or a
string.

## Syntax

```basm,ignore
const NAME = value
const NAME: Type = value
pub const NAME = value
```

- **`: Type`** converts the value to `Type`, and checks it, before binding
  it. See [Conversions](../types/conversions.md).
- **`pub`** lets other files import the constant. See
  [Visibility](../modules/visibility.md).

```basm
from std.binary import bits

const BYTE: bits<8> = 200

macro emit_byte(b: bits<8>) {
    @emit b
}

emit_byte BYTE
```

```emits
bits<8> { value: 200 }
```

A value that doesn't fit the type is a compile error. `bits<8>` checks that
its value fits in eight bits:

```basm,fail
from std.binary import bits

const BYTE: bits<8> = 300
```

```error
invariant `fits_inside_width(width, value)` was violated for `bits`
```

## Constants never change

There are no variables in BitterASM. A constant is bound once and never
reassigned, and nothing in the language can be mutated. To compute a value
step by step, use recursion or [`@fold`](../meta/fold.md).

## Constants inside macros

Inside a macro body, `const` names an intermediate value. It's visible for
the rest of that body:

```basm
macro hypot_squared(a: int, b: int) {
    const aa = a * a
    const bb = b * b
    @emit aa + bb
}

hypot_squared 3, 4
```

```emits
25
```

A `pub const` inside a macro is different: it declares a new top-level
constant when the macro is called. See
[Generating declarations](../macros/generating.md).

## Registers are constants

Architecture packages use constants for registers. For example,
`std.riscv.impl` declares each register as a constant of type `Reg`, a
five-bit number:

```basm,ignore
pub type Reg = bits<5>

@for i in 0..32 {
    pub const x`i` = Reg(i)
}

pub const zero = x0
pub const ra = x1
```

The `` x`i` `` builds the names `x0` to `x31`. See
[Spliced names](../splicing/names.md).
