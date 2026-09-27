# Custom syntax

By default, a macro is called as its name followed by comma-separated
arguments: `swap 1, 2`. The `syntax` facet gives a macro any call shape you
like. That's how assembly syntax is built: `lw a0, 8(sp)`,
`mov rax, [rbx + 8]` and `x1 = x2 + x3` are all ordinary macros with custom
syntax.

```basm
macro swap(a: int, b: int)
    | syntax { swap $a$ with $b$ }
{
    @emit b
    @emit a
}

swap 1 with 2
```

```emits
2 1
```

## Patterns

A pattern is a sequence of tokens between `{` and `}`:

- **`$name$`** is a *capture*. The call site can put any expression there,
  and it becomes the argument for parameter `name`.
- **Anything else** is literal: the call site must contain exactly that
  token.

Every capture must name one of the macro's parameters. To match a literal
`` ` `` or `$`, escape it: `` \` `` or `\$`.

## Three kinds of pattern

**Anchored** patterns start with the macro's own name, like the `swap`
pattern above. They're tried only for statements that start with that name,
so they're cheap. Most instruction syntax is anchored.

**Unanchored** patterns start with something else, usually a capture. They
are tried against every statement, so a statement doesn't need to start with
a mnemonic at all:

```basm
macro store(dst: int, src: int)
    | syntax { $dst$ <- $src$ }
{
    @emit dst * 100 + src
}

const r1 = 1

r1 <- 7
```

```emits
107
```

A statement must start with a name, so `r1 <- 7` works where `1 <- 7`
wouldn't.

**Operand** patterns start with a token that can't begin a statement, such
as `[`. They're tried wherever an *argument* starts, and a match becomes an
expression call. So an operand pattern belongs on a macro that returns a
value:

```basm
struct Mem {
    pub addr: int,
}

macro mem(addr: int) -> Mem
    | syntax { [$addr$] }
{
    @return Mem(addr)
}

macro load(dst: int, src: Mem) {
    @emit dst
    @emit src.addr
}

load 1, [0x40]
```

```emits
1 64
```

`[0x40]` becomes `mem(0x40)`, a `Mem`, and [overloading](overloading.md)
picks the `load` that takes a `Mem`. This is how `std.x86_64.nasm` gives
`[rel label]` its own type.

## Changing another macro's syntax

A `syntax` statement assigns a call shape to a macro declared somewhere
else, without touching its declaration:

```basm
macro copy(dst: int, src: int) {
    @emit dst
    @emit src
}

syntax copy(dst, src) = { copy $src$ to $dst$ }

copy 5 to 6
```

```emits
6 5
```

The names in parentheses are the macro's parameters, which the pattern can
capture. This is what makes **dialects** possible. `std.riscv.impl` declares
every instruction with no syntax of its own. Then:

- `std.riscv.native` assigns conventional syntax: `lw a0, 8(sp)`.
- `std.riscv.c_like` assigns C-like syntax to the same macros:
  `a0 = a1 + a2`.

A program imports one dialect or the other, and both share one
implementation. A file that imports a macro gets the syntax its dialect
assigned too. If two imports assign different syntax to the same macro,
the importing file must pick one with a `syntax` statement of its own.

## Designing patterns

A capture accepts any expression, and it stops only where the next literal
token appears. Two patterns are ambiguous when some statement matches both,
so start them differently:

- `std.riscv.c_like` writes register forms as `$rd$ = $rs1$ + $rs2$` and
  immediate forms as `$rd$ <- $rs1$ + $imm$`. If both used `=`,
  `x1 = x2 + imm - 1` would be a valid parse of either one.
- Avoid a literal `:` right after a leading capture. `name:` is how a
  [label](../programs/labels.md) is written, and that check runs first.

Declare a macro with custom syntax *before* any statement that uses it in
the same file. The parser can misread an earlier call that uses a shape it
hasn't seen yet.
