//! A hand-written recursive-descent parser (Pratt parsing for expressions,
//! in [`expressions`]) over a single file's [`Token`] stream.
//!
//! Generic argument lists are the awkward part: `Reg<64>` and `Reg<T>` are
//! only distinguishable by knowing whether `Reg`'s declaration takes a
//! const or a type parameter, and that declaration may live later in this
//! file or in another file entirely. [`Parser`] handles the first case with
//! a throwaway prepass (see [`parse_seeded`]) that walks the file purely to
//! collect every declaration's generic signature before parsing it for
//! real; [`crate::loader`] handles the second by threading signatures
//! collected from one file's [`parse_seeded`] call into the next as a seed.
//!
//! A macro's `syntax` facet (see [`crate::facets::syntax`]) needs the exact
//! same treatment: matching a call site against a custom pattern requires
//! already knowing that pattern, possibly from later in this file or from
//! another file entirely, so [`ParserSeed`] carries `macro_syntaxes`
//! alongside `generic_signatures` through the same prepass and the same
//! cross-file threading in [`crate::loader`].

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ast::{
    BinaryOp, CallArgument, ConstDeclaration, Expr, ExtendField, ExtendValue, ImportItems, ImportStatement,
    Invocation, Label, MacroDeclaration, MacroParameter, MetaStatement, ModulePath,
    NamePart, Program, Section, Statement, SyntaxOverrideStatement, UnaryOp, TypeAliasDeclaration,
};

use crate::facets::syntax::SyntaxPattern;
use crate::token::{DocComment, Span, Token, TokenKind};
use crate::types::{GenericParameter, StructBodyItem, StructField, TypeExpr, TypeArgument};

mod expressions;
mod statements;
mod type_exprs;
mod declarations;
mod macros;
mod construct;
mod facets;
mod invocation_syntax;
mod docs;

#[cfg(test)]
mod tests;

// ================
// public entry point
// ================

/// Parses a single, self-contained token stream with no externally-known
/// generic signatures. Most callers should go through [`crate::loader`]
/// instead, which seeds signatures pulled in from imports.
///
/// ```
/// use bitterasm::lexer::lex;
/// use bitterasm::parser::parse;
///
/// let tokens = lex("const x = 1\n").unwrap();
/// let program = parse(tokens).unwrap();
/// assert_eq!(program.statements.len(), 1);
/// ```
pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    let (program, mut errors) = parse_recovering(tokens);
    if errors.is_empty() { Ok(program) } else { Err(errors.swap_remove(0)) }
}

/// Like [`parse`], but doesn't stop at the first error: a statement that
/// fails to parse is reported and skipped, and parsing picks up at the next
/// one. Returns every statement that did parse, and every error in source
/// order. An editor uses it to keep working on the rest of a broken file.
///
/// ```
/// use bitterasm::lexer::lex;
/// use bitterasm::parser::parse_recovering;
///
/// let tokens = lex("const x = )\nconst y = 2\nconst z = (\n").unwrap();
/// let (program, errors) = parse_recovering(tokens);
/// assert_eq!(program.statements.len(), 1);
/// assert_eq!(errors.len(), 2);
/// ```
pub fn parse_recovering(tokens: Vec<Token>) -> (Program, Vec<ParseError>) {
    let (program, _, errors) = parse_seeded(tokens, Vec::new(), &ParserSeed::default());
    (program, errors)
}

/// What the same-file prepass discovers, threaded across files the same
/// way: `generic_signatures` (struct/type-alias generic signatures) and
/// `macro_syntaxes` (custom call-site patterns from a `syntax` facet) are
/// unrelated key spaces produced by different declarations, but both are
/// needed *before* the rest of a file can be parsed for real, so both ride
/// through [`parse_seeded`]/[`crate::loader`] together as one seed value
/// rather than two parallel maps that always have to move in lockstep.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParserSeed {
    pub generic_signatures: HashMap<String, Vec<GenericParamKind>>,
    pub macro_syntaxes: HashMap<String, Vec<SyntaxPattern>>,

    /// The effective `syntax name(...) = { ... }` override active for each
    /// name by the end of this file — inherited overrides plus whatever
    /// this file declared itself, with a local declaration always taking
    /// priority. `crate::loader` merges this across a file's several
    /// imports and is where a genuine cross-import conflict (two *different*
    /// inherited patterns for the same name, neither locally resolved) is
    /// caught — see its `syntax_override_conflict` handling.
    pub syntax_overrides: HashMap<String, SyntaxPattern>,

    /// Unanchored patterns (`facets::syntax::is_anchored` is false) — see
    /// `Parser::add_syntax_pattern`'s doc. `(target macro name, pattern)`,
    /// order-insensitive (matching is tried against every one of them
    /// regardless of order) and deduplicated by equality so a diamond
    /// import doesn't turn into a false "ambiguous" match.
    pub unanchored_syntaxes: Vec<(String, SyntaxPattern)>,

    /// Just the names *this* file itself declared an override for (not
    /// inherited) — `crate::loader` checks this, and only this, to decide
    /// whether a conflict among this file's own imports was actually
    /// resolved before propagating `syntax_overrides` onward. Never
    /// consumed more than one import hop away, so — unlike every other
    /// field here — it's never accumulated from a file's own imports.
    pub local_syntax_overrides: HashSet<String>,
}

