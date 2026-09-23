//! `syntax` — macro-only, at most one *as a facet*. Lets a macro declare
//! its own call-site shape instead of the default bare `name arg, arg, ...`
//! form, e.g. `syntax { mov $dst$, $value$ }`. A `$...$` pair is a *plain*
//! capture: parse an expression there, bind it to the parameter of that
//! name — not a richer "evaluate and splice the result" mechanism.
//!
//! The standalone form (`syntax name(a, b) = { ... }`, parsed in
//! `crate::parser::mod::parse_syntax_override`, not here) reuses this same
//! pattern grammar and [`parse_pattern`], but *overrides* an existing
//! macro's call-site shape from outside its own declaration, instead of
//! declaring one alongside it — see that function's doc for the
//! one-pattern-per-name/conflict rules that only apply to it.
//!
//! This file only owns the pattern *data* and token-level parsing/validation
//! (no token-stream matching against a live parse — that needs `&mut
//! Parser`, and lives in `crate::parser::invocation_syntax` instead, the
//! same split `crate::facets` already keeps from `crate::parser::facets` for
//! every other facet). Unlike every other facet here, `syntax`'s parsed data
//! (not just its payload shape and cardinality rule) is consumed outside
//! this file — by the parser, to match call sites, and by the loader, to
//! thread patterns across file imports — so it's `pub(crate)`, not a bare
//! private `mod`.
//!
//! One accepted v1 limitation: within one file a custom-syntax call site
//! that textually precedes its own declaration can fail to parse — the
//! same-file prepass tolerates its own parse errors by discarding them and
//! moving on, so it usually still reaches (and registers) a later
//! declaration even after misreading an earlier, not-yet-known
//! custom-shaped call site under default rules; it only hard stops when
//! that default misreading itself can't consume the line (e.g. a pattern
//! separator, like `:`, that isn't a valid continuation of any default
//! expression — separators that happen to *look* like one, e.g. `<-`,
//! silently "work" regardless of ordering, for the wrong reason).

use crate::token::TokenKind;

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Pattern;

