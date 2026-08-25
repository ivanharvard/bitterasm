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

`bitterasm compile` (the `bitterasm` package's own bin target, `src/main.rs`) currently only gets as far as the language actually supports (see Known gaps below) — most `.basm` files under `std/` load; `std/tinycpu/native.basm` (and anything importing it) is the one known exception, for reasons documented there. `bitter` (the separate crate in `bitter/`, depending on `bitterasm` as a library) is meant to read a `.em` file and encode it into real target output; it's still a stub.

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
   - Generic **const** arguments are folded/evaluated, not compared as raw `Expr` trees — `ResolvedGenericArg::Const` holds a folded `Int`, so `bits<4 + 4>` and `bits<8>` resolve to the same type.
   - A declaration discovered *mid-resolution* — a macro's `generated` output, or `0..N`'s synthesized struct (see below) — is registered via `AliasResolver::register_generated` into a second `SymbolTable` (`generated_symbols`, sharing the same `SymbolId` space via `SymbolTable::with_base`) rather than requiring a mutable/growable `Program`. `AliasResolver::get_symbol`/`lookup_symbol` check both tables; `find_struct_declaration`/`find_macro_declaration`/etc. fall back to scanning `self.generated` after `program.statements`. See `resolver/generated.rs`.

`src/types.rs` holds the type-expression AST (`TypeExpr`, `TypeArgument`, `GenericParameter`, `StructField`) shared between the parser (as written) and the resolver (as resolved) — it's deliberately its own module rather than folded into `ast.rs`, because both the parser and resolver need to walk these trees in ways that don't apply to any other AST node. `StructField` carries `is_pub`/`is_const`/`default` — `is_pub` gates whether `@for` visits a field (see below), `default` fills a field omitted from a construction, `is_const` is parsed but not yet enforced.

`@emit`/`@return`/`@assert`/inline `@here` (macro bodies), `@if`/`@else`/`@for` (macro bodies, struct-declaration field lists, brace-literal construction bodies, and the top level), brace-literal struct construction (`Array<u8, N> { field: value, ... }`, including generic callees — `parser/expressions.rs::finish_construct`, `resolver/values.rs::eval_construct_value`), and spliced declaration/field names (`` r`id` ``, `ast::NamePart`/`SplicedName`) all work today — **except** a struct/macro *declaration's own name* isn't spliceable (`StructDeclaration.name`/`MacroDeclaration.name` are plain `String`), only field/const names are.

**`@for i in X` and `0..N`**: `@for`'s loop var/source are packed positionally into `MetaStatement.args` as `[var, source]` (`source` is any `Expr`, no longer required to be `start..end`). `X` is evaluated to a `Value::Struct` and its **pub** fields are visited in declaration order (`resolver/generated.rs::eval_for_source`) — uniformly across macro-body, construct-item, and struct-body `@for` (the last one stays restricted to `Int`-valued pub fields, since it only has a const-generic scope to evaluate against, not a general `Value` one). **Top-level `@for` is the one exception**: it runs before symbol/struct resolution exists at all (`resolver/toplevel.rs`), so it stays restricted to literal `start..end` sugar and errors (`ResolveError::TopLevelForRequiresRange`) on anything else. `start..end` itself is `ast::Expr::Range`, a real expression (not grammar owned only by `@for`) that desugars to a synthesized, unspellable-named, `pub`-Int-fielded struct via `AliasResolver::eval_range_value` — a literal materialization (one real struct field per element), not a compiler-side lazy-loop rewrite, per "abstraction does not imply optimization."

## Known gaps (read before assuming something is broken)

- **No target-encoding step.** `bitterasm compile` resolves a program, expands every top-level invocation, and writes the resulting `emitted: Vec<Value>` stream to a `.em` file — a self-contained JSON shape (`src/emit.rs`'s `EmittedValue`, with `SymbolId`s resolved to names) that doesn't depend on any in-process state. Nothing turns that into actual machine bytes/text yet — that's `bitter`'s job (`bitter/`), and it's still a stub.
- **Macros have no generic type parameters.** `MacroDeclaration` has no `generic_params` at all (only `struct`/`type` do), so a signature like `macro get(arr: Array<T, ...>, index: int) -> T` — generic over an element type `T`, with `...` (a real `Ellipsis` token, matching "any const value for this parameter") standing in for the array's length — doesn't resolve. `std/array.basm`'s `get`/`updated`/`reversed` are commented out for exactly this reason; nothing else in `std/` currently calls them.
- **A macro can only ever be invoked as its own statement, never as a sub-expression.** `Expr::Call`'s callee must resolve to a *struct* (`eval_call_value`/`eval_construct_value` construct a `Value::Struct`); there's no path for "invoke this macro and use its return value here." `std/tinycpu/native.basm`'s `` `u8string.title(name)`Instr `` needs this (plus the point below) and is the reason `loader::tests::diamond_import_does_not_duplicate_declarations` still fails — everything else on `mini.basm`'s import path (`std.tinycpu.native` → `std.binary`, `std.u8string`, `std.array`, `std.ctypes`, `std.iter`, `std.math`, `std.decimal`, `std.unsigned`) loads cleanly.
- **A struct/macro declaration's own name isn't spliceable.** `StructDeclaration.name`/`MacroDeclaration.name` are plain `String`, unlike a field or const's `SplicedName` — `` struct `expr`Suffix `` doesn't parse. Also blocks `native.basm`.
- Macro parameter lists don't support a trailing comma (unlike struct fields and construction items, which do) or a default value (unlike struct fields, which now do via `StructField.default`).
- **No macro overloading.** `SymbolTable` is name-only (`resolver/symbols.rs::insert` errors on any duplicate name, regardless of `SymbolKind`) — `collect_symbols` hard-errors the moment two `macro`s share a name anywhere in the flattened program, even with different parameter types. `std/decimal.basm`'s two `to_int` overloads (`Decimal`/`Fraction`) and `std/math.basm`'s three-way `ceil`/`abs` overloads (`int`/`Decimal`/`Fraction`) all parse fine but fail resolution this way — this is a pre-existing mismatch in that WIP code's own design (assuming type-based overload resolution that was never built), not something the `@for`/import work above touched. Affects anything importing `std.math`/`std.decimal` (transitively: `std.iter` → `std.array` → `std.u8string`), so `cargo run -- compile` on those currently fails at the resolver stage with `DuplicateSymbol` even though they all load/parse cleanly.
