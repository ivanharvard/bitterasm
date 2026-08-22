//! `syntax` — macro-only, at most one. Lets a macro declare its own
//! call-site shape instead of the default bare `name arg, arg, ...` form,
//! e.g. `syntax "mov $dst$, $value$"`. A `$...$` pair is a *plain* capture:
//! parse an expression there, bind it to the parameter of that name — not a
//! richer "evaluate and splice the result" mechanism.
//!
//! This file only owns the pattern *data* and string-level parsing/validation
//! (no token-stream matching against a live parse — that needs `&mut
//! Parser`, and lives in `crate::parser::invocation_syntax` instead, the
//! same split `crate::facets` already keeps from `crate::parser::facets` for
//! every other facet). Unlike every other facet here, `syntax`'s parsed data
//! (not just its payload shape and cardinality rule) is consumed outside
//! this file — by the parser, to match call sites, and by the loader, to
//! thread patterns across file imports — so it's `pub(crate)`, not a bare
//! private `mod`.
//!
//! Two accepted v1 limitations: a pattern can't describe a call-site
//! literal `$` (any `$` starts or ends a capture), and within one file a
//! custom-syntax call site that textually precedes its own declaration can
//! fail to parse — the same-file prepass tolerates its own parse errors by
//! discarding them and moving on, so it usually still reaches (and
//! registers) a later declaration even after misreading an earlier,
//! not-yet-known custom-shaped call site under default rules; it only hard
//! stops when that default misreading itself can't consume the line (e.g. a
//! pattern separator, like `:`, that isn't a valid continuation of any
//! default expression — separators that happen to *look* like one, e.g.
//! `<-`, silently "work" regardless of ordering, for the wrong reason).

use crate::token::TokenKind;

use super::{DeclKind, PayloadShape, Violation};

pub const PAYLOAD: PayloadShape = PayloadShape::Expr;

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

/// Parses a `syntax` facet's string payload into a `SyntaxPattern`, and
/// validates it against `macro_name`/`params` (both already fully known by
/// the time a macro's facets are parsed). Errors are plain messages — like
/// `Violation`, a small facet-owned type the caller (the parser) turns into
/// a real `ParseError` with the facet's own span, not a computed offset
/// into the pattern string (`value` is already escape-collapsed by the
/// lexer, so a byte offset into it wouldn't line up with the original
/// source once any escape precedes the error point).
pub fn parse_pattern(
    macro_name: &str,
    value: &str,
    params: &[String],
) -> Result<SyntaxPattern, String> {
    let tokens: Vec<TokenKind> = crate::lexer::lex(value)
        .map_err(|error| format!("invalid syntax pattern: {error}"))?
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| !matches!(kind, TokenKind::Eof | TokenKind::Newline))
        .collect();

    let segments = split_into_segments(tokens)?;

    validate_starts_with_macro_name(&segments, macro_name)?;
    validate_captures(&segments, params)?;
    validate_no_empty_gaps(&segments)?;

    Ok(SyntaxPattern {
        segments,
        param_order: params.to_vec(),
    })
}

fn split_into_segments(tokens: Vec<TokenKind>) -> Result<Vec<PatternSegment>, String> {
    let mut segments = Vec::new();
    let mut literal = Vec::new();
    let mut iter = tokens.into_iter();

    while let Some(token) = iter.next() {
        if token != TokenKind::Dollar {
            literal.push(token);
            continue;
        }

        if !literal.is_empty() {
            segments.push(PatternSegment::Literal(std::mem::take(&mut literal)));
        }

        let name = match iter.next() {
            Some(TokenKind::Identifier(name)) => name,

            _ => {
                return Err(
                    "a `$...$` capture must contain exactly one identifier".to_string(),
                )
            }
        };

        match iter.next() {
            Some(TokenKind::Dollar) => {}

            _ => {
                return Err(
                    "a `$...$` capture must contain exactly one identifier \
                     (unterminated capture: missing closing `$`)"
                        .to_string(),
                )
            }
        }

        segments.push(PatternSegment::Capture(name));
    }

    if !literal.is_empty() {
        segments.push(PatternSegment::Literal(literal));
    }

    Ok(segments)
}

fn validate_starts_with_macro_name(
    segments: &[PatternSegment],
    macro_name: &str,
) -> Result<(), String> {
    let starts_with_name = matches!(
        segments.first(),
        Some(PatternSegment::Literal(tokens))
            if matches!(tokens.first(), Some(TokenKind::Identifier(name)) if name == macro_name)
    );

    if starts_with_name {
        Ok(())
    } else {
        Err(format!(
            "a syntax pattern must begin with the macro's own name, e.g. `{macro_name} ...`"
        ))
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
            0 => return Err(format!("parameter `{param}` is never captured (`${param}$`)")),
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
