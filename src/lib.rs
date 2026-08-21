//! BitterASM is a metalanguage for constructing assembly languages: the
//! language core defines no registers, instructions, or binary encoding of
//! its own, and leaves all of that to ordinary BitterASM code in `std`. See
//! the repository [README](https://github.com/) for the design rationale.
//!
//! # Pipeline
//!
//! A `.basm` file goes through four stages, each its own module:
//!
//! 1. [`lexer`] turns source text into a flat [`Token`](token::Token) stream.
//! 2. [`parser`] turns tokens into an [`ast::Program`], one file at a time.
//!    Because generic type arguments (`bits<8>` vs. `bits<T>`) parse
//!    differently depending on whether the referenced name takes a const or
//!    a type parameter, the parser needs to know a declaration's generic
//!    signature before it can parse a use of it — including declarations
//!    that live in a file imported later. See [`parser::parse_seeded`].
//! 3. [`loader`] resolves `from ... import ...` statements to files on disk
//!    and flattens the whole import graph into a single, self-contained
//!    [`ast::Program`], so every later stage can stay import-agnostic.
//! 4. [`resolver`] collects declarations into a [`resolver::SymbolTable`]
//!    and resolves type expressions ([`types::TypeExpr`]) against it,
//!    producing [`resolver::ResolvedType`]s and instantiated struct fields.
//!
//! [`types`] holds the shared type-expression AST used by both the parser
//! (as written in source) and the resolver (as resolved against symbols).

pub mod ast;
pub mod lexer;
pub mod token;
pub mod parser;
pub mod types;
pub mod resolver;
pub mod loader;
pub mod facets;
pub mod eval;