/// Parses `tokens`, treating `seed` as generic signatures and macro syntax
/// patterns known in advance (e.g. pulled in from imported modules, whose
/// declarations aren't textually present in this token stream), and
/// attaching `docs` (from [`crate::lexer::lex_with_docs`]) to the items
/// they document. Returns
/// everything visible by the end of the file (seed plus whatever this file
/// declared itself), so callers can pass it on to files that import *this*
/// one in turn. Parsing recovers from errors the way [`parse_recovering`]
/// does, so the program holds every statement that parsed.
pub(crate) fn parse_seeded(
    tokens: Vec<Token>,
    docs: Vec<DocComment>,
    seed: &ParserSeed,
) -> (Program, ParserSeed, Vec<ParseError>) {
    // dummy pass to collect signatures
    let mut prepass = Parser::new(tokens.clone());
    prepass.generic_signatures.extend(seed.generic_signatures.clone());
    prepass.macro_syntaxes.extend(seed.macro_syntaxes.clone());
    prepass.syntax_overrides.extend(seed.syntax_overrides.clone());
    prepass.unanchored_syntaxes.extend(seed.unanchored_syntaxes.iter().cloned());
    let _ = prepass.parse_program();

    let mut parser = Parser::new(tokens);
    parser.pending_docs = docs;
    parser.generic_signatures = prepass.generic_signatures;
    parser.macro_syntaxes = prepass.macro_syntaxes;
    parser.syntax_overrides = prepass.syntax_overrides;
    parser.unanchored_syntaxes = prepass.unanchored_syntaxes;

    let program = parser.parse_program();

    (
        program,
        ParserSeed {
            generic_signatures: parser.generic_signatures,
            macro_syntaxes: parser.macro_syntaxes,
            syntax_overrides: parser.syntax_overrides,
            local_syntax_overrides: parser.local_syntax_overrides,
            unanchored_syntaxes: parser.unanchored_syntaxes,
        },
        parser.errors,
    )
}

// Parses `tokens` leniently (ignoring its errors) purely to
// discover which modules it imports, without needing to know anything about
// those modules' contents first. Import statements don't depend on any
// generic signature, so this succeeds even when later statements in the
// file don't (e.g. because they use a type from one of these imports).
pub(crate) fn discover_imports(tokens: Vec<Token>) -> Vec<ImportStatement> {
    let mut parser = Parser::new(tokens);
    let _ = parser.parse_program();
    parser.imports
}

// ================
// errors
// ================

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at byte {}..{}",
            self.message,
            self.span.start,
            self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

// ================
// parser
// ================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenericParamKind {
    Type,
    Const,
}

