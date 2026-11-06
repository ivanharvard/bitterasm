<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.png">
    <img src="assets/logo.png" alt="BitterASM" width="480">
  </picture>
</p>

<p align="center">Making Assembly Sweeter.</p>

<!-- ANCHOR: overview -->
## What is BitterASM?
BitterASM is a metalanguage for constructing assembly languages, inspired by several existing assemblers with syntax similar to Rust.

## Motivations and Philosophy

Traditional assemblers often expose developers to design choices that would be considered poor ergonomics in modern programming languages and tooling. There are several existing approaches to reconciling these flaws, but almost all of them fail:

Approach | Why it fails to entirely solve the issue
---------|----------------------------------
Using a macro assembler | Improves code reuse, but retains weak scoping and little to no typing
Creating a higher level programming language | Ignores the issue entirely, forcing assembly developers to learn an entirely new language
Extending an existing assembler with new directives or instructions | Often hackish and relies on non-public APIs
Improving assembler tooling (linters, language servers, etc.) | Improves the developer experience without resolving the language design limitation
Multi-architecture/multi-syntax languages | Often requires parallel development to maintain each syntax or architecture

Instead of isolated improvements, BitterASM combines these approaches to provide assembly developers with a stronger development cycle in a language that will have a gentle learning curve for most assembly developers. BitterASM's goal is to provide a language that can theoretically support any assembly language---past, present, or future---such that the benefits of BitterASM are not just isolated to a single architecture or syntax family (or even the most common ones).

To ensure this is possible, BitterASM attempts to make as few assumptions about the architecture as possible. The complexity lies instead in the code that defines the architecture. 

## Features

The only primitive inside of BitterASM is an `int`, an arbitrarily sized signed integer. From this you can construct other types:
```basm
type uint = int
    | invariant x >= 0
```

Or pack types together:
```basm
## `value / 10^scale`
pub struct Decimal
    | invariant source.scale >= 0
{
    ## The digits, as an integer.
    pub value: int,
    ## How many of those digits come after the decimal point.
    pub scale: int
}
```

And create macros that take in your constructed types:
```basm
pub macro mov(
    rd: Reg,
    imm: int,
    w: int
)
    | emits Bytes<...>
{
    const need_rex = w == 1 || reg_ext(rd) == 1
    const rex = rex_byte(w, 0, 0, reg_ext(rd))
    const opcode = Byte(0xB8 + reg_field(rd))
    const imm_len = 4 + 4 * w
    @emit instr(need_rex, rex, opcode, 0, Byte(0), imm, imm_len)
}
```

Some macros emit values into an `.em` file that can be interpreted by some evaluator (one called `bitter` is provided for you, and converts your `.em` into binary machine code). Other macros simply return values, similar to functions in other programming languages. Some macros do both!

Some `type`s, `struct`s, and `macro`s have been provided for you in the `std` library, though you are free to implement your own! 

Don't like an architecture's instructions? Build your own pseudoinstruction!
```basm
## Sets up a stack frame with `locals` bytes of local storage.
pub macro fn_prologue(locals: uint) {
    push rbp
    mov rbp, rsp
    sub rsp, locals
}

fn_prologue 16      # push rbp; mov rbp, rsp; sub rsp, 16
```

Don't like the syntax? Change the import!
```basm
from std.x86_64.att import *
```
```basm
from std.x86_64.intel import *
```
```basm
from std.x86_64.nasm import *
```

Or make your own!
```basm
syntax add(rd, rs1, rs2) = { $rd$ = $rs1$ + $rs2$ }
```

## "Hello, World!" in x86-64 NASM-style BitterASM vs NASM

As stated before, for most assembly developers, there will be a very gentle learning curve if the architecture you are targeting already has definitions written in BitterASM. For instance, these are two equivalent programs!

<table>
<tr>
<th>x86-64 NASM-style BitterASM</th>
<th>NASM</th>
</tr>
<tr>
<td>

```basm
from std.x86_64.nasm import *
from std.formats.elf import *

elf64_executable EM_X86_64, _start

const text = "Hello, World!\n"

section .rodata
msg:
    db text

section .text
pub _start:
    mov eax, 1          # sys_write
    mov edi, 1          # stdout
    lea rsi, [rel msg]  # buffer
    mov edx, text.len   # length
    syscall

    mov eax, 60         # sys_exit
    xor edi, edi        # status 0
    syscall
```

</td>
<td>

```nasm
global _start

section .rodata
msg:
    db "Hello, World!", 10
len equ $ - msg

section .text
_start:
    mov eax, 1          ; sys_write
    mov edi, 1          ; stdout
    lea rsi, [rel msg]  ; buffer
    mov edx, len        ; length
    syscall

    mov eax, 60         ; sys_exit
    xor edi, edi        ; status 0
    syscall
```

</td>
</tr>
</table>

## Installation

```bash
git clone https://github.com/ivanharvard/bitterasm
# optional LSP for VSCode, in the same parent directory
git clone https://github.com/ivanharvard/bitterasm-lsp   
cd bitterasm && ./install.sh
```
<!-- ANCHOR_END: overview -->

## Documentation

[ivanharvard.github.io/bitterasm](https://ivanharvard.github.io/bitterasm/)

### AI Disclosure

Much of the documentation was AI generated and reviewed by a human. Before full release, the documentation will be rewritten.


