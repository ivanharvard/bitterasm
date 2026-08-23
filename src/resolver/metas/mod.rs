//! Meta-statement-specific logic lives in each meta's own file
//! ([`assert`]) — adding a new `@` meta means adding a new file and one
//! match arm in [`macro_body::walk_macro_body`](super::macro_body), not
//! editing an existing meta's file. Mirrors [`crate::facets`]'s
//! one-file-per-facet layout, but a step further removed from the parser:
//! a facet's shape (payload kind, cardinality) has to be known before
//! anything is resolvable, while a meta is just `@name arg, arg, ...`
//! ([`crate::parser::macros::parse_meta_statement`]) for every name alike,
//! so there's no parser-facing counterpart to `facets::payload_shape` here
//! — argument shape is this file's own business, checked when the meta
//! actually runs.
//!
//! `@emit` and `@return` aren't here yet — each has its own control-flow
//! shape in `walk_macro_body` (appending to the running `emitted` list,
//! or returning out of the whole body early) that doesn't reduce to a
//! single `Result`-returning call the way `@assert`'s does. Splitting
//! them out the same way, once there's a second meta shaped like
//! `@assert`, is expected.

pub mod assert;
