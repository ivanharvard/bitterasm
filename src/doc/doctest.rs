//! Running the examples in documentation: the book's pages
//! (`tests/book.rs`) and doc comments (`bitterasm doc --test`) alike, so
//! both follow one set of rules. Those rules are described for authors in
//! `docs/book/src/contributing/writing-docs.md`:
//!
//! - ```` ```basm ```` must compile with every lint denied, except
//!   `generated_declarations`.
//! - ```` ```basm,fail ```` must fail to compile.
//! - ```` ```basm,ignore ```` is a fragment, highlighted but not compiled.
//! - ```` ```basm,file=path.basm ```` isn't compiled itself: it's written to
//!   `path.basm` next to the later examples, so they can import it.
//!
//! Directly after a compiled `basm` block, any of:
//!
//! - ```` ```emits ```` lists what it must emit, in order: integers separated
//!   by spaces, or one struct or enum per line, written
//!   `Point { x: 1, y: 2 }`, `bits<8> { value: 65 }` or `Shape.Circle(2)`.
//! - ```` ```bytes ```` lists, in hex, the bytes `bitter encode` packs its
//!   output into.
//! - ```` ```error ```` (after `basm,fail`) holds text the compiler's error
//!   must contain.
//!
//! Examples are compiled by running `bitterasm` itself, from a working
//! directory whose `std` (and other absolute imports) they can use.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::emit::{EmFile, EmittedGenericArg, EmittedType, EmittedValue};

const EXPECTATIONS: &[&str] = &["emits", "bytes", "error"];

/// One fenced code block. `location` is `file:line`, for reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub location: String,
    pub info: String,
    pub body: String,
}

impl Block {
    pub fn language(&self) -> &str {
        self.info.split(',').next().unwrap_or("").trim()
    }

    fn attributes(&self) -> impl Iterator<Item = &str> {
        self.info.split(',').skip(1).map(str::trim)
    }

    fn has(&self, attribute: &str) -> bool {
        self.attributes().any(|a| a == attribute)
    }

    fn file(&self) -> Option<&str> {
        self.attributes().find_map(|a| a.strip_prefix("file="))
    }
}

/// Every fenced code block in `text`, in order. Fences may be indented
/// (inside a list item); the body keeps its lines relative to that
/// indentation. `location(n)` names the 0-based line `n`, for reporting.
pub fn fenced_blocks(text: &str, location: impl Fn(usize) -> String) -> Result<Vec<Block>, String> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, usize, String, String)> = None;

    for (number, line) in text.lines().enumerate() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        match &mut open {
            None => {
                if let Some(info) = trimmed.strip_prefix("```") {
                    open = Some((number, indent, info.trim().to_owned(), String::new()));
                }
            }
            Some((start, fence_indent, info, body)) => {
                if trimmed.starts_with("```") {
                    blocks.push(Block {
                        location: location(*start),
                        info: std::mem::take(info),
                        body: std::mem::take(body),
                    });
                    open = None;
                } else {
                    let strip = indent.min(*fence_indent);
                    body.push_str(&line[strip..]);
                    body.push('\n');
                }
            }
        }
    }

    match open {
        Some((start, ..)) => Err(format!("{}: unclosed code fence", location(start))),
        None => Ok(blocks),
    }
}

/// In a doc comment, a fence with no language is BitterASM, as in rustdoc,
/// and `ignore`, `fail` and `file=` may be written without it.
pub fn doc_comment_info(info: &str) -> String {
    let info = info.trim();
    let first = info.split(',').next().unwrap_or("").trim();
    if first.is_empty() {
        "basm".to_string()
    } else if first == "ignore" || first == "fail" || first.starts_with("file=") {
        format!("basm,{info}")
    } else {
        info.to_string()
    }
}

