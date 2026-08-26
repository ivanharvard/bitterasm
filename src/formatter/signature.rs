use super::FormatConfig;

/// Moves legacy piped macro signature fields back onto the declaration.
/// Genuine facets remain as `| ...` continuation lines.
pub(super) fn normalize_macro_signatures(source: &str, config: &FormatConfig) -> String {
    if !config.pub_on_declaration && !config.return_type_on_declaration {
        return source.to_string();
    }

    let mut lines: Vec<String> = Vec::new();
    let mut macro_line = None;

    for line in source.split('\n') {
        let trimmed = line.trim();
        if is_macro_declaration(trimmed) {
            lines.push(line.to_string());
            macro_line = Some(lines.len() - 1);
            continue;
        }

        if let Some(index) = macro_line {
            if config.pub_on_declaration && trimmed == "| pub" {
                add_pub(&mut lines[index]);
                continue;
            }
            if config.return_type_on_declaration {
                if let Some(return_type) = trimmed.strip_prefix("| ->") {
                    lines[index].push_str(" ->");
                    lines[index].push_str(return_type);
                    continue;
                }
            }

            if trimmed.starts_with('|') || trimmed.is_empty() {
                lines.push(line.to_string());
                continue;
            }
            macro_line = None;
        }

        lines.push(line.to_string());
    }

    lines.join("\n")
}

fn is_macro_declaration(line: &str) -> bool {
    line.starts_with("macro ") || line.starts_with("pub macro ")
}

fn add_pub(line: &mut String) {
    let content_start = line.len() - line.trim_start().len();
    if !line[content_start..].starts_with("pub ") {
        line.insert_str(content_start, "pub ");
    }
}
