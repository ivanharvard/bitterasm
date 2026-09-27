# Conversions: `as`, `to` and `from`

Values never change type on their own. `as` converts a value explicitly:

```basm,ignore
value as Type
```

`as` binds more tightly than any operator, so put a compound expression in
parentheses: `(a + b) as T`, `(-1) as T`.

The same conversion happens when a `const` has a type: `const b: bits<8> = 65`
means `const b = 65 as bits<8>`.

## What `as` does

`as` tries these steps, in order:

1. **Same type:** nothing to do.
2. **A conversion the types declare**, with a `to` or `from` facet. See below.
3. **A type alias:** check the alias's invariants, then convert to the type
   underneath. See [Type aliases](aliases.md).
4. **A struct with one field:** wrap the value in that field, converting it to
   the field's type, and check the struct's invariants. This is how `65 as
   bits<8>` works: `bits<8>` has a single field, `value`.

If nothing applies, it's an error.

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

## Declaring conversions: `to` and `from`

A struct or type alias can declare conversions with facets, each naming a
macro that performs one:

- **`| to f(source)`**, on the type being converted *from*: how to turn this
  type into something else.
- **`| from f(source)`**, on the type being converted *to*: how to make this
  type from something else.

In the facet, `source` is the value being converted. `as T` uses a conversion
when its macro returns `T` and accepts `source`'s type.

```basm
struct Cents
    | to cents_to_int(source)
    | from dollars_to_cents(source)
{
    pub amount: int,
}

macro cents_to_int(c: Cents) -> int {
    @return c.amount
}

macro dollars_to_cents(dollars: int) -> Cents {
    @return Cents(dollars * 100)
}

macro show(value: int) {
    @emit value
}

show (3 as Cents).amount
show Cents(250) as int
```

```emits
300 250
```

A type can declare any number of each. If more than one conversion fits, the
`as` is ambiguous, which is an error. `std.decimal`'s `Decimal` and
`Fraction` use this to convert between each other and `int`.

## Converting to a generic type: `target`

In a conversion to a generic type, `target` holds the destination type's
const parameters, by name. `as Fixed<4>` sees `target.scale` as `4`:

```basm
struct Fixed<const scale: int>
    | from int_to_fixed(source, target.scale)
{
    pub raw: int,
}

macro int_to_fixed(n: int, scale: int) -> Fixed {
    @return Fixed<scale> { raw: n << scale }
}

macro emit_fixed(f: Fixed<4>) {
    @emit f
}

emit_fixed 3 as Fixed<4>
```

```emits
Fixed<4> { raw: 48 }
```

`std.string` uses `target` the same way, to convert to
`Utf8String<len, endian>` with the length and byte order the caller asked
for.

## Writing conversion macros

A conversion macro is only used if its signature fits exactly:

- **It must declare its return type.** For a generic destination, write the
  bare type name, `-> Fixed`, not `-> Fixed<...>`.
- **Its parameters must be concrete types.** A generic conversion macro,
  such as `f<const S: int>(x: Fixed<S>)`, isn't used; write `x: Fixed<...>`
  instead.

> **Warning:** A conversion macro that doesn't fit is skipped silently. If
> the destination is a one-field struct, `as` then falls back to wrapping the
> value, which gives a different result with no error. Test conversions to
> one-field structs.
