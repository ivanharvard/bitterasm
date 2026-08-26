use crate::token::{Token, TokenKind};

use super::FormatConfig;

pub(super) fn make_indent(depth: usize, config: &FormatConfig) -> String {
    if config.hard_tabs {
        "\t".repeat(depth)
    } else {
        " ".repeat(depth.saturating_mul(config.indent_width))
    }
}

pub(super) fn leading_closers(tokens: &[&Token]) -> usize {
    tokens.iter().take_while(|token| {
        matches!(token.kind, TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace)
    }).count()
}

pub(super) fn leading_generic_closers(tokens: &[&Token]) -> usize {
    match tokens.first().map(|token| &token.kind) {
        Some(TokenKind::Greater) => 1,
        Some(TokenKind::ShiftRight) => 2,
        _ => 0,
    }
}

pub(super) fn is_facet(tokens: &[&Token]) -> bool {
    tokens.first().is_some_and(|token| token.kind == TokenKind::Pipe)
}

pub(super) fn contains_facet(tokens: &[&Token]) -> bool {
    tokens.windows(2).any(|pair| {
        pair[0].kind == TokenKind::Pipe
            && matches!(&pair[1].kind, TokenKind::Identifier(name) if crate::facets::payload_shape(name).is_some())
    })
}

pub(super) fn update_delimiters(delimiters: &mut Vec<TokenKind>, tokens: &[&Token]) {
    for token in tokens {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                delimiters.push(token.kind.clone());
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                delimiters.pop();
            }
            _ => {}
        }
    }
}

pub(super) fn update_generic_depth(depth: &mut usize, tokens: &[&Token]) {
    let mut previous: Option<&Token> = None;
    for token in tokens {
        match token.kind {
            TokenKind::Less if previous.is_some_and(|previous| {
                previous.span.end == token.span.start
                    && matches!(previous.kind, TokenKind::Identifier(_) | TokenKind::Greater | TokenKind::ShiftRight)
            }) => *depth += 1,
            TokenKind::Greater if *depth > 0 => *depth -= 1,
            TokenKind::ShiftRight if *depth > 0 => *depth = depth.saturating_sub(2),
            _ => {}
        }
        previous = Some(token);
    }
}
