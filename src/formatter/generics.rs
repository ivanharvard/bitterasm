use crate::lexer;

use super::indentation::update_generic_depth;
use super::FormatConfig;

/// Rejoins a generic argument list that fits on one line. This also repairs
/// lists split by formatter versions that treated generic commas as ordinary
/// call/list separators.
pub(super) fn collapse_short_generics(source: &str, config: &FormatConfig) -> Result<String, String> {
    if !config.collapse_short_multiline_generics {
        return Ok(source.to_string());
    }

    let tokens = lexer::lex(source).map_err(|error| format!("lex error: {error}"))?;
    let mut output = Vec::new();
    let mut group: Option<(String, Vec<String>)> = None;
    let mut generic_depth: usize = 0;
    let mut offset = 0;

    for line in source.split('\n') {
        let end = offset + line.len();
        let line_tokens: Vec<_> = tokens.iter()
            .filter(|token| token.span.start >= offset && token.span.start < end)
            .collect();
        let depth_before = generic_depth;
        update_generic_depth(&mut generic_depth, &line_tokens);

        if let Some((joined, originals)) = &mut group {
            if !line.trim().is_empty() {
                joined.push(' ');
                joined.push_str(line.trim());
            }
            originals.push(line.to_string());
            if generic_depth == 0 {
                let indent_len = originals[0].len() - originals[0].trim_start().len();
                if indent_len + joined.chars().count() <= config.max_width {
                    output.push(format!("{}{}", &originals[0][..indent_len], joined));
                } else {
                    output.append(originals);
                }
                group = None;
            }
        } else if depth_before == 0 && generic_depth > 0 {
            group = Some((line.trim().to_string(), vec![line.to_string()]));
        } else {
            output.push(line.to_string());
        }

        offset = end + 1;
    }

    if let Some((_, mut originals)) = group {
        output.append(&mut originals);
    }
    Ok(output.join("\n"))
}