impl From<&GenericParameter> for GenericParamKind {
    fn from(param: &GenericParameter) -> Self {
        match param {
            GenericParameter::Type { .. } => GenericParamKind::Type,
            GenericParameter::Const { .. } => GenericParamKind::Const,
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,

    generic_signatures: HashMap<String, Vec<GenericParamKind>>,
    macro_syntaxes: HashMap<String, Vec<SyntaxPattern>>,
    syntax_overrides: HashMap<String, SyntaxPattern>,
    local_syntax_overrides: HashSet<String>,
    unanchored_syntaxes: Vec<(String, SyntaxPattern)>,
    imports: Vec<ImportStatement>,

    // See `parse_statement_block`'s doc — a standalone `syntax` override
    // is only recognized at true top level (depth 0).
    pub(super) block_depth: usize,

    // While parsing a generic argument expression (e.g. the `width` in
    // `bits<width>`), `>`-shaped tokens (`Greater`, `GreaterEqual`,
    // `ShiftRight`) can't be treated as operators — any of them might
    // instead be closing the argument list. Every other operator, `<<`
    // included, is unambiguous (only `>` ever closes a list, never `<`),
    // so this is narrower than refusing every low-precedence operator.
    // Lifted while parsing inside parens, same as `Foo<(a > b)>` needing
    // parens in C++ for the same reason.
    pub(super) restrict_closing_ops: bool,

    // While parsing an `@if`/`@for` header's condition/range-bound
    // expression, a bare identifier immediately followed by `{` can't be
    // read as the start of a brace-literal construction (`expressions.rs`'s
    // postfix loop) — that `{` is the header's own body, e.g. `@if i ==
    // index { ... }`. Same ambiguity Rust avoids by banning bare struct
    // literals in `if`/`while`/`for` scrutinee position; lifted inside
    // parens for the same reason `restrict_closing_ops` is, so `@if (Foo {
    // x: 1 }).y { ... }` still allows construction there.
    pub(super) restrict_brace_construction: bool,

    /// True while parsing a const's value, where a line starting with `|`
    /// continues it: `const flash: space = stream\n    | start(0x8000)`.
    pub(super) pipe_continues_line: bool,

    /// True while parsing a splice's contents (`` `...` `` or a spliced
    /// name's `` `...` `` piece), where an identifier followed by a
    /// backtick is that splice closing, not a spliced name starting.
    pub(super) in_splice: bool,

    // `##`/`#!` lines not yet claimed by an item, in source order, and the
    // ones that turned out to document nothing. See `docs.rs`.
    pending_docs: Vec<DocComment>,
    stray_docs: Vec<DocComment>,

    /// Every statement that failed to parse, in source order. See
    /// `recover`.
    errors: Vec<ParseError>,

    /// Above 0 while trying a custom syntax pattern that may not match.
    /// A failed try is thrown away, so a block inside it mustn't record
    /// its errors and carry on; it passes them up instead.
    pub(super) speculating: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            generic_signatures: HashMap::new(),
            macro_syntaxes: HashMap::new(),
            syntax_overrides: HashMap::new(),
            local_syntax_overrides: HashSet::new(),
            unanchored_syntaxes: Vec::new(),
            imports: Vec::new(),
            block_depth: 0,
            restrict_closing_ops: false,
            restrict_brace_construction: false,
            pipe_continues_line: false,
            in_splice: false,
            pending_docs: Vec::new(),
            stray_docs: Vec::new(),
            errors: Vec::new(),
            speculating: 0,
        }
    }

    fn register_generic_signature(
        &mut self,
        name: &str,
        params: &[GenericParameter],
    ) {
        if params.is_empty() {
            return;
        }

        self.generic_signatures.insert(
            name.to_string(),
            params.iter().map(GenericParamKind::from).collect(),
        );
    }

    fn register_macro_syntax(&mut self, name: &str, pattern: SyntaxPattern) {
        self.add_syntax_pattern(name, pattern);
    }

    // A standalone `syntax name(...) = { ... }` override, unlike a
    // declaration's own `syntax` facet, *takes over* `name`'s call-site
    // shape rather than adding one more accepted overload — so it clears
    // out whatever was already registered for `name` (anchored or not,
    // inherited or from an earlier facet declaration) before adding the
    // new one, instead of appending to it. Two such overrides for the same
    // name *in this file* is an unambiguous mistake (there's no
    // import-order excuse for it), so that's a hard parse error; a
    // cross-import conflict this file doesn't locally override at all is
    // instead caught later, by `crate::loader`, once it can see whether
    // this file's own declarations (tracked in `local_syntax_overrides`)
    // resolved it.
    fn register_syntax_override(
        &mut self,
        name: &str,
        pattern: SyntaxPattern,
        span: Span,
    ) -> Result<(), ParseError> {
        if !self.local_syntax_overrides.insert(name.to_string()) {
            return Err(ParseError::new(
                format!("`{name}`'s syntax is already overridden in this file"),
                span,
            ));
        }

        self.macro_syntaxes.remove(name);
        self.unanchored_syntaxes.retain(|(existing, _)| existing != name);

        self.syntax_overrides.insert(name.to_string(), pattern.clone());
        self.add_syntax_pattern(name, pattern);

        Ok(())
    }

