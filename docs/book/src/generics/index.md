# Generics

A generic declaration takes **parameters in angle brackets**, so one
declaration covers a whole family of types or macros. `bits<8>` and
`bits<32>` are two types made from one generic struct, `bits<const width:
int>`.

## Two kinds of parameter

| Kind | Written | Stands for | Example |
|---|---|---|---|
| Type parameter | `T` | a type | `Pair<T>`, used as `Pair<int>` |
| Const parameter | `const N: int` | a value known at compile time | `bits<const width: int>`, used as `bits<8>` |

Structs, enums and macros can be generic. Type aliases currently can't.

## Generic structs

```basm
struct Pair<T> {
    pub a: T,
    pub b: T,
}

macro emit_pair(p: Pair<int>) {
    @emit p
}

emit_pair Pair<int> { a: 1, b: 2 }
```

```emits
Pair<int> { a: 1, b: 2 }
```

A generic struct is constructed with braces, naming its arguments:
`Pair<int> { ... }`. The `Name(...)` form is only for non-generic structs.

Generic enums work the same way. See [Enums](../types/enums.md#generic-enums).

## Generic macros

A generic macro's parameters are **inferred** from its arguments. You never
write them at the call:

```basm
struct Pair<T> {
    pub a: T,
    pub b: T,
}

macro first<T>(p: Pair<T>) -> T {
    @return p.a
}

macro show(value: int) {
    @emit value
}

show first(Pair<int> { a: 5, b: 6 })
```

```emits
5
```

Inside the macro, a const parameter is an ordinary value, and a type
parameter can be used anywhere a type can.

A generic overload loses to a non-generic one that also fits. See
[Overloading](../macros/overloading.md).

## In this chapter

- [Const parameters](const.md): values in types, like `bits<8>` and
  `Array<T, N + 1>`.
- [Wildcards: `...`](wildcards.md): accepting any argument without naming
  it.
- [Macros as parameters](macro-parameters.md): passing a macro to a macro.
