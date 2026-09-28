# Macros

Macros are how BitterASM does everything. An instruction like `add a0, a1, a2`
is a call to a macro named `add`. A "pseudo-instruction" is a macro too,
and so is a helper that computes a value. The language draws no line between
them.

A macro runs **at compile time**. It can compute values, check conditions,
call other macros, and **emit** values into the program's output.

## Declaring a macro

```basm,ignore
macro name(param: Type, ...) -> ReturnType
    | facet ...
{
    body
}
```

- **Parameters** each have a name and a type. See
  [Parameters and defaults](parameters.md).
- **`-> ReturnType`** is optional. See [Returning values](returning.md).
- **Facets** (`| before ...`, `| syntax { ... }`, ...) are optional modifiers,
  one per line, before the body.
- **`pub macro`** lets other files import it. See
  [Visibility](../modules/visibility.md).

## Two ways to call a macro

**As a statement**, like an instruction: the name, then its arguments
separated by commas, with no parentheses. Whatever the macro emits goes into
the output.

**As an expression**, like a function: the name, then its arguments in
parentheses. The call's value is whatever the macro returns.

```basm
macro square(x: int) -> int {
    @return x * x
}

macro show(value: int) {
    @emit value
}

show 3              # statement call
show square(4)      # `square(4)` is an expression call
```

```emits
3 16
```

A statement call throws away the macro's return value, so a statement call
of `square` would do nothing. An expression call can't throw away emitted
values, so a macro that emits can only be called as an expression in a few
places. See [Where emitted values can go](returning.md#where-emitted-values-can-go).

A macro with no parameters is called by its name alone:

```basm
macro nop() {
    @emit 0x13
}

nop
nop
```

```emits
19 19
```

## Inside a macro

A macro's body is a list of statements:

- [Meta keywords](../meta/index.md) such as `@emit`, `@return`, `@if` and
  `@for`.
- Calls to other macros, which emit into this macro's output.
- `const` declarations naming intermediate values.
- Declarations that the macro generates when it's called. See
  [Generating declarations](generating.md).

```basm
macro show(value: int) {
    @emit value
}

macro countdown(start: int) {
    @for i in 0..start {
        show start - i
    }
    show 0
}

countdown 3
```

```emits
3 2 1 0
```

## In this chapter

- [Parameters and defaults](parameters.md)
- [Returning values](returning.md)
- [Overloading](overloading.md): several macros with one name.
- [Recursion](recursion.md): macros that call themselves.
- [Hooks: `before` and `after`](hooks.md): code that runs around a macro.
- [Custom syntax](syntax.md): calls shaped like `mov rax, [rbx]` or `x <- 5`.
- [Generating declarations](generating.md): macros that declare constants and
  types.

Macros can also be generic, and can take other macros as arguments. See
[Generics](../generics/index.md).
