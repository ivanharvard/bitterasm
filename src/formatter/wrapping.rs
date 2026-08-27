use crate::token::{Token, TokenKind};

use super::indentation::{leading_closers, leading_generic_closers, make_indent};
use super::FormatConfig;

pub(super) fn split_inline_facets(
    content: &str,
    tokens: &[&Token],
    line_offset: usize,
    base_depth: usize,
    config: &FormatConfig,
) -> Option<Vec<String>> {
    if !config.facets_on_new_line {
        return None;
    }
    let source_start = tokens.first().map_or(line_offset, |token| token.span.start);
    let splits: Vec<_> = tokens.windows(2)
        .filter(|pair| {
            pair[0].kind == TokenKind::Pipe
                && matches!(&pair[1].kind, TokenKind::Identifier(name) if crate::facets::payload_shape(name).is_some())
        })
        .map(|pair| pair[0].span.start.saturating_sub(source_start))
        .filter(|position| *position > 0 && *position < content.len())
        .collect();
    if splits.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    let mut start = 0;
    for (index, end) in splits.iter().copied().chain(std::iter::once(content.len())).enumerate() {
        let depth = base_depth + usize::from(index > 0 && config.indent_facets);
        lines.push(format!("{}{}", make_indent(depth, config), content[start..end].trim()));
        start = end;
    }
    Some(lines)
}

pub(super) fn wrap_comment(comment: &str, indent: &str, width: usize) -> Vec<String> {
    let text = comment.strip_prefix('#').unwrap_or(comment).trim();
    let prefix = format!("{indent}#");
    if text.is_empty() {
        return vec![prefix];
    }
    let prefix = format!("{prefix} ");
    let available = width.saturating_sub(prefix.chars().count()).max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > available {
            lines.push(format!("{prefix}{current}"));
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    lines.push(format!("{prefix}{current}"));
    lines
}

pub(super) fn wrap_code(
    content: &str,
    tokens: &[&Token],
    line_offset: usize,
    outer: &[TokenKind],
    outer_generic_depth: usize,
    depth_bias: usize,
    config: &FormatConfig,
) -> Vec<String> {
    let leading_closers = leading_closers(tokens);
    let leading_generic_closers = leading_generic_closers(tokens);
    let base_depth = outer.len().saturating_sub(leading_closers)
        + outer_generic_depth.saturating_sub(leading_generic_closers)
        + depth_bias;
    let first_indent = make_indent(base_depth, config);
    let source_start = tokens.first().map_or(line_offset, |token| token.span.start);
    let mut local = outer.to_vec();
    let mut generic_depth = outer_generic_depth;
    let mut previous: Option<&Token> = None;
    let mut breaks = Vec::new();
    let mut openings = Vec::new();
    let mut closings = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::Less if previous.is_some_and(|previous| {
                previous.span.end == token.span.start
                    && matches!(previous.kind, TokenKind::Identifier(_) | TokenKind::Greater | TokenKind::ShiftRight)
            }) => generic_depth += 1,
            TokenKind::Greater if generic_depth > 0 => generic_depth -= 1,
            TokenKind::ShiftRight if generic_depth > 0 => {
                generic_depth = generic_depth.saturating_sub(2);
            }
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                local.push(token.kind.clone());
                openings.push((token.span.end.saturating_sub(source_start), local.len()));
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                closings.push((token.span.start.saturating_sub(source_start), local.len()));
                local.pop();
            }
            TokenKind::Comma
                if generic_depth == 0
                    && (matches!(local.last(), Some(TokenKind::LParen | TokenKind::LBracket))
                        || matches!(local.last(), Some(TokenKind::LBrace))
                            && local.len() > outer.len()) =>
            {
                breaks.push((token.span.end.saturating_sub(source_start), local.len()));
            }
            _ => {}
        }
        previous = Some(token);
    }

    let starts_multiline_list = local.len() > outer.len() && !breaks.is_empty();
    if first_indent.chars().count() + content.chars().count() <= config.max_width
        && !starts_multiline_list
    {
        if leading_closers == 0 {
            if let Some((position, depth)) = closings.iter().find(|(_, depth)| *depth <= outer.len()) {
                if !content[..*position].trim().is_empty() {
                    return vec![
                        format!("{first_indent}{}", content[..*position].trim()),
                        format!(
                            "{}{}",
                            make_indent(depth.saturating_sub(1) + depth_bias, config),
                            content[*position..].trim()
                        ),
                    ];
                }
            }
        }
        return vec![format!("{first_indent}{content}")];
    }
    if breaks.is_empty() {
        return vec![format!("{first_indent}{content}")];
    }

    let list_depth = breaks.iter().map(|(_, depth)| *depth).min().unwrap();
    let list_breaks: Vec<_> = breaks.iter()
        .filter(|(_, depth)| *depth == list_depth)
        .map(|(position, _)| *position)
        .collect();
    let body_start = openings.iter()
        .find(|(_, depth)| *depth == list_depth)
        .map_or(0, |(position, _)| *position);
    let body_end = closings.iter()
        .find(|(position, depth)| *depth == list_depth && *position > body_start)
        .map_or(content.len(), |(position, _)| *position);

    let mut lines = Vec::new();
    if body_start > 0 {
        lines.push(format!("{first_indent}{}", content[..body_start].trim()));
    }
    let item_indent = make_indent(list_depth + depth_bias, config);
    let mut start = body_start;
    for end in list_breaks.into_iter().filter(|end| *end <= body_end) {
        lines.push(format!("{item_indent}{}", content[start..end].trim()));
        start = end;
    }
    if !content[start..body_end].trim().is_empty() {
        lines.push(format!("{item_indent}{}", content[start..body_end].trim()));
    }
    if body_end < content.len() {
        let closing_indent = make_indent(list_depth.saturating_sub(1) + depth_bias, config);
        lines.push(format!("{closing_indent}{}", content[body_end..].trim()));
    }
    lines
}
