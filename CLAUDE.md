# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

BitterASM is a metalanguage for constructing assembly languages. The language core assumes nothing about a target architecture — no registers, instructions, opcodes, addressing modes, binary encoding, or endianness. Every one of those is defined in ordinary BitterASM code (in `std/`) as libraries and architecture packages. `mov` is not a built-in instruction; it's a macro someone wrote and exposed publicly. The only thing the language itself assumes is `Int`, an arbitrary-precision integer — even `bit`/`bits<N>` are defined in terms of it in the standard library, not built in.

Consequences that matter when reading or extending this code:
- There's no privileged "binary mode." Importing `std.binary` doesn't change how the language behaves; it just brings macros and types into scope like any other import.
- Syntax for an instruction lives with the macro that defines it, not with the language, so the parser/resolver must stay architecture-agnostic — don't special-case ISA concepts in `src/`.
- "Abstraction does not imply optimization": a macro expansion is taken literally, never rewritten by the compiler.

The project is early-stage (lexer, parser, and resolver in Rust; a growing `std` library in BitterASM itself) — expect frequent breaking changes to both the language and `std`.

## Commands

```sh
cargo build --workspace                                # build both the bitterasm and bitter binaries
cargo run -- compile <path/to/file.basm> [-o out.em]    # resolve + expand a .basm file, writing its emitted values to a .em file (note the `--`)
cargo test                                              # run all unit tests + doctests
cargo test <name_substring>                             # run a subset, e.g. `cargo test loader::`
cargo test --doc                                        # doctests only
cargo doc --no-deps --open                              # render rustdoc, including module-level design notes
```

There is no lint/format config checked in (no `rustfmt.toml`/`clippy.toml`); `cargo build` surfaces `dead_code` warnings, which is the closest thing to a lint gate right now.

