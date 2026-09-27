# Hooks: `before` and `after`

A macro can declare calls that run every time it's called: `before` hooks
run first, then the body, then `after` hooks.

```basm
macro show(value: int) {
    @emit value
}

macro traced(x: int) -> int
    | before show(100)
    | after show(result.returned)
{
    @return x + 1
}

traced 1
```

```emits
100 2
```

## Syntax

```basm,ignore
macro name(params)
    | before hook_call(params...)
    | after hook_call(params..., result.returned)
{
    body
}
```

- A hook is a macro call. It can use the macro's parameters.
- In an `after` hook, `result.returned` is the value the body returned.
- A macro can have any number of each. They run in the order they're
  written.
- Anything a hook emits goes into the output, like the body's own emits.

## Checking arguments

The most common use is a shared check. `std.array` checks every index once,
in a hook, rather than in each macro's body:

```basm
macro oob_check(len: int, index: int) {
    @assert index >= 0, "negative index"
    @assert index < len, "index past the end"
}

macro element_offset(len: int, index: int) -> int
    | before oob_check(len, index)
{
    @return index * 4
}

macro show(value: int) {
    @emit value
}

show element_offset(8, 3)
```

```emits
12
```

## Hooks compose

A hook is an ordinary macro call, so if the hook macro has hooks of its own,
they run too. Checks built from hooks keep working however deep the call
chain goes.

Hooks change when a macro can [tail-call](recursion.md#limits) itself: a
macro with an `after` hook never does, because the hook has to run after
each call returns.
