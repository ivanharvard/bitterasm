# Generating declarations

A macro body can contain declarations: `pub const`, `struct`, `enum`,
`type`, and even `macro`. Each call to the macro adds them to the program as
if they had been written at the top level.

```basm
macro declare_word(bits: int) {
    pub const WORD`bits`_BYTES = `bits / 8`
    struct Word`bits` {
        pub value: int,
    }
}

declare_word 16
declare_word 32

macro show(value: int) {
    @emit value
}

macro emit_word(w: Word16) {
    @emit w
}

show WORD32_BYTES
emit_word Word16(7)
```

```emits
4
Word16 { value: 7 }
```

Two calls generated `WORD16_BYTES`, `Word16`, `WORD32_BYTES` and `Word32`.

## What gets evaluated

When the macro runs, the declaration's **name** is evaluated: the backticks
in `` Word`bits` `` splice in the parameter's value. See
[Spliced names](../splicing/names.md).

Everything else is copied into the program *as written*, and evaluated later
where the declaration ends up. There, the macro's parameters no longer
exist.

The one exception is a `pub const`'s value. Backticks in it are evaluated
when the macro runs, so `` `bits / 8` `` above becomes `2` or `4`. Without
the backticks, `bits / 8` would refer to a `bits` that isn't defined at the
top level.

A generated struct, enum, type alias or macro gets no such exception: only
its name can use the macro's parameters.

## Inside a macro, `pub const` and `const` differ

- `const x = ...` names a value for the rest of the macro body.
- `pub const x = ...` generates a top-level constant.

## Seeing what was generated

A program that generates declarations gets a `generated_declarations`
warning, as a reminder that it contains code you can't see in its source.
`bitterasm expand` prints the program with each call replaced by its body,
which shows what was generated:

```sh
bitterasm expand program.basm
```

To silence the warning, allow the lint (see
[Diagnostics and lints](../tools/diagnostics.md)).