/// How to compile examples.
#[derive(Debug, Clone)]
pub struct Runner {
    /// The `bitterasm` binary to compile with.
    pub bitterasm: PathBuf,
    /// The `bitter` binary, for ```` ```bytes ```` checks.
    pub bitter: Option<PathBuf>,
    /// Where examples are compiled from, so `from std...` finds `std`.
    pub cwd: PathBuf,
}

/// The outcome of running one group of blocks.
#[derive(Debug, Default)]
pub struct Report {
    pub checked: usize,
    pub failures: Vec<String>,
}

impl Runner {
    /// Runs `blocks` (one page, or one module's doc comments) in order, in
    /// a fresh scratch directory, so `file=` blocks are visible to the
    /// examples after them and nothing else.
    pub fn run(&self, blocks: &[Block], scratch: &Path) -> Report {
        let mut report = Report::default();
        let _ = std::fs::remove_dir_all(scratch);
        if let Err(error) = std::fs::create_dir_all(scratch) {
            report.failures.push(format!("{}: {error}", scratch.display()));
            return report;
        }

        let mut index = 0;
        while index < blocks.len() {
            let block = &blocks[index];
            index += 1;

            if block.language() != "basm" {
                if EXPECTATIONS.contains(&block.language()) {
                    report.failures.push(format!(
                        "{}: a `{}` block must directly follow a compiled `basm` block",
                        block.location,
                        block.language()
                    ));
                }
                continue;
            }

            if let Some(file) = block.file() {
                let path = scratch.join(file);
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(&path, &block.body));
                if let Err(error) = written {
                    report.failures.push(format!("{}: {error}", block.location));
                }
                continue;
            }

            if block.has("ignore") {
                continue;
            }

            let start = index;
            while index < blocks.len() && EXPECTATIONS.contains(&blocks[index].language()) {
                index += 1;
            }

            report.checked += 1;
            if let Err(message) = self.check(scratch, block, &blocks[start..index], report.checked) {
                report.failures.push(format!("{}: {message}", block.location));
            }
        }

        let _ = std::fs::remove_dir_all(scratch);
        report
    }

    fn check(&self, dir: &Path, block: &Block, expectations: &[Block], n: usize) -> Result<(), String> {
        let expected = |language: &str| expectations.iter().find(|b| b.language() == language);

        let source = dir.join(format!("example{n}.basm"));
        let em = source.with_extension("em");
        std::fs::write(&source, &block.body).map_err(|error| error.to_string())?;

        let output = Command::new(&self.bitterasm)
            .current_dir(&self.cwd)
            .args(["compile", &source.display().to_string(), "-o", &em.display().to_string()])
            // Every lint but `generated_declarations`, which only reports
            // that a program generated declarations: what some examples are
            // there to show.
            .args(["-D", "all", "-A", "generated_declarations", "--color", "never"])
            .output()
            .map_err(|error| format!("couldn't run {}: {error}", self.bitterasm.display()))?;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if block.has("fail") {
            if output.status.success() {
                return Err("expected a compile error, but it compiled".into());
            }
            if let Some(expected) = expected("error") {
                let expected = expected.body.trim();
                if !stderr.contains(expected) {
                    return Err(format!("the error doesn't contain {expected:?}:\n{stderr}"));
                }
            }
            return Ok(());
        }

        if !output.status.success() {
            return Err(format!("failed to compile:\n{stderr}"));
        }

        if let Some(expected) = expected("emits") {
            let json = std::fs::read_to_string(&em).map_err(|e| format!("no .em file: {e}"))?;
            let file = EmFile::parse(&json, crate::emit::features::ALL).map_err(|e| format!("invalid .em: {e:?}"))?;
            let emitted: Vec<String> = file.entries.iter().map(|entry| render(&entry.value)).collect();
            let expected: Vec<String> = expected.body.lines().flat_map(expected_entries).collect();
            if emitted != expected {
                return Err(format!("expected it to emit {expected:?}, but it emitted {emitted:?}"));
            }
        }

        if let Some(expected) = expected("bytes") {
            let bitter = self
                .bitter
                .as_ref()
                .ok_or("checking `bytes` needs `bitter`, which wasn't found next to `bitterasm` or on PATH")?;
            let bin = source.with_extension("bin");
            let output = Command::new(bitter)
                .args(["encode", &em.display().to_string(), "-o", &bin.display().to_string()])
                .output()
                .map_err(|error| format!("couldn't run {}: {error}", bitter.display()))?;
            if !output.status.success() {
                return Err(format!("bitter encode failed:\n{}", String::from_utf8_lossy(&output.stderr)));
            }
            let bytes: Vec<String> = std::fs::read(&bin)
                .map_err(|error| error.to_string())?
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            let expected: Vec<String> = expected.body.split_whitespace().map(str::to_lowercase).collect();
            if bytes != expected {
                return Err(format!("expected the bytes {}, but got {}", expected.join(" "), bytes.join(" ")));
            }
        }

        Ok(())
    }
}

