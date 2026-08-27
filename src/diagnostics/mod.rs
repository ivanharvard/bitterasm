mod diagnostic;
mod errors;
mod lint;
mod renderer;
mod source_map;

pub use diagnostic::{Diagnostic, Label, LabelStyle, Severity};
pub use errors::{lex_error, load_error, parse_error, resolve_error};
pub use lint::{lint_program, load_lint_config, LintConfig, LintLevel, LintName};
pub use renderer::{render, DiagnosticFormat, RenderOptions};
pub use source_map::{SourceFile, SourceId, SourceMap};
