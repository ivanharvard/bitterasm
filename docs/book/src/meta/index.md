# Meta keywords

Words starting with `@` are **meta keywords**. They control what a macro
does while the compiler runs it: emit values, return, check conditions,
branch and loop. There are eight:

| Keyword | Does | Page |
|---|---|---|
| `@emit value` | Adds `value` to the program's output | [`@emit`](emit.md) |
| `@return [value]` | Ends the macro, optionally with a value | [`@return`](return.md) |
| `@assert cond[, "message"]` | Fails compilation if `cond` is false | [`@assert`](assert.md) |
| `@if cond { } @else { }` | Runs one branch | [`@if` and `@else`](if.md) |
| `@match value { pattern => { } }` | Runs the first arm that matches | [`@match`](match.md) |
| `@for x in source { }` | Runs the body once per element | [`@for`](for.md) |
| `@fold acc = init @for x in source { }` | A `@for` that carries values between iterations | [`@fold` and `@next`](fold.md) |
| `@next value` | Moves a `@fold` to its next iteration | [`@fold` and `@next`](fold.md) |

Nothing a meta keyword does survives into the output except what it emits.
There's no `@if` at run time on the target machine; that would be an
instruction, which is a macro some architecture package provides.

## Where they can be used

| | Macro body | Top level | Struct declaration | Struct construction |
|---|---|---|---|---|
| `@emit`, `@return`, `@assert` | ✓ | | | |
| `@if`, `@for`, `@fold`/`@next` | ✓ | ✓ | ✓ | ✓ |
| `@match` | ✓ | ✓ | | |

At the top level, `@for` and `@if` repeat or choose *statements*: they can
generate declarations and calls. In a struct declaration they choose
*fields*, and in a construction they choose *field values*. See
[Structs](../types/structs.md#generating-fields).

```basm
macro show(value: int) {
    @emit value
}

@for i in 0..3 {
    show i * i
}

@if 2 > 1 {
    show 100
}
```

```emits
0 1 4 100
```
