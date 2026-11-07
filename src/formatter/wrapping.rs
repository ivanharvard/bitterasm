use crate::token::{Token, TokenKind};

use super::indentation::{leading_closers, leading_generic_closers, make_indent};
use super::patterns::pattern_mask;
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

/// Formats one `##`/`#!` line. Doc text is Markdown, so unlike an
/// ordinary comment its spacing means something: fenced code (tracked
/// across lines in `in_fence`), tables and headings are kept exactly as
/// written, and prose is only re-wrapped when a line is over `width`,
/// with continuation lines keeping its indentation.
pub(super) fn wrap_doc_comment(
    comment: &str,
    indent: &str,
    width: usize,
    in_fence: &mut bool,
) -> Vec<String> {
    let Some((is_module, text)) = crate::lexer::doc_comment(comment) else {
        return wrap_comment(comment, indent, width);
    };
    let marker = if is_module { "#!" } else { "##" };
    let text = text.trim_end();
    if text.is_empty() {
        return vec![format!("{indent}{marker}")];
    }

    let prefix = format!("{indent}{marker} ");
    let content = text.trim_start();
    let is_fence = content.starts_with("```") || content.starts_with("~~~");
    let verbatim = *in_fence || is_fence || content.starts_with('|') || content.starts_with('#');
    if is_fence {
        *in_fence = !*in_fence;
    }
    if verbatim || prefix.chars().count() + text.chars().count() <= width {
        return vec![format!("{prefix}{text}")];
    }

    let leading = &text[..text.len() - content.len()];
    let available = width.saturating_sub(prefix.chars().count() + leading.chars().count()).max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in content.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > available {
            lines.push(format!("{prefix}{leading}{current}"));
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    lines.push(format!("{prefix}{leading}{current}"));
    lines
}

/// How wide the code on a line is, leaving out a trailing comment: a
/// comment lined up with the ones around it may run past `max_width`.
pub(super) fn code_width(content: &str, tokens: &[&Token], line_offset: usize) -> usize {
    let source_start = tokens.first().map_or(line_offset, |token| token.span.start);
    let end = tokens.last().map_or(content.len(), |token| token.span.end - source_start);
    content[..end.min(content.len())].chars().count()
}

/// What's open around a line when it starts.
pub(super) struct Surroundings<'a> {
    /// The brackets still open from earlier lines, outermost first.
    pub outer: &'a [TokenKind],
    /// How many of `outer` share a line with the one before them, and so
    /// add no indentation.
    pub shared: usize,
    pub generic_depth: usize,
    /// Indentation levels added on top, for facets, label bodies and
    /// statement continuations.
    pub depth_bias: usize,
}