    // Anchored (starts with `name` itself, e.g. `add $rd$, $rs1$, $rs2$`):
    // stored under `macro_syntaxes[name]`, the parser's O(1) dispatch-by-
    // leading-token path. Unanchored (e.g. `$rd$ = $rs1$ + $rs2$`, no
    // instruction name required at all — the whole point of supporting
    // it): nothing about a call site's first token says which macro it's
    // for, so it's kept in one flat pool every identifier-led statement
    // tries against, regardless of its own leading token. See
    // `facets::syntax::is_anchored`'s doc and
    // `parse_invocation_via_syntax_candidates`.
    fn add_syntax_pattern(&mut self, name: &str, pattern: SyntaxPattern) {
        if crate::facets::syntax::is_anchored(&pattern, name) {
            let patterns = self.macro_syntaxes.entry(name.to_string()).or_default();
            if !patterns.contains(&pattern) {
                patterns.push(pattern);
            }
        } else {
            let entry = (name.to_string(), pattern);
            if !self.unanchored_syntaxes.contains(&entry) {
                self.unanchored_syntaxes.push(entry);
            }
        }
    }

    // ===============
    // program
    // ===============

    fn parse_program(&mut self) -> Program {
        let start = self.current().span.start;

        let mut statements = Vec::new();

        self.skip_newlines();
        let doc = self.take_module_doc();

        while !self.at_eof() {
            let doc_lines = self.take_doc_lines();
            let statement_start = self.pos;
            match self.parse_statement() {
                Ok(mut statement) => {
                    self.attach_doc(&mut statement, doc_lines);
                    statements.push(statement);
                }
                Err(error) => {
                    self.errors.push(error);
                    // At top level there's no block to be missing its `}`.
                    let resume = self.recovery_point(statement_start, false).unwrap_or(self.pos);
                    self.pos = resume.max(statement_start + 1).min(self.tokens.len() - 1);
                }
            }
            self.skip_newlines();
        }

        let end = self.current().span.end;

        Program {
            statements,
            span: Span::new(start, end),
            doc,
            stray_docs: self.finish_docs(),
        }
    }

    // =============
    // recovery
    // =============

    /// Where to pick up after the statement starting at token
    /// `statement_start` failed to parse at the current token. Scans from
    /// the statement's start, keeping a stack of open brackets, for:
    ///
    /// - a newline with no bracket open, at or past the error: the
    ///   statement ends there, so parsing resumes after it;
    /// - in a block, a `}` with no `{` open: it closes the block, so
    ///   parsing resumes at it, leaving it for the block. Any `(` or `[`
    ///   still open was never closed;
    /// - with a bracket left open, a line starting at column 0: a bracket
    ///   was never closed, and that line most likely starts the next
    ///   top-level statement. At top level parsing resumes there; in a
    ///   block, `None` says the block itself is unclosed, so it passes the
    ///   error up instead of reading the rest of the file as its body;
    /// - the end of the file.
    fn recovery_point(&self, statement_start: usize, in_block: bool) -> Option<usize> {
        let error_pos = self.pos;
        let mut open: Vec<TokenKind> = Vec::new();

        for index in statement_start..self.tokens.len() {
            let token = &self.tokens[index];
            let opener = match token.kind {
                TokenKind::RParen => Some(TokenKind::LParen),
                TokenKind::RBrace => Some(TokenKind::LBrace),
                TokenKind::RBracket => Some(TokenKind::LBracket),
                _ => None,
            };

            let at_line_start = index > error_pos
                && self.tokens[index - 1].kind == TokenKind::Newline
                && self.tokens[index - 1].span.end == token.span.start;
            if !open.is_empty() && at_line_start && opener.is_none() && token.kind != TokenKind::Pipe {
                return (!in_block).then_some(index);
            }

            match &token.kind {
                TokenKind::Eof => return Some(index),
                TokenKind::LParen | TokenKind::LBrace | TokenKind::LBracket => open.push(token.kind.clone()),
                TokenKind::Newline if open.is_empty() && index >= error_pos => return Some(index + 1),
                _ => {}
            }

            // A closer closes its own kind of bracket, and any opened after
            // it that were never closed. One with nothing to close is the
            // block's `}`, or at top level a stray closer, skipped over.
            if let Some(opener) = opener {
                match open.iter().rposition(|kind| *kind == opener) {
                    Some(at) => open.truncate(at),
                    None if in_block && opener == TokenKind::LBrace && index >= error_pos => return Some(index),
                    None => {}
                }
            }
        }

        Some(self.tokens.len() - 1)
    }

    // =============
    // statement end
    // =============

