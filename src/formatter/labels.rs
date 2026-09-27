//! Which lines belong to a label's body, for `indent_label_bodies`: the
//! lines after a top-level label (`start:`, `pub _start:`, `.loop:`), up to
//! the next label, `section`, or declaration. A `const` doesn't end a body;
//! data often names its own length right under its label.

use crate::token::{Token, TokenKind};

use super::indentation::{update_delimiters, update_generic_depth};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Blank,
    Comment,
    Label,
    // A `section` line or a declaration: the end of any label body.
    Boundary,
    Code,
}

/// One flag per `\n`-separated line of `source`: whether it's part of a
/// label's body. A comment line counts when the code it precedes does, so a
/// comment above the next label stays with that label.
pub(super) fn label_body_lines(source: &str, tokens: &[Token]) -> Vec<bool> {
    let mut kinds = Vec::new();
    let mut delimiters = Vec::new();
    let mut generic_depth = 0usize;
    let mut offset = 0usize;

    for line in source.split('\n') {
        let end = offset + line.len();
        let line_tokens: Vec<&Token> = tokens
            .iter()
            .filter(|token| token.span.start >= offset && token.span.start < end)
            .filter(|token| token.kind != TokenKind::Newline)
            .collect();
        let content = line.trim();
        let top_level = delimiters.is_empty() && generic_depth == 0;

        kinds.push(if content.is_empty() {
            LineKind::Blank
        } else if content.starts_with('#') {
            LineKind::Comment
        } else if top_level && is_label(&line_tokens) {
            LineKind::Label
        } else if top_level && ends_label_body(&line_tokens) {
            LineKind::Boundary
        } else {
            LineKind::Code
        });

        update_delimiters(&mut delimiters, &line_tokens);
        update_generic_depth(&mut generic_depth, &line_tokens);
        offset = end + 1;
    }

    let mut in_body = false;
    let mut flags = Vec::with_capacity(kinds.len());

    for (index, kind) in kinds.iter().enumerate() {
        flags.push(match kind {
            LineKind::Label => {
                in_body = true;
                false
            }
            LineKind::Boundary => {
                in_body = false;
                false
            }
            LineKind::Code => in_body,
            LineKind::Blank => false,
            LineKind::Comment => {
                in_body
                    && kinds[index + 1..]
                        .iter()
                        .find(|kind| !matches!(kind, LineKind::Blank | LineKind::Comment))
                        .is_some_and(|kind| *kind == LineKind::Code)
            }
        });
    }

    flags
}

// `name:`, `.name:`, or either with `pub`, alone on its line.
fn is_label(tokens: &[&Token]) -> bool {
    let tokens = match tokens.first() {
        Some(token) if token.kind == TokenKind::Pub => &tokens[1..],
        _ => tokens,
    };

    match tokens {
        [name, colon] => {
            matches!(name.kind, TokenKind::Identifier(_)) && colon.kind == TokenKind::Colon
        }
        [dot, word, colon] => {
            dot.kind == TokenKind::Dot
                && dot.span.end == word.span.start
                && word.kind.word_text().is_some()
                && colon.kind == TokenKind::Colon
        }
        _ => false,
    }
}

// `section`, an import, or a `macro`/`struct`/`enum`/`type`/`syntax`
// declaration.
fn ends_label_body(tokens: &[&Token]) -> bool {
    let tokens = match tokens.first() {
        Some(token) if token.kind == TokenKind::Pub => &tokens[1..],
        _ => tokens,
    };

    match tokens {
        [first, ..] if matches!(
            first.kind,
            TokenKind::Section | TokenKind::From | TokenKind::Macro | TokenKind::Struct
                | TokenKind::Enum | TokenKind::Type
        ) => true,
        [first, name, open, ..] => {
            matches!(&first.kind, TokenKind::Identifier(word) if word == "syntax")
                && matches!(name.kind, TokenKind::Identifier(_))
                && open.kind == TokenKind::LParen
        }
        _ => false,
    }
}
