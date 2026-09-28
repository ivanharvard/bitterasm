# Visibility and re-exports

## `pub`

A declaration without `pub` is private to its file. Put `pub` in front to
let other files import it:

```basm,ignore
pub const WIDTH = 8
pub macro nop() { ... }
pub struct Point { ... }
pub enum Endian { ... }
pub type Reg = bits<5>
pub start:                    # a label
```

Struct fields have their own `pub`. See [Struct fields](../types/fields.md).

```basm,file=lib.basm
pub const PUBLIC = 1
const PRIVATE = 2
```

```basm,fail
from .lib import PRIVATE
```

```error
has no `PRIVATE`
```

## Re-exporting with `pub from`

`pub from ... import` imports names *and* exports them again, as if this
file had declared them. That's how a dialect presents an instruction set
plus its own syntax as one module:

- `std.riscv.native` does `pub from .impl import *`, and adds
  conventional syntax.
- `std.x86_64.nasm` re-exports `std.x86_64.intel`, which re-exports
  `std.x86_64.impl`.

A program then imports just the dialect.

```basm,file=base.basm
pub const BASE = 100

pub macro show(value: int) {
    @emit value
}
```

```basm,file=extended.basm
pub from .base import *

pub const EXTRA = 5
```

```basm
from .extended import *

show BASE + EXTRA
```

```emits
105
```

## When names collide

**Your own declarations win.** A file's own declaration hides an imported
one with the same name. So adding a new `pub` name to a library can't break
a file that already uses that name.

```basm
from .base import show

const BASE = 7

show BASE
```

```emits
7
```

**Two imports of one name are ambiguous, but only if you use it.** If two
modules both export `BASE`, and a file imports both with `*`, using `BASE` is
an error that names both modules:

```basm,file=other.basm
pub const BASE = 200
```

```basm,fail
from .base import *
from .other import *

show BASE
```

```error
`BASE` is imported from more than one module
```

**Importing by name picks one.** A name imported by name takes precedence
over one brought in by `*`:

```basm
from .base import *
from .other import BASE

show BASE
```

```emits
200
```

**Macros are the exception.** Same-named macros from different modules don't
collide. Their overloads merge into one set, and each call picks the overload
that fits. See [Overloading](../macros/overloading.md#overloads-across-files).