/// One line of an `emits` block: several integers, or one other value.
fn expected_entries(line: &str) -> Vec<String> {
    let line = line.trim();
    let ints: Vec<&str> = line.split(|c: char| c.is_whitespace() || c == ',').filter(|s| !s.is_empty()).collect();
    if ints.iter().all(|s| s.parse::<num_bigint::BigInt>().is_ok()) {
        ints.into_iter().map(str::to_owned).collect()
    } else {
        vec![line.to_owned()]
    }
}

/// An emitted value as an `emits` block spells it. Types are named without
/// their module path.
fn render(value: &EmittedValue) -> String {
    match value {
        EmittedValue::Int { value } => value.clone(),
        EmittedValue::Struct { id, args, fields } => {
            let fields: Vec<String> = fields.iter().map(|(name, value)| format!("{name}: {}", render(value))).collect();
            if fields.is_empty() {
                format!("{}{} {{}}", short(id), render_args(args))
            } else {
                format!("{}{} {{ {} }}", short(id), render_args(args), fields.join(", "))
            }
        }
        EmittedValue::Enum { id, variant, payload, .. } => match payload {
            Some(payload) => format!("{}.{variant}({})", short(id), render(payload)),
            None => format!("{}.{variant}", short(id)),
        },
        EmittedValue::Deferred { symbol, .. } => format!("<{symbol}>"),
    }
}

fn render_args(args: &[EmittedGenericArg]) -> String {
    if args.is_empty() {
        return String::new();
    }
    let args: Vec<String> = args
        .iter()
        .map(|arg| match arg {
            EmittedGenericArg::Const { value } => value.clone(),
            EmittedGenericArg::Type(EmittedType::Builtin { name }) => name.clone(),
            EmittedGenericArg::Type(EmittedType::Struct { id, args } | EmittedType::Enum { id, args }) => {
                format!("{}{}", short(id), render_args(args))
            }
        })
        .collect();
    format!("<{}>", args.join(", "))
}

fn short(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untagged_doc_comment_fences_are_basm() {
        assert_eq!(doc_comment_info(""), "basm");
        assert_eq!(doc_comment_info("ignore"), "basm,ignore");
        assert_eq!(doc_comment_info("fail"), "basm,fail");
        assert_eq!(doc_comment_info("file=x.basm"), "basm,file=x.basm");
        assert_eq!(doc_comment_info("basm,fail"), "basm,fail");
        assert_eq!(doc_comment_info("emits"), "emits");
        assert_eq!(doc_comment_info("sh"), "sh");
    }

    #[test]
    fn finds_fenced_blocks_with_their_lines() {
        let text = "prose\n```basm\nshow 1\n```\n\n  ```emits\n  1\n  ```\n";
        let blocks = fenced_blocks(text, |line| format!("page:{}", line + 1)).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].location, "page:2");
        assert_eq!(blocks[0].body, "show 1\n");
        assert_eq!(blocks[1].language(), "emits");
        assert_eq!(blocks[1].body, "1\n");
        assert!(fenced_blocks("```\nopen", |line| line.to_string()).is_err());
    }
}
