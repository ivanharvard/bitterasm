# Doc comments

`##` documents the item below it, and `#!` documents the whole file. The
text is Markdown.

## Syntax

```basm,ignore
#! What this file is for.

## What this item does.
item
```

## Example

```basm
#! Helpers for showing values.

## Emits `value` unchanged.
##
## ```
## show 7
## ```
macro show(value: int) {
    @emit value
}

show 7
```

```emits
7
```

Doc comments are for the people who use your code. Plain `#` comments are
for the people who maintain it, and are never part of the docs. The two mix
freely:

```basm
## Emits `value` twice.
# Written as two `@emit`s rather than a loop, to keep the expansion short.
macro twice(value: int) {
    @emit value
    @emit value
}

twice 3
```

```emits
3 3
```

## What can be documented

A `##` block documents the next item, as long as only blank lines and `#`
comments sit in between. An item is any of:

- a `macro`, `struct`, `enum`, `type` alias or `const`
- a label
- a `syntax` line
- a struct field or an enum variant

```basm
## A 2D point.
struct Point {
    ## Distance from the left edge.
    pub x: int,
    ## Distance from the top edge.
    pub y: int,
}

## Where a program starts.
enum Entry {
    ## At the first instruction.
    Start,
    ## At an offset from it.
    Offset: int,
}

macro show(value: int) {
    @emit value
}

show Point { x: 1, y: 2 }.y
```

```emits
2
```

An item inside a block is documented the same way, such as each constant
a top-level `@for` generates:

```basm
@for i in 0..4 {
    ## Register `i`.
    pub const r`i` = i
}

macro show(value: int) {
    @emit value
}

show r3
```

```emits
3
```

`#!` lines only document the file when they come before its first
statement.

To turn doc comments into reference pages, see
[Generating docs](../tools/docs.md).

## Where doc comments are ignored

A doc comment that doesn't document anything is reported by the
`unused_doc_comments` lint. That happens when:

- a `##` sits above something that isn't an item, such as an invocation
  or an `@emit`
- a `##` is at the end of a file or a block
- a `#!` comes after the file's first statement

```basm,fail
macro show(value: int) {
    @emit value
}

## Shows seven.
show 7
```

```error
doc comment documents nothing
```

Use `#` for an ordinary comment.

`###` and longer runs of `#` are ordinary comments too, so banners such as
`#### Registers ####` never end up in the docs.

## Formatting

`bitterasm fmt` keeps each line's `##` or `#!` marker. It wraps prose that
runs past `comment_width`, but leaves code blocks, tables and headings
exactly as written.