    fn at_statement_end(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        )
    }

    // consume a new line if present and return the ending byte offset
    // eof is also a valid statement terminator
    fn statement_end(&mut self) -> Result<usize, ParseError> {
        match self.current().kind {
            TokenKind::Newline => {
                let end = self.current().span.end;
                self.advance();
                Ok(end)
            }

            TokenKind::Eof => Ok(self.current().span.end),

            // A block closer terminates its final statement without being
            // consumed; `parse_statement_block` owns the brace itself.
            TokenKind::RBrace => Ok(self.current().span.start),

            _ => Err(ParseError::new(
                "expected end of line",
                self.current().span,
            )),
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    // True when the upcoming tokens are `@else` — used by `@if`
    // (macro-body statements) and a struct body's own `@if` alike, to
    // decide whether a then-branch is followed by an else branch without
    // consuming anything.
    fn at_else_meta(&self) -> bool {
        self.check(&TokenKind::At)
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| matches!(&token.kind, TokenKind::Identifier(name) if name == "else"))
    }

    // Optional trailing newline after a block's closing `}`, mirroring
    // `parse_macro_declaration`/`parse_struct_declaration`'s own
    // closing-brace handling.
    fn consume_trailing_newline(&mut self) {
        if self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    // A name that may be built from evaluated fragments — `` r`id` `` is
    // the literal text `r` followed by `id` evaluated and pasted in as
    // text (see `ast::NamePart`). Every ordinary name (the overwhelming
    // majority) is a single `Identifier` token with no backtick following
    // it, parsing to one `Literal` part. Used for a `const` declaration's
    // name and a struct field's name — the two positions a macro body or
    // `@for`-generated struct body needs a computed name in.
    fn parse_spliced_name(&mut self) -> Result<(Vec<NamePart>, Span), ParseError> {
        let first_token = self.current().clone();
        let first = self.expect_identifier()?;

        let mut parts = vec![NamePart::Literal(first)];
        let mut end = first_token.span.end;

        while self.check(&TokenKind::Backtick)
            && self.tokens.get(self.pos + 1).is_some_and(|token| !matches!(
                token.kind,
                TokenKind::Colon
                    | TokenKind::Comma
                    | TokenKind::RBrace
                    | TokenKind::RParen
                    | TokenKind::RBracket
                    | TokenKind::Newline
                    | TokenKind::Eof
                    | TokenKind::Backtick
            ))
        {
            self.advance();

            let outer_in_splice = self.in_splice;
            self.in_splice = true;
            let inner = self.parse_expr_bp_until(0, Some(&TokenKind::Backtick));
            self.in_splice = outer_in_splice;
            let inner = inner?;

            let closing = self.current().clone();
            self.expect_simple(TokenKind::Backtick)?;
            end = closing.span.end;

            parts.push(NamePart::Splice(inner));

            if matches!(self.current().kind, TokenKind::Identifier(_)) {
                let literal_token = self.current().clone();
                let literal = self.expect_identifier()?;
                parts.push(NamePart::Literal(literal));
                end = literal_token.span.end;
            }
        }

        Ok((parts, Span::new(first_token.span.start, end)))
    }

    // A `syntax` pattern's `{ ... }` body — shared by the `syntax` facet
    // (`parser::facets`) and the standalone `syntax name(...) = { ... }`
    // override statement (`parse_syntax_override`, below). Consumes
    // already-lexed tokens directly (no re-lexing a string), tracking
    // brace depth so a pattern could in principle nest braces of its own;
    // newlines inside are skipped rather than treated as significant,
    // since a pattern is conceptually one line regardless of how the
    // source wraps it. The opening `{` must already be current.
    fn parse_pattern_block(&mut self, context: &str) -> Result<Vec<TokenKind>, ParseError> {
        self.expect_simple(TokenKind::LBrace)?;

        let mut tokens = Vec::new();
        let mut depth: usize = 1;

        loop {
            if self.at_eof() {
                return Err(ParseError::new(
                    format!("unterminated {context}"),
                    self.current().span,
                ));
            }

            let kind = self.current().kind.clone();

            match kind {
                TokenKind::LBrace => {
                    depth += 1;
                    tokens.push(kind);
                    self.advance();
                }

                TokenKind::RBrace => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                    tokens.push(kind);
                }

                TokenKind::Newline => {
                    self.advance();
                }

                other => {
                    tokens.push(other);
                    self.advance();
                }
            }
        }

        Ok(tokens)
    }

    // =============
    // expectations
    // =============

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        let token = self.current().clone();

        match token.kind {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }

            other => Err(ParseError::new(
                format!("expected identifier, found {other:?}"),
                token.span,
            )),
        }
    }

    fn expect_simple(&mut self, expected: TokenKind) -> Result<(), ParseError> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(
                format!(
                    "expected {:?}, found {:?}",
                    expected,
                    self.current().kind
                ),
                self.current().span,
            ))
        }
    }

    /// Whether the `<` at the current token opens generic arguments rather
    /// than a comparison. A name with a generic signature can still be an
    /// ordinary value, like a parameter `index` while a generic macro
    /// `index` exists, so `index < arr.len` is a comparison. Generic
    /// arguments hold only names, numbers, `,`, `.`, `...` and arithmetic,
    /// and their closing `>` is followed by what can follow a generic name:
    /// a call, a construction, a variant, or the end of a facet or argument.
    pub(super) fn looks_like_generic_args(&self) -> bool {
        let mut depth = 0i32;
        let mut in_splice = false;
        for (offset, token) in self.tokens[self.pos..].iter().enumerate() {
            let kind = &token.kind;
            if in_splice {
                match kind {
                    TokenKind::Backtick => in_splice = false,
                    TokenKind::Newline | TokenKind::Eof => return false,
                    _ => {}
                }
                continue;
            }
            match kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater | TokenKind::ShiftRight => {
                    depth -= if *kind == TokenKind::ShiftRight { 2 } else { 1 };
                    if depth < 0 {
                        return false;
                    }
                    if depth == 0 {
                        let next = self.tokens.get(self.pos + offset + 1).map(|token| &token.kind);
                        return matches!(
                            next,
                            None | Some(
                                TokenKind::LParen
                                    | TokenKind::LBrace
                                    | TokenKind::Dot
                                    | TokenKind::Pipe
                                    | TokenKind::Newline
                                    | TokenKind::Eof
                                    | TokenKind::RParen
                                    | TokenKind::RBracket
                                    | TokenKind::RBrace
                                    | TokenKind::Comma
                            )
                        );
                    }
                }
                TokenKind::Backtick => in_splice = true,
                TokenKind::Integer(_)
                | TokenKind::Comma
                | TokenKind::Dot
                | TokenKind::Ellipsis
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::LParen
                | TokenKind::RParen => {}
                _ if kind.word_text().is_some() => {}
                _ => return false,
            }
        }
        false
    }

    // `>>` lexes as one `ShiftRight` token, so closing two nested generic
    // lists at once (e.g. `Param<Reg<64>>`) needs it split back into two
    // `Greater` tokens.
    fn expect_generic_close(&mut self) -> Result<Token, ParseError> {
        if self.check(&TokenKind::ShiftRight) {
            let token = self.current().clone();
            let mid = token.span.start + 1;

            self.tokens[self.pos] = Token::new(TokenKind::Greater, mid, token.span.end);
            self.tokens.insert(self.pos, Token::new(TokenKind::Greater, token.span.start, mid));

            return Ok(self.advance().clone());
        }

        let token = self.current().clone();
        self.expect_simple(TokenKind::Greater)?;
        Ok(token)
    }

    // =============
    // token navigation
    // =============

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.at_eof() {
            self.pos += 1;
        }
        self.previous()
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        same_variant(&self.current().kind, kind)
    }

    // `.` immediately followed (no whitespace) by an identifier or keyword
    // — the spelling of a dotted label name like `.loop`.
    fn at_dot_identifier(&self) -> bool {
        let dot = self.current();

        matches!(dot.kind, TokenKind::Dot)
            && self.tokens.get(self.pos + 1).is_some_and(|token| {
                token.kind.word_text().is_some()
                    && token.span.start == dot.span.end
            })
    }

    // Consumes a dotted label name (`at_dot_identifier` must hold) and
    // returns it with its leading `.` kept.
    fn expect_dotted_name(&mut self) -> String {
        self.advance();

        let word = self.advance().kind.word_text().unwrap_or_default();

        format!(".{word}")
    }

    // `.name:` at statement start — a dotted label definition.
    fn at_dotted_label(&self) -> bool {
        self.at_dot_identifier()
            && self.tokens
                .get(self.pos + 2)
                .is_some_and(|token| matches!(token.kind, TokenKind::Colon))
    }

    fn check_next(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.pos + 1)
            .map(|token| same_variant(&token.kind, kind))
            .unwrap_or(false)
    }
}

// =============
// token comparison
// =============

fn same_variant(a: &TokenKind, b: &TokenKind) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}
