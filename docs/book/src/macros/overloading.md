# Overloading

Several macros can share a name, as long as their parameters differ. A call
picks the one whose parameter types and count fit its arguments:

```basm
struct Reg {
    pub n: int,
}

macro describe(r: Reg) {
    @emit 1000 + r.n
}

macro describe(v: int) {
    @emit v
}

describe Reg(3)
describe 3
```

```emits
1003 3
```

This is how instruction sets handle operand forms: `std.x86_64` has one `mov`
for register-to-register, another for an immediate, another for memory, and
so on, each with its own encoding.

## How a call picks an overload

1. Only overloads that accept the number of arguments are considered,
   counting [defaults](parameters.md#defaults).
2. Of those, only overloads whose parameter types match the arguments are
   considered.
3. A non-generic overload beats a [generic](../generics/index.md) one.
4. If more than one candidate is left, the call is ambiguous, which is an
   error.

```basm
macro kind<T>(_x: T) {
    @emit 1
}

macro kind(_x: int) {
    @emit 2
}

kind 5
```

```emits
2
```

```basm,fail
macro g(_a: int, _b: int = 0) {
    @emit 3
}

macro g(_a: int) {
    @emit 4
}

g 1
```

```error
multiple overloads of `g` accept (int)
```

## Overloads across files

A file's overloads merge with same-named overloads it imports, from any
number of modules. That's how a dialect adds new operand forms to an
instruction it builds on: `std.x86_64.nasm` adds `mov rax, [rel label]`
alongside every `mov` from `std.x86_64.intel`. See
[Imports](../modules/imports.md).
