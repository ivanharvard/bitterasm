# Generating docs

`bitterasm doc` turns a module's [doc comments](../basics/doc-comments.md)
into reference pages, and checks the examples in them. The
[std reference](../std/reference/index.md) is built by this command,
exactly as your own modules would be.

## Syntax

```sh
bitterasm doc <paths>... [-o <dir>]          # write pages (default: ./doc)
bitterasm doc <paths>... --summary SUMMARY.md # and list them in an mdBook
bitterasm doc <paths>... --test               # compile the examples in doc comments
bitterasm doc <paths>... -o <dir> --check     # fail if <dir> is out of date
```

Each path is a `.basm` file or a directory to search.

## Where pages go

The pages follow the modules' folders. The folder every module shares is
left off, so documenting `myarch` writes:

```text
doc/
  index.md          # myarch: what's in it, each with its summary
  util.md           # myarch.util
  x86/
    index.md        # myarch.x86's contents
    native.md       # myarch.x86.native
```

Each page's title is the module's own name, with the path to it underneath:
`myarch › x86 › native`, linking back to each folder. A module with a
folder of the same name next to it (`x86.basm` and `x86/`) becomes that
folder's `index.md`, followed by the folder's contents.

## Adding the pages to an mdBook

mdBook only shows pages its `SUMMARY.md` lists. Put two marker lines under
the entry the pages belong to:

```md
- [Reference](reference/index.md)
    <!-- bitterasm doc: begin -->
    <!-- bitterasm doc: end -->
```

and pass `--summary`. `bitterasm doc` replaces whatever is between the
markers with the pages, nested by folder and indented like the markers:

```sh
bitterasm doc myarch -o book/src/reference --summary book/src/SUMMARY.md
```

## What goes on a page

Almost everything on a page comes from the code. Doc comments only add the
prose.

- **Macros**, one section per name, with a row for each overload: how it's
  written, its parameters, and what it returns (`-> T`) or emits
  (`| emits T`).
- **Types**: structs with their `pub` fields, enums with their variants,
  and type aliases.
- **Constants** and `pub` **labels**.

The syntax column shows how a macro is written *in that module*. Suppose a
module declares an instruction and two dialects re-export it:

```basm,file=impl.basm
## Adds two registers.
pub macro add(rd: int, rs1: int, rs2: int) {
    @emit rd + rs1 + rs2
}
```

```basm,file=native.basm
pub from .impl import *
```

```basm,file=c_like.basm
pub from .impl import *

syntax add(rd, rs1, rs2) = { $rd$ = $rs1$ + $rs2$ }
```

The page for `native` shows `add rd, rs1, rs2`, and the page for `c_like`
shows `rd = rs1 + rs2`. Both work:

```basm
from .c_like import *

const x = 1
x = 2 + 3
```

```emits
6
```

A module's page includes everything it re-exports with `pub from`.
Re-exported macros are listed in full, since a dialect may spell them
differently, with a column saying which module declares each overload.
Re-exported types and constants read the same everywhere, so they're
listed by name with a link to their own module's page.

When a name has several overloads, each row shows the first paragraph of
that overload's doc, and any longer doc appears in full below the table.

## Testing examples

`--test` compiles every example in the doc comments of the given modules,
following the same rules as this book's examples (see
[Writing the docs](../contributing/writing-docs.md#code-blocks)). A fence
with no language is BitterASM, so an instruction's encoding can be checked
right where it's documented:

```basm,ignore
## Returns from the current function.
##
## ```
## from myarch.native import *
##
## ret
## ```
##
## ```bytes
## c3
## ```
pub macro ret() | emits Byte { ... }
```

Examples are compiled from the current directory, so they import your
modules the way a program would. Checking `bytes` needs `bitter`, found
next to `bitterasm` or on `PATH`. A failure names the example's line:

```text
error: myarch/native.basm:12: expected the bytes c3, but got c2
```

## Keeping pages up to date

`--check` writes nothing, and fails if any page in the output directory
differs from what `bitterasm doc` would write, or belongs to a module that
no longer exists. With `--summary`, it also fails if the list in
`SUMMARY.md` is out of date. Run it in CI next to `--test`:

```sh
bitterasm doc myarch --test
bitterasm doc myarch -o book/src/reference --summary book/src/SUMMARY.md --check
```

Writing never deletes anything: a page left over from a module that's gone
is pointed out, for you to delete.

To make sure everything public is documented, turn on the `missing_docs`
lint, which is allowed by default. It reports each `pub` item without a
`##` comment, and a file without a `#!` block. A macro counts as
documented when any of its overloads in that file is.

```sh
bitterasm check myarch/native.basm -W missing_docs
```