pub(super) fn wrap_code(
    content: &str,
    tokens: &[&Token],
    line_offset: usize,
    around: &Surroundings,
    config: &FormatConfig,
) -> Vec<String> {
    let Surroundings { outer, shared, generic_depth: outer_generic_depth, depth_bias } = *around;
    let leading_closers = leading_closers(tokens);
    let leading_generic_closers = leading_generic_closers(tokens);
    let base_depth = outer.len().saturating_sub(leading_closers + shared)
        + outer_generic_depth.saturating_sub(leading_generic_closers)
        + depth_bias;
    let first_indent = make_indent(base_depth, config);
    let source_start = tokens.first().map_or(line_offset, |token| token.span.start);
    let mut local = outer.to_vec();
    let mut generic_depth = outer_generic_depth;
    let mut previous: Option<&Token> = None;
    let mut breaks = Vec::new();
    let mut closings = Vec::new();
    // Which list each comma belongs to: the position of the bracket opened
    // on this line that encloses it, or `None` for a list opened on an
    // earlier line. Two lists at the same depth, like the closed `(...)` and
    // the open `{` of `macro f(a, b) {`, are different lists.
    let mut open_here: Vec<usize> = Vec::new();
    let mut break_lists: Vec<Option<usize>> = Vec::new();
    // Where each bracket opened on this line closes, by its opener's start.
    let mut closed_at: Vec<(usize, usize)> = Vec::new();
    let in_pattern = pattern_mask(tokens);
    for (token, _) in tokens.iter().zip(&in_pattern).filter(|(_, masked)| !**masked) {
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
                open_here.push(token.span.start);
            }
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                let position = token.span.start.saturating_sub(source_start);
                closings.push((position, local.len()));
                // Brackets nest, so this closes the innermost one opened on
                // this line, or else one opened earlier.
                if let Some(opener) = open_here.pop() {
                    closed_at.push((opener, position));
                }
                local.pop();
            }
            TokenKind::Comma
                if generic_depth == 0
                    && (matches!(local.last(), Some(TokenKind::LParen | TokenKind::LBracket))
                        || matches!(local.last(), Some(TokenKind::LBrace))
                            && local.len() > outer.len()) =>
            {
                breaks.push((token.span.end.saturating_sub(source_start), local.len()));
                break_lists.push(open_here.last().copied());
            }
            _ => {}
        }
        previous = Some(token);
    }

    // A line that opens a list and leaves it open after one of its commas
    // continues that list on the next lines, so it's laid out one item per
    // line even when it fits. A comma inside a list that's already closed
    // again doesn't count.
    let starts_multiline_list = break_lists
        .iter()
        .any(|list| list.is_some_and(|opener| open_here.contains(&opener)));
    if first_indent.chars().count() + code_width(content, tokens, line_offset) <= config.max_width
        && !starts_multiline_list
    {
        // A line that closes a list opened on an earlier line, after other
        // code, puts that closer on its own line: `) -> int) -> int {`
        // becomes `) -> int` and `) -> int {`. Leading closers don't count;
        // they already start the line.
        let still_open = outer.len().saturating_sub(leading_closers);
        if let Some((position, depth)) = closings.iter().find(|(_, depth)| *depth <= still_open) {
            if !content[..*position].trim().is_empty() {
                return vec![
                    format!("{first_indent}{}", content[..*position].trim()),
                    format!(
                        "{}{}",
                        make_indent(depth.saturating_sub(1 + shared) + depth_bias, config),
                        content[*position..].trim()
                    ),
                ];
            }
        }
        return vec![format!("{first_indent}{content}")];
    }
    if breaks.is_empty() {
        return vec![format!("{first_indent}{content}")];
    }

    // Break the outermost list that has commas. Other brackets at the same
    // depth, like the `(x)` in `f(x) + g(a, b)`, aren't that list.
    let list_depth = breaks.iter().map(|(_, depth)| *depth).min().unwrap();
    let list = breaks.iter()
        .zip(&break_lists)
        .find(|((_, depth), _)| *depth == list_depth)
        .and_then(|(_, list)| *list);
    let list_breaks: Vec<_> = breaks.iter()
        .zip(&break_lists)
        .filter(|((_, depth), owner)| *depth == list_depth && **owner == list)
        .map(|((position, _), _)| *position)
        .collect();
    let (body_start, body_end) = match list {
        Some(opener) => (
            opener.saturating_sub(source_start) + 1,
            closed_at.iter()
                .find(|(open, _)| *open == opener)
                .map_or(content.len(), |(_, close)| *close),
        ),
        None => (
            0,
            closings.iter()
                .find(|(position, depth)| *depth == list_depth && *position > 0)
                .map_or(content.len(), |(position, _)| *position),
        ),
    };

    let mut lines = Vec::new();
    if body_start > 0 {
        lines.push(format!("{first_indent}{}", content[..body_start].trim()));
    }
    let item_indent = make_indent(list_depth.saturating_sub(shared) + depth_bias, config);
    let mut start = body_start;
    for end in list_breaks.into_iter().filter(|end| *end <= body_end) {
        lines.push(format!("{item_indent}{}", content[start..end].trim()));
        start = end;
    }
    if !content[start..body_end].trim().is_empty() {
        lines.push(format!("{item_indent}{}", content[start..body_end].trim()));
    }
    if body_end < content.len() {
        // Indent the rest the way it would be as a line of its own: one
        // level out for each closer it starts with, so `))` closing two
        // lists lands where a second formatting pass would put it.
        let closers = tokens
            .iter()
            .skip_while(|token| token.span.start.saturating_sub(source_start) < body_end)
            .take_while(|token| matches!(token.kind, TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace))
            .count()
            .max(1);
        let closing_indent = make_indent(list_depth.saturating_sub(closers + shared) + depth_bias, config);
        lines.push(format!("{closing_indent}{}", content[body_end..].trim()));
    }
    lines
}
