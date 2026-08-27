use super::{Diagnostic, LabelStyle, Severity, SourceMap};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFormat {
    Terminal,
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub format: DiagnosticFormat,
    pub color: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { format: DiagnosticFormat::Plain, color: false }
    }
}

pub fn render(diagnostics: &[Diagnostic], sources: &SourceMap, options: RenderOptions) -> String {
    if options.format == DiagnosticFormat::Json {
        #[derive(Serialize)]
        struct JsonDiagnostic<'a> {
            #[serde(flatten)]
            diagnostic: &'a Diagnostic,
            locations: Vec<JsonLocation>,
        }
        #[derive(Serialize)]
        struct JsonLocation {
            path: String,
            line: usize,
            column: usize,
        }
        let values: Vec<_> = diagnostics
            .iter()
            .map(|diagnostic| JsonDiagnostic {
                diagnostic,
                locations: diagnostic.labels.iter().filter_map(|label| {
                    let file = sources.get(label.source)?;
                    let (line, column) = file.line_column(label.span.start);
                    Some(JsonLocation { path: file.name.display().to_string(), line, column })
                }).collect(),
            })
            .collect();
        return serde_json::to_string_pretty(&values).unwrap_or_else(|error| {
            format!(r#"[{{"severity":"error","message":"failed to serialize diagnostics: {error}"}}]"#)
        });
    }

    let mut output = String::new();
    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        };
        let severity = if options.color && options.format == DiagnosticFormat::Terminal {
            let color = if diagnostic.severity == Severity::Error { "31" } else { "33" };
            format!("\x1b[{color};1m{severity}\x1b[0m")
        } else {
            severity.to_string()
        };
        output.push_str(&format!("{severity}: {}", diagnostic.message));
        if let Some(lint) = diagnostic.lint {
            output.push_str(&format!(" [{}]", lint.as_str()));
        }
        output.push('\n');

        for label in &diagnostic.labels {
            let Some(file) = sources.get(label.source) else { continue };
            let (line, column) = file.line_column(label.span.start);
            output.push_str(&format!("  --> {}:{line}:{column}\n", file.name.display()));
            if let Some(text) = file.line(line) {
                let line_width = line.to_string().len();
                output.push_str(&format!("{:line_width$} |\n", ""));
                output.push_str(&format!("{line} | {text}\n"));
                let line_start = file.line_start(line).unwrap_or(0);
                let start = file.source[line_start..label.span.start.min(file.source.len())].chars().count();
                let end = label.span.end.min(file.source.len());
                let width = file.source[label.span.start.min(end)..end].chars().count().max(1);
                let marker = if label.style == LabelStyle::Primary { '^' } else { '-' };
                output.push_str(&format!(
                    "{:line_width$} | {}{}",
                    "",
                    " ".repeat(start),
                    marker.to_string().repeat(width),
                ));
                if let Some(message) = &label.message {
                    output.push(' ');
                    output.push_str(message);
                }
                output.push('\n');
            }
        }
        for note in &diagnostic.notes {
            output.push_str(&format!("  = note: {note}\n"));
        }
        for help in &diagnostic.help {
            output.push_str(&format!("  = help: {help}\n"));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Diagnostic, LintName};
    use crate::token::Span;

    #[test]
    fn plain_renderer_includes_location_excerpt_and_lint_name() {
        let mut sources = SourceMap::default();
        let source = sources.add("demo.basm", "macro f(value: int) {\n}\n".to_string());
        let diagnostic = Diagnostic::warning(LintName::UNUSED_PARAMETER, "unused parameter")
            .primary(source, Span::new(8, 18), "unused here");
        let rendered = render(&[diagnostic], &sources, RenderOptions::default());
        assert!(rendered.contains("warning: unused parameter [unused_parameter]"));
        assert!(rendered.contains("demo.basm:1:9"));
        assert!(rendered.contains("unused here"));
    }

    #[test]
    fn json_renderer_is_machine_readable() {
        let mut sources = SourceMap::default();
        let source = sources.add("demo.basm", "x\n".to_string());
        let diagnostic = Diagnostic::warning(LintName::UNUSED_PARAMETER, "unused")
            .primary(source, Span::new(0, 1), "unused");
        let rendered = render(
            &[diagnostic],
            &sources,
            RenderOptions { format: DiagnosticFormat::Json, color: false },
        );
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed[0]["locations"][0]["line"], 1);
    }
}
