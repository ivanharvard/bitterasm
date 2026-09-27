# Evaluators

The compiler, `bitterasm`, never produces machine code. It runs a program and
records the values it emits, in order, in a `.em` file. An **evaluator**
reads that file and decides what the values mean.

```text
prog.basm ──bitterasm compile──▶ prog.em ──evaluator──▶ output
```

`bitter` is the evaluator that ships with BitterASM. It packs values into
bytes: binary machine code, or an executable. See
[Packing bytes with `bitter`](bitter.md).

## Why split it this way

The language never assumes a program's output is binary. A `bits<8>` is a
struct from the standard library, not a language feature. Only `bitter`
knows that `bits<8>` means "eight bits."

So a different evaluator could read the same `.em` file and produce
something else entirely: a hex dump for a teaching tool, a listing, words for
a 36-bit machine, trits for a ternary one. The language, and every library
that doesn't depend on `bitter`, stays the same.

## Which types an evaluator knows

In `.em`, every struct and enum is identified by its module path and name,
such as `std.binary.bits`. An evaluator gives meaning to the ids it knows,
and treats everything else however its contract says. `bitter` knows a
handful of ids from `std.binary` and `std.bitter`, and packs any other
struct as the concatenation of its fields.

## In this chapter

- [The `.em` format](em-format.md): the specification an evaluator reads.
- [Packing bytes with `bitter`](bitter.md): how `bitter` turns values into
  bytes.