`bitterasm compile` (the `bitterasm` package's own bin target, `src/main.rs`) currently only gets as far as the language actually supports (see Known gaps below) — most `.basm` files under `std/` and `tests/fixtures/` fail to load today for reasons documented there. `bitter` (the separate crate in `bitter/`, depending on `bitterasm` as a library) is meant to read a `.em` file and encode it into real target output; it's still a stub.

## Architecture

The crate is split as `src/lib.rs` (the real logic, documented with rustdoc) + a thin `src/main.rs` that drives it. A `.basm` file goes through four stages, each its own module — read `src/lib.rs`'s module doc first, then drill into the stage you're touching:

1. **`lexer`** (`src/lexer.rs`) — source text → flat `Token` stream. Newlines are significant tokens (statements are newline-terminated, not `;`-terminated). `>>` always lexes as one `ShiftRight` token, even where it closes two nested generic lists (`Reg<Param<64>>`); the parser is responsible for splitting it back into two `>` tokens where the grammar needs that.

2. **`parser`** (`src/parser/`) — tokens → one file's `ast::Program`. Hand-written recursive descent, with Pratt parsing for expressions in `parser/expressions.rs`. The one non-obvious piece of machinery: parsing `Reg<64>` vs `Reg<T>` requires already knowing whether `Reg`'s declaration takes a const or a type parameter — including declarations that appear later in the same file, or in a file that hasn't been read yet. `Parser` handles the same-file case with a throwaway prepass over the token stream (see `parse_seeded`) that just collects generic signatures before parsing for real; cross-file forward references are handled one level up, by `loader` threading signatures from an imported file's `parse_seeded` call into the seed for the importing file.

3. **`loader`** (`src/loader.rs`) — resolves `from ... import ...` statements to files on disk and flattens the whole import graph into a single, self-contained `ast::Program`, so every later stage can stay import-agnostic. Two things worth knowing before touching this file:
   - Module paths map onto the filesystem 1:1 (`std.binary.native` → `<root>/std/binary/native.basm`), where `<root>` is the current working directory for absolute imports or the importing file's own directory (ascended once per leading dot) for relative ones.
   - Because the resolver works over one flattened, global program with a single flat symbol table, "private" (non-`pub`) declarations can't just be omitted from that table — a `pub` sibling in the same file may still depend on them. Instead they're renamed to a name no `.basm` source could ever spell (`name#<module_id>`, using `#`, which the lexer only treats as a comment starter) so they can't collide with or be referenced from outside their own module. Diamond imports are deduplicated by canonical path so a module is only read/parsed once.

4. **`resolver`** (`src/resolver/`) — resolves the flattened program against itself, now that imports are gone:
   - `resolver::collect_symbols` builds a flat `SymbolTable` (`resolver/symbols.rs`) of top-level struct/type-alias/const declarations.
   - `AliasResolver` (`resolver/aliases.rs`) resolves `types::TypeExpr` trees — struct field types, alias targets, generic arguments — against that table into `resolver::ResolvedType` (`resolver/types.rs`), instantiating generic struct fields as it goes. Type aliases resolve lazily and memoize per symbol; a `Visiting` state (as opposed to `Unvisited`/`Resolved`) is how reference cycles (`type A = B; type B = A`) get caught instead of recursing forever.
   - Generic **const** arguments are *not* currently folded/evaluated — `ResolvedType::Struct`'s args store the raw `Expr`, so `bits<4 + 4>` and `bits<8>` resolve to distinct types today even though they denote the same one. Anything depending on const-generic equality needs to account for this.

`src/types.rs` holds the type-expression AST (`TypeExpr`, `TypeArgument`, `GenericParameter`, `StructField`) shared between the parser (as written) and the resolver (as resolved) — it's deliberately its own module rather than folded into `ast.rs`, because both the parser and resolver need to walk these trees in ways that don't apply to any other AST node.

`@emit`/`@return`/`@assert`/inline `@here` (macro bodies), `@if`/`@else`/`@for` (macro bodies, struct-declaration field lists, and the top level — `resolver/macro_body.rs`, `resolver/structs.rs`, `resolver/toplevel.rs`), and spliced declaration/field names (`` r`id` ``, `ast::NamePart`/`SplicedName`) all work today. `@for`'s loop var/range are packed positionally into `MetaStatement.args`; a struct body's own `@for`/`@if` is a separate `types::StructBodyItem` shape (field-list-bodied, not statement-bodied) rather than reusing `MetaStatement`.

## Known gaps (read before assuming something is broken)

- **Brace-literal struct construction doesn't parse.** `Array<u8, N> { field: value }`-style construction — a generic-instantiated callee followed by a `{ name: value, ... }` argument list — isn't supported; the only call syntax today is paren-style, non-generic-callee `Reg(id = 0)` (`parser/expressions.rs::finish_call`, `resolver/values.rs::eval_call_value`). This is what blocks `std/array.basm` and `std/u8string.basm` from loading, independent of `@if`/`@for` (both files also use those, and those work).
- **No per-field `pub`/default value on struct fields.** `types::StructField` has no `is_pub`/default-value slot, so `pub len: int = N`-shaped fields (also used by `std/array.basm`) don't parse either.
- **`from <package> import <submodule>` doesn't resolve.** A module path maps 1:1 onto a `.basm` *file* (`loader::resolve_module_path`) — there's no notion of a directory-as-package whose members are its `.basm` files. `from std import u8string` looks for `std.basm` (not `std/u8string.basm`) and fails with `ModuleNotFound`. This is what makes `loader::tests::diamond_import_does_not_duplicate_declarations` fail and blocks `std/tinycpu/native.basm` from loading at all, independent of anything else in that file.
- **`std/tinycpu/native.basm`'s `pub const id: bits<2>` struct field doesn't parse**, unrelated to the import gap above — a struct field is always `name: Type`; there's no `const`-prefixed field syntax.
- **A macro's `generated` declarations have nowhere to be spliced back into the resolved program.** `run_macro_body`/`walk_macro_body` correctly collect a macro-body-generated `pub const`/`struct`/`macro`/`type`/label into `MacroExpansion::generated`, but `compile` (`src/main.rs`) just warns and drops them; only `bitterasm expand` (pure syntax, no evaluation) shows what they'd look like.
- **No target-encoding step.** `bitterasm compile` resolves a program, expands every top-level invocation, and writes the resulting `emitted: Vec<Value>` stream to a `.em` file — a self-contained JSON shape (`src/emit.rs`'s `EmittedValue`, with `SymbolId`s resolved to names) that doesn't depend on any in-process state. Nothing turns that into actual machine bytes/text yet — that's `bitter`'s job (`bitter/`), and it's still a stub.
- Generic **const** arguments aren't folded/evaluated in *type* position either (see the resolver bullet above) — `bits<4 + 4>` and `bits<8>` are distinct types today.
