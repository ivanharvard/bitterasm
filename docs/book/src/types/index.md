# Types

BitterASM has one built-in data type, [`int`](../basics/integers.md). Two
more kinds of value exist only at compile time and can never be emitted: a
[label](../programs/labels.md), of type `label`, and a
[macro](../macros/macro-parameters.md), of a type like `macro(int) -> int`.
Every other type is declared in BitterASM code, usually in a library:

| Kind | Declared with | Example |
|---|---|---|
| [Struct](structs.md) | `struct` | an instruction format, `bits<N>`, `Array<T, N>` |
| [Enum](enums.md) | `enum` | `Endian { Little, Big }`, `Option<T>` |
| [Type alias](aliases.md) | `type` | `type Reg = bits<5>` |

Types can be [generic](../generics/index.md): `bits<8>` and `Array<int, 4>`
are instances of generic structs.

## Why types matter

Types exist only at compile time. The compiler uses them to:

- **Check arguments.** A macro that takes a `Reg` rejects anything that
  isn't a `Reg`.
- **Pick overloads.** `mov rax, rbx` and `mov rax, 5` call different `mov`
  macros because `rbx` and `5` have different types. See
  [Overloading](../macros/overloading.md).
- **Enforce rules.** An [invariant](invariants.md) such as "fits in 8 bits"
  or "is even" is checked every time a value of the type is made.
- **Describe output.** A struct's fields tell an evaluator like `bitter` how
  to lay out bits. See [Packing bytes with `bitter`](../output/bitter.md).

## Data types and compile-time types

Every data type is built from `int`. Data is anything that can end up in
the output: it can be emitted and packed into bits.

A few types describe compile-time machinery instead, for type safety and
more expressive macros. The first is the [macro type](../macros/macro-parameters.md),
such as `macro(int) -> int`. A value of one of these types can be passed to a
macro and named with `const`, but it is never data: it can't be emitted.

## No automatic conversions

A value never changes type on its own. To turn one type into another, convert
it explicitly with [`as`](conversions.md):

```basm
from std.binary import bits

macro emit_byte(b: bits<8>) {
    @emit b
}

emit_byte 65 as bits<8>
```

```emits
bits<8> { value: 65 }
```

## Equality

`==` and `!=` work on any two values: structs compare field by field, and
enums compare variant and payload.

## In this chapter

- [Structs](structs.md): declaring, constructing and reading them.
- [Struct fields: `pub` and `skip`](fields.md): visibility and iteration.
- [Enums](enums.md): a value that is one of several variants.
- [Type aliases](aliases.md): new names for types, and new types with rules.
- [Invariants](invariants.md): rules every value of a type must follow.
- [Conversions: `as`, `to` and `from`](conversions.md).
