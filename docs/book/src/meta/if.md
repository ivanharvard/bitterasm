# `@if` and `@else`

`@if` runs its body only when a condition is true. An optional `@else` body
runs otherwise.

## Syntax

```basm,ignore
@if condition {
    ...
}

@if condition {
    ...
} @else {
    ...
}
```

`@else` goes on the same line as the closing `}`.

## Example

```basm
macro abs(x: int) -> int {
    @if x < 0 {
        @return -x
    } @else {
        @return x
    }
}

macro show(value: int) {
    @emit value
}

show abs(-4)
show abs(4)
```

```emits
4 4
```

## More than two branches

There's no `@else @if`. Nest another `@if` inside the `@else`, or use
[`@match`](match.md):

```basm
macro sign(x: int) {
    @if x < 0 {
        @emit -1
    } @else {
        @if x == 0 {
            @emit 0
        } @else {
            @emit 1
        }
    }
}

sign -5
sign 0
sign 9
```

```emits
-1 0 1
```

## Choosing an encoding

`@if` is how an instruction picks between encodings. For example, it might
use a short form when an immediate fits:

```basm
macro load_imm(value: int) {
    @if value in -2048..2048 {
        @emit 1    # one instruction
    } @else {
        @emit 2    # two instructions
    }
}

load_imm 100
load_imm 100_000
```

```emits
1 2
```

The choice is explicit and visible in the macro. BitterASM never swaps in a
"better" encoding on its own.

## Outside macros

At the top level, `@if` includes or skips statements, including
declarations:

```basm
const DEBUG = 1

macro show(value: int) {
    @emit value
}

@if DEBUG {
    show 0xdeb
}
```

```emits
3563
```

In a struct declaration or construction, it includes or skips fields. See
[Structs](../types/structs.md#generating-fields).
