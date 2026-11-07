//! Syntax patterns and the macro bodies that follow them.
//!
//! A pattern (`| syntax { ... }`, or the `{ ... }` of a standalone
//! `syntax name(...) = { ... }` override) is the call site as users will
//! type it, so the formatter never wraps or re-spaces inside one. Its
//! brackets are text to match, not lists.

use crate::facets::PayloadShape;
use crate::token::{Token, TokenKind};

use super::indentation::make_indent;
use super::wrapping::code_width;
use super::FormatConfig;

/// Marks which tokens on this line are inside a syntax pattern, braces
/// included. A pattern that continues past the line marks everything after
/// its opening brace.
pub(super) fn pattern_mask(tokens: &[&Token]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut index = 0;
    while index < tokens.len() {
        if let Some(open) = pattern_open(tokens, index) {
            let end = matching_brace(tokens, open).unwrap_or(tokens.len() - 1);
            mask[open..=end].iter_mut().for_each(|masked| *masked = true);
            index = end + 1;
        } else {
            index += 1;
        }
    }
    mask
}

/// The index of the `{` that opens a pattern starting at `tokens[index]`:
/// `| syntax {` or, on a line that starts with `syntax`, `= {`.
fn pattern_open(tokens: &[&Token], index: usize) -> Option<usize> {
    let is_syntax = |token: &Token| matches!(&token.kind, TokenKind::Identifier(word) if word == "syntax");
    let next_is_brace = tokens.get(index + 1).is_some_and(|token| token.kind == TokenKind::LBrace);
    let facet = index > 0
        && tokens[index - 1].kind == TokenKind::Pipe
        && is_syntax(tokens[index]);
    let assignment = tokens[index].kind == TokenKind::Equal
        && tokens.first().is_some_and(|token| is_syntax(token));
    ((facet || assignment) && next_is_brace).then_some(index + 1)
}

/// The index of the `}` closing the `{` at `open`, if it's on this line.
fn matching_brace(tokens: &[&Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.kind {
            TokenKind::LBrace => depth += 1,
            TokenKind::RBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Moves a macro body off a facet line onto lines of its own:
///
/// ```text
///     | syntax { ror $size$ $dst$, $src$ }
/// {
///     ror dst, src, size
/// }
/// ```
///
/// Only facets whose payload can't hold braces qualify: `syntax`, and the
/// type and bare ones like `emits`. A body that opens on the facet line and
/// continues below always moves.
/// One that also closes there stays put while the line fits `max_width`.
/// `decl_depth` is the declaration's own depth, one less than the facet's.
pub(super) fn split_facet_body(
    content: &str,
    tokens: &[&Token],
    line_offset: usize,
    line_depth: usize,
    decl_depth: usize,
    config: &FormatConfig,
) -> Option<Vec<String>> {
    if !tokens.first().is_some_and(|token| token.kind == TokenKind::Pipe) {
        return None;
    }
    let TokenKind::Identifier(name) = &tokens.get(1)?.kind else {
        return None;
    };
    // Where the body's `{` is. An expression payload can hold braces of its
    // own, so there's no telling where one of those ends.
    let body_open = match crate::facets::payload_shape(name)? {
        PayloadShape::Pattern => {
            let payload_end = match tokens.get(2)?.kind {
                TokenKind::LBrace => matching_brace(tokens, pattern_open(tokens, 1)?)?,
                TokenKind::String(_) => 2,
                _ => return None,
            };
            payload_end + 1
        }
        PayloadShape::Type | PayloadShape::Bare => {
            (2..tokens.len()).find(|index| tokens[*index].kind == TokenKind::LBrace)?
        }
        PayloadShape::Expr | PayloadShape::Block => return None,
    };
    if tokens.get(body_open)?.kind != TokenKind::LBrace {
        return None;
    }

    let source_start = tokens.first().map_or(line_offset, |token| token.span.start);
    let at = |index: usize| tokens[index].span.start - source_start;
    let facet_line = format!("{}{}", make_indent(line_depth, config), content[..at(body_open)].trim());
    let decl_indent = make_indent(decl_depth, config);

    match matching_brace(tokens, body_open) {
        None => {
            let rest = content[at(body_open) + 1..].trim();
            let mut lines = vec![facet_line, format!("{decl_indent}{{")];
            if !rest.is_empty() {
                lines.push(format!("{}{rest}", make_indent(decl_depth + 1, config)));
            }
            Some(lines)
        }
        Some(close) if close == tokens.len() - 1 => {
            let fits = make_indent(line_depth, config).chars().count()
                + code_width(content, tokens, line_offset)
                <= config.max_width;
            if fits {
                return None;
            }
            let inner = content[at(body_open) + 1..at(close)].trim();
            let mut lines = vec![facet_line, format!("{decl_indent}{{")];
            if !inner.is_empty() {
                lines.push(format!("{}{inner}", make_indent(decl_depth + 1, config)));
            }
            lines.push(format!("{decl_indent}}}"));
            Some(lines)
        }
        Some(_) => None,
    }
}
