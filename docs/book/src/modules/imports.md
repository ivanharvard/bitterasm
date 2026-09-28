# Imports

## Syntax

```basm,ignore
from module import *              # every `pub` name in the module
from module import a, b, c        # only these names
pub from module import ...        # import, and re-export
```

```basm,ignore
from std.binary import *
from std.string import validate_ascii, string_from_struct
from .helpers import Pair
```

## Importing names

`import *` brings in every `pub` declaration of the module, except `pub`
labels, which must be imported by name (see
[Labels](../programs/labels.md#labels-across-files)).

Listing names imports just those:

```basm,file=consts.basm
pub const WIDTH = 8
pub const HEIGHT = 4
```

```basm
from .consts import WIDTH

macro show(value: int) {
    @emit value
}

show WIDTH
```

```emits
8
```

Importing a name the module doesn't export, or that isn't `pub`, is an
error. An import that's never used gets the `unused_import` warning.

## Importing a directory

If the module path names a directory, the listed names are modules in it:
`from std.riscv import native` means `from std.riscv.native import *`.

```basm
from std.riscv import native

add a0, a1, a2
```

```bytes
33 85 c5 00
```

## The search path

A relative path starts with dots. One dot is the importing file's own
directory, and each extra dot goes up one more:

| Path | Found in |
|---|---|
| `.helpers` | the importing file's directory |
| `.lib.helpers` | its `lib` subdirectory |
| `..helpers` | its parent directory |
| `...helpers` | two directories up |

Here `lib/double.basm` imports from its parent directory:

```basm,file=numbers.basm
pub const BASE = 21
```

```basm,file=lib/double.basm
from ..numbers import BASE

pub const DOUBLED = BASE * 2
```

```basm
from .lib.double import DOUBLED

macro show(value: int) {
    @emit value
}

show DOUBLED
```

```emits
42
```

An absolute path (`std.binary`) is looked up in these directories, in order:

1. The current directory.
2. Each directory in `BITTERASM_PATH`, which is separated like `PATH`.
3. `~/.bitterasm`, where the installer puts `std`.

So a project can override a library by putting its own copy earlier in the
search path.

## Imports aren't passed on

If `a` imports `b`, a file that imports `a` doesn't see `b`'s names. It
must import `b` itself, unless `a` re-exports them with `pub from`. See
[Visibility and re-exports](visibility.md).

Imports only affect names. `a`'s macros still use `b` however they're
called, because a name always means what it meant in the file that wrote it.

## What an import brings with it

Importing a macro also brings:

- **Its overloads.** An imported macro's overloads merge with same-named
  overloads from other imports and from the importing file. See
  [Overloading](../macros/overloading.md#overloads-across-files).
- **Its syntax.** A [custom syntax](../macros/syntax.md) assigned to the macro
  applies in the importing file too.