pub fn check(decl_kind: DeclKind, count: usize) -> Result<(), Violation> {
    if decl_kind != DeclKind::Macro {
        return Err(Violation::NotApplicable);
    }

    if count > 1 {
        return Err(Violation::TooMany);
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternSegment {
    Literal(Vec<TokenKind>),
    Capture(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxPattern {
    pub segments: Vec<PatternSegment>,
    // Declared params, in declaration order — for reordering captured
    // expressions into positional `Invocation.operands`.
    pub param_order: Vec<String>,
}

/// Parses a `syntax` facet/override's raw `{ ... }` tokens into a
/// `SyntaxPattern`, and validates it against `params` (already known by the
/// time either form is parsed). `tokens` is already lexed by the
/// surrounding parse (a brace-delimited block of ordinary source tokens,
/// not a re-lexed string) — the caller is responsible for stripping the
/// delimiting braces and any `Newline`/`Eof` before calling this. Errors
/// are plain messages — like `Violation`, a small facet-owned type the
/// caller (the parser) turns into a real `ParseError` with the pattern's
/// own span.
///
/// Deliberately doesn't require the pattern to start with (or contain
/// anywhere) the macro's own name — `$rd$ = $rs1$ + $rs2$` is exactly as
/// valid as `add $rd$, $rs1$, $rs2$`, so a caller never has to know or
/// spell an instruction's name to use it. `crate::parser::is_anchored`
/// classifies a parsed pattern after the fact, for the one thing that
/// distinction still matters for: whether the parser can dispatch on it by
/// a call site's leading token alone, or has to try it unconditionally
/// alongside every other unanchored pattern (see that function's doc).
pub fn parse_pattern(tokens: Vec<TokenKind>, params: &[String]) -> Result<SyntaxPattern, String> {
    let segments = split_into_segments(tokens)?;

    validate_captures(&segments, params)?;
    validate_no_empty_gaps(&segments)?;

    Ok(SyntaxPattern {
        segments,
        param_order: params.to_vec(),
    })
}

/// Whether `pattern`'s first segment is a literal starting with `name` —
/// the parser's cheap, O(1) dispatch path (`crate::parser::statements`):
/// an anchored pattern is only ever tried when the current statement's
/// leading identifier already equals `name`, the same way looking a plain
/// invocation's callee up by name already works. An unanchored pattern
/// (starts with a capture, or a literal spelling something other than its
/// own macro's name) can't be found that way — nothing about the call
/// site's first token says which macro it's for — so it's tried against
/// *every* identifier-led statement instead, regardless of what that
/// statement's leading token is.
pub fn is_anchored(pattern: &SyntaxPattern, name: &str) -> bool {
    matches!(
        pattern.segments.first(),
        Some(PatternSegment::Literal(tokens))
            if matches!(tokens.first(), Some(TokenKind::Identifier(first)) if first == name)
    )
}

/// Whether `pattern` is an *operand* pattern — one whose first token is a
/// literal that can't start a statement (anything but an identifier or
/// keyword, e.g. `[rel $target$]`). Every statement a custom syntax can
/// match starts with a word, so such a pattern could never match one;
/// instead the parser tries it wherever an operand expression starts, and
/// a match becomes an ordinary call to the pattern's macro (`rel(msg)`),
/// so it's only meaningful on a macro that `@return`s a value. Operand
/// patterns share the unanchored pool (threaded across imports the same
/// way), but statement matching skips them.
pub fn is_operand_pattern(pattern: &SyntaxPattern) -> bool {
    matches!(
        pattern.segments.first(),
        Some(PatternSegment::Literal(tokens))
            if tokens.first().is_some_and(|first| first.word_text().is_none())
    )
}

fn split_into_segments(tokens: Vec<TokenKind>) -> Result<Vec<PatternSegment>, String> {
    let mut segments = Vec::new();
    let mut literal = Vec::new();
    let mut iter = tokens.into_iter();

    while let Some(token) = iter.next() {
        if token != TokenKind::Dollar {
            literal.push(unescape_literal_token(token)?);
            continue;
        }

        if !literal.is_empty() {
            segments.push(PatternSegment::Literal(std::mem::take(&mut literal)));
        }

        let name = match iter.next() {
            Some(TokenKind::Identifier(name)) => name,

            _ => return Err("a `$...$` capture must contain exactly one identifier".to_string()),
        };

        match iter.next() {
            Some(TokenKind::Dollar) => {}

            _ => {
                return Err("a `$...$` capture must contain exactly one identifier \
                     (unterminated capture: missing closing `$`)"
                    .to_string())
            }
        }

        segments.push(PatternSegment::Capture(name));
    }

    if !literal.is_empty() {
        segments.push(PatternSegment::Literal(literal));
    }

    Ok(segments)
}

fn unescape_literal_token(token: TokenKind) -> Result<TokenKind, String> {
    match token {
        TokenKind::Escaped('$') => Ok(TokenKind::Dollar),
        TokenKind::Escaped('`') => Ok(TokenKind::Backtick),
        TokenKind::Escaped(ch) => Err(format!("unknown escaped syntax-pattern literal `\\{ch}`")),
        other => Ok(other),
    }
}

fn validate_captures(segments: &[PatternSegment], params: &[String]) -> Result<(), String> {
    let captures: Vec<&str> = segments
        .iter()
        .filter_map(|segment| match segment {
            PatternSegment::Capture(name) => Some(name.as_str()),
            PatternSegment::Literal(_) => None,
        })
        .collect();

    for name in &captures {
        if !params.iter().any(|param| param == name) {
            return Err(format!("`${name}$` doesn't name a declared parameter"));
        }
    }

    for param in params {
        let count = captures.iter().filter(|name| **name == param).count();

        match count {
            0 => {
                return Err(format!(
                    "parameter `{param}` is never captured (`${param}$`)"
                ))
            }
            1 => {}
            _ => return Err(format!("parameter `{param}` is captured more than once")),
        }
    }

    Ok(())
}

fn validate_no_empty_gaps(segments: &[PatternSegment]) -> Result<(), String> {
    for window in segments.windows(2) {
        if let [PatternSegment::Capture(a), PatternSegment::Capture(b)] = window {
            return Err(format!(
                "`${a}$` and `${b}$` need a literal between them to tell where one ends \
                 and the other begins"
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_dollar_in_pattern_is_a_literal_token_not_a_capture_delimiter() {
        let params = vec!["imm".to_string()];
        let pattern = parse_pattern(
            vec![
                TokenKind::Identifier("mov".to_string()),
                TokenKind::Escaped('$'),
                TokenKind::Dollar,
                TokenKind::Identifier("imm".to_string()),
                TokenKind::Dollar,
            ],
            &params,
        )
        .expect("escaped dollar should parse as a literal pattern token");

        assert_eq!(
            pattern.segments,
            vec![
                PatternSegment::Literal(vec![
                    TokenKind::Identifier("mov".to_string()),
                    TokenKind::Dollar,
                ]),
                PatternSegment::Capture("imm".to_string()),
            ],
        );
    }
}
