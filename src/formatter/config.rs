use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatConfig {
    pub indent_width: usize,
    pub hard_tabs: bool,
    pub indent_facets: bool,
    pub facets_on_new_line: bool,
    pub pub_on_declaration: bool,
    pub return_type_on_declaration: bool,
    pub collapse_short_multiline_generics: bool,
    pub max_blank_lines: usize,
    pub max_width: usize,
    pub comment_width: usize,
    pub newline_style: NewlineStyle,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent_width: 4,
            hard_tabs: false,
            indent_facets: true,
            facets_on_new_line: true,
            pub_on_declaration: true,
            return_type_on_declaration: true,
            collapse_short_multiline_generics: true,
            max_blank_lines: 1,
            max_width: 100,
            comment_width: 80,
            newline_style: NewlineStyle::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlineStyle {
    Auto,
    Unix,
    Windows,
}

pub fn discover_config(start: &Path) -> Option<PathBuf> {
    let start = if start.is_dir() { start } else { start.parent()? };
    for directory in start.ancestors() {
        for name in ["bitterasm.toml", ".bitterasm.toml"] {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn load_config(path: &Path) -> Result<FormatConfig, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    parse_config(&source)
        .map_err(|error| format!("invalid formatter config {}: {error}", path.display()))
}

pub(super) fn parse_config(source: &str) -> Result<FormatConfig, String> {
    let mut config = FormatConfig::default();
    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let (key, raw_value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {line_number}: expected `key = value`"))?;
        let key = key.trim();
        let value = raw_value.trim().trim_matches('"');
        match key {
            "indent_width" => config.indent_width = parse_usize(value, line_number, key)?,
            "hard_tabs" => config.hard_tabs = parse_bool(value, line_number, key)?,
            "indent_facets" => config.indent_facets = parse_bool(value, line_number, key)?,
            "facets_on_new_line" => config.facets_on_new_line = parse_bool(value, line_number, key)?,
            "pub_on_declaration" => config.pub_on_declaration = parse_bool(value, line_number, key)?,
            "return_type_on_declaration" => {
                config.return_type_on_declaration = parse_bool(value, line_number, key)?
            }
            "collapse_short_multiline_generics" => {
                config.collapse_short_multiline_generics = parse_bool(value, line_number, key)?
            }
            "max_blank_lines" => config.max_blank_lines = parse_usize(value, line_number, key)?,
            "max_width" => config.max_width = parse_positive_usize(value, line_number, key)?,
            "comment_width" => config.comment_width = parse_positive_usize(value, line_number, key)?,
            "newline_style" => config.newline_style = match value {
                "Auto" => NewlineStyle::Auto,
                "Unix" => NewlineStyle::Unix,
                "Windows" => NewlineStyle::Windows,
                _ => return Err(format!("line {line_number}: `{key}` must be Auto, Unix, or Windows")),
            },
            _ => return Err(format!("line {line_number}: unknown option `{key}`")),
        }
    }
    Ok(config)
}

fn parse_bool(value: &str, line: usize, key: &str) -> Result<bool, String> {
    value.parse().map_err(|_| format!("line {line}: `{key}` must be true or false"))
}

fn parse_usize(value: &str, line: usize, key: &str) -> Result<usize, String> {
    value.parse().map_err(|_| format!("line {line}: `{key}` must be a non-negative integer"))
}

fn parse_positive_usize(value: &str, line: usize, key: &str) -> Result<usize, String> {
    let value = parse_usize(value, line, key)?;
    if value == 0 {
        return Err(format!("line {line}: `{key}` must be greater than zero"));
    }
    Ok(value)
}
