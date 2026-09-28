# Writing the docs

This book lives next to the compiler, so a change to the language and its
documentation land in the same pull request. Its examples are tests:
`cargo test --test book` compiles every one of them, so an example that
stops working fails CI instead of quietly going stale.

## How pages are organized

Each chapter starts with a general page: what the feature is for, and a
short example. Its subchapters go deep on one piece each. A subchapter
usually has:

1. A one-line summary.
2. A **Syntax** block, marked `ignore`.
3. A small, complete, tested example with its output.
4. Details, edge cases and errors, each with its own example where
   possible.

## Code blocks

| Fence | Meaning |
|---|---|
| ```` ```basm ```` | Must compile, with every lint except `generated_declarations` denied. |
| ```` ```basm,fail ```` | Must fail to compile. |
| ```` ```basm,ignore ```` | A fragment: highlighted, but not compiled. |
| ```` ```basm,file=name.basm ```` | Not compiled itself. Saved as `name.basm` next to the page's later examples, so they can import it with `from .name import ...`. |

Directly after a compiled `basm` block, any of these check its result:

| Fence | Checks |
|---|---|
| ```` ```emits ```` | The emitted values, in order. |
| ```` ```bytes ```` | The bytes `bitter encode` packs them into, in hex. |
| ```` ```error ```` | Text the compile error must contain (after `basm,fail`). |

In an `emits` block, integers can share a line, separated by spaces. Other
values go one per line, written as the test prints them:
`Point { x: 1, y: 2 }`, `bits<8> { value: 65 }`, `Shape.Circle(2)`,
`Word<16, 1> { ... }` (an enum argument is shown as its index), or
`<name>` for a label imported from another file. If you're unsure, write
your best guess: the test failure shows the actual output.

## Writing examples

Each `basm` block is compiled on its own, from the repository root, so it can
import `std`, and any `file=` blocks earlier on the same page.

Top-level `@emit` isn't allowed, so an example that shows values defines a
small macro to emit them:

```basm
macro show(value: int) {
    @emit value
}

show 6 * 7
```

```emits
42
```

Prefer a complete, checked example over an `ignore`d fragment. Keep `ignore`
for syntax summaries, and for code that can't stand alone.

When the language has a limitation or a known bug, say so in a note, and
show it with a `fail` example if you can. When the bug is fixed, the test
fails, which reminds you to update the page.

## Building

```sh
mdbook serve docs/book --open   # live preview
mdbook build docs/book          # writes docs/book/book
```

The introduction is included from the README's `overview` anchor, so edit it
there.

## The std reference

`std/reference/` is generated from std's doc comments, so don't edit it by
hand. Edit the `##` and `#!` comments in `std/`, then regenerate it:

```sh
bitterasm doc std -o docs/book/src/std/reference --summary docs/book/src/SUMMARY.md
```

That also updates the reference's entries in `SUMMARY.md`, so a new module
needs nothing else. `cargo test --test doc_comments` fails while either is
out of date.
