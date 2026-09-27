# Type aliases

`type` declares a new name for a type.

## Syntax

```basm,ignore
type Name = ExistingType
type Name = ExistingType
    | invariant condition
```

`pub type` lets other files import it.

## Plain aliases: just a new name

Without an invariant, an alias is simply another name for its type. The two
are interchangeable:

```basm
from std.binary import bits

type Reg = bits<5>

macro use_reg(r: Reg) {
    @emit r
}

use_reg Reg(3)           # construct through the alias
use_reg 4 as bits<5>     # a bits<5> is a Reg
```

```emits
bits<5> { value: 3 }
bits<5> { value: 4 }
```

Architecture packages use plain aliases to make their types
self-documenting. In `std.riscv.impl`, `Reg`, `Opcode`, `Funct3` and `Imm12`
are all aliases of `bits<N>`.

## Aliases with invariants: a new type

With an [invariant](invariants.md), an alias becomes a **distinct type**
with a rule. A value only becomes one through [`as`](conversions.md), which
checks the rule:

```basm
from std.binary import bits

type EvenByte = bits<8>
    | invariant v % 2 == 0

macro use_even(b: EvenByte) {
    @emit b
}

use_even 6 as EvenByte
```

```emits
bits<8> { value: 6 }
```

A plain `bits<8>` isn't an `EvenByte`, even if its value is even, because
nothing checked it:

```basm,fail
from std.binary import bits

type EvenByte = bits<8>
    | invariant v % 2 == 0

macro use_even(b: EvenByte) {
    @emit b
}

use_even 6 as bits<8>
```

```error
type mismatch for `b`: expected `EvenByte`, found `bits`
```

And `as` rejects values that break the rule:

```basm,fail
from std.binary import bits

type EvenByte = bits<8>
    | invariant v % 2 == 0

macro use_even(b: EvenByte) {
    @emit b
}

use_even 7 as EvenByte
```

```error
was violated for `EvenByte`
```

### Naming the value in an invariant

In an alias's invariant, the value being checked can have any name that
isn't already declared: `v` above, `x` in `std.ctypes`. The compiler takes the
one free name in the condition to mean the value. Using two different free
names is an error.

### Aliases of `int`

An alias of `int` with an invariant, like `std.unsigned`'s `uint`, checks
its rule on `as`. But a plain integer can't remember that it was checked, so
the result is an ordinary `int`. As a result, a parameter typed `uint` can't
be satisfied by anything:

```basm,fail
from std.unsigned import uint

macro f(x: uint) {
    @emit x
}

f 5 as uint
```

```error
type mismatch for `x`: expected `uint`, found `int`
```

Use an alias of `int` for its check (`const n = x as uint`), and type
parameters as `int`. Aliases of structs don't have this limit.

## Layers

An alias can be built on another type that has rules of its own. Converting
with `as` checks every layer, from the outside in. `std.ctypes` declares
`uint8_t` as a `bits<8>` whose value is at least zero, so `as uint8_t` checks
`x >= 0`, then `bits<8>`'s own rule that the value fits in eight bits:

```basm
from std.ctypes import *

macro emit_byte(b: uint8_t) {
    @emit b
}

emit_byte 200 as uint8_t
```

```emits
bits<8> { value: 200 }
```

```basm,fail
from std.ctypes import *

macro emit_byte(b: uint8_t) {
    @emit b
}

emit_byte (-1) as uint8_t
```

```error
invariant `(x >= 0)` was violated for `uint8_t`
```

Note the parentheses in `(-1) as uint8_t`. `as` binds more tightly than `-`,
so `-1 as uint8_t` would mean `-(1 as uint8_t)`.
