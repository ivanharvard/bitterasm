// Compiles every BitterASM example in the book (`docs/book/src`), so the docs
// can't drift from the language. The conventions are described in
// `docs/book/src/contributing/writing-docs.md`:
//
// - ```` ```basm ```` must compile with every lint denied, except
//   `generated_declarations`.
// - ```` ```basm,fail ```` must fail to compile.
// - ```` ```basm,ignore ```` is a fragment, highlighted but not compiled.
// - ```` ```basm,file=path.basm ```` isn't compiled itself: it's written to
//   `path.basm` next to the page's later examples, so they can import it.
//
// Directly after a compiled `basm` block, any of:
//
// - ```` ```emits ```` lists what it must emit, in order: integers separated
//   by spaces, or one struct or enum per line, written
//   `Point { x: 1, y: 2 }`, `bits<8> { value: 65 }` or `Shape.Circle(2)`.
// - ```` ```bytes ```` lists, in hex, the bytes `bitter encode` packs its
//   output into.
// - ```` ```error ```` (after `basm,fail`) holds text the compiler's error
//   must contain.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

use bitterasm::emit::{EmFile, EmittedGenericArg, EmittedType, EmittedValue};

const BOOK: &str = "docs/book/src";

// Every lint but `generated_declarations`, which only reports that a program
// generated declarations: what some examples are there to show. (`-D all -A
// ...` can't express this: the CLI applies every `-A` before any `-D`.)
const DENIED_LINTS: &[&str] = &["unused", "unreachable_code", "fold_without_next", "unfulfilled_lint_expectation"];

const EXPECTATIONS: &[&str] = &["emits", "bytes", "error"];

struct Block {
    location: String,
    info: String,
    body: String,
}

impl Block {
    fn language(&self) -> &str {
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

#[test]
fn book_examples_compile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pages = Vec::new();
    markdown_files(&root.join(BOOK), &mut pages);
    pages.sort();
    assert!(!pages.is_empty(), "no pages found under {BOOK}");

    let mut failures = Vec::new();
    let mut checked = 0;

    for (page_index, page) in pages.iter().enumerate() {
        let dir = std::env::temp_dir().join(format!("bitterasm-book-{}-{page_index}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp directory should be creatable");

        let blocks = fenced_blocks(root, page);
        let mut index = 0;
        while index < blocks.len() {
            let block = &blocks[index];
            index += 1;

            if block.language() != "basm" {
                if EXPECTATIONS.contains(&block.language()) {
                    failures.push(format!(
                        "{}: a `{}` block must directly follow a compiled `basm` block",
                        block.location,
                        block.language()
                    ));
                }
                continue;
            }

            if let Some(file) = block.file() {
                let path = dir.join(file);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, &block.body).unwrap();
                continue;
            }

            if block.has("ignore") {
                continue;
            }

            let start = index;
            while index < blocks.len() && EXPECTATIONS.contains(&blocks[index].language()) {
                index += 1;
            }

            checked += 1;
            if let Err(message) = check(root, &dir, block, &blocks[start..index], checked) {
                failures.push(format!("{}: {message}", block.location));
            }
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    assert!(checked > 0, "no ```basm examples found under {BOOK}");
    assert!(
        failures.is_empty(),
        "{} of {checked} book example(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn check(root: &Path, dir: &Path, block: &Block, expectations: &[Block], n: usize) -> Result<(), String> {
    let expected = |language: &str| expectations.iter().find(|b| b.language() == language);

    let source = dir.join(format!("example{n}.basm"));
    let em = source.with_extension("em");
    std::fs::write(&source, &block.body).expect("temp example should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_bitterasm"))
        .current_dir(root)
        .args(["compile", &source.display().to_string(), "-o", &em.display().to_string()])
        .args(DENIED_LINTS.iter().flat_map(|lint| ["-D", lint]))
        .args(["--color", "never"])
        .output()
        .expect("bitterasm compile should run");
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
        let file = EmFile::parse(&json, bitterasm::emit::features::ALL).map_err(|e| format!("invalid .em: {e:?}"))?;
        let emitted: Vec<String> = file.entries.iter().map(|entry| render(&entry.value)).collect();
        let expected: Vec<String> = expected.body.lines().flat_map(expected_entries).collect();
        if emitted != expected {
            return Err(format!("expected it to emit {expected:?}, but it emitted {emitted:?}"));
        }
    }

    if let Some(expected) = expected("bytes") {
        let bin = source.with_extension("bin");
        let output = Command::new(bitter_bin())
            .args(["encode", &em.display().to_string(), "-o", &bin.display().to_string()])
            .output()
            .expect("bitter encode should run");
        if !output.status.success() {
            return Err(format!("bitter encode failed:\n{}", String::from_utf8_lossy(&output.stderr)));
        }
        let bytes: Vec<String> = std::fs::read(&bin).unwrap().iter().map(|b| format!("{b:02x}")).collect();
        let expected: Vec<String> = expected.body.split_whitespace().map(str::to_lowercase).collect();
        if bytes != expected {
            return Err(format!("expected the bytes {}, but got {}", expected.join(" "), bytes.join(" ")));
        }
    }

    Ok(())
}

// `CARGO_BIN_EXE_bitter` isn't set for a test in this package, so build
// `bitter` on demand next to `bitterasm`, as `tests/wasm_module.rs` does.
fn bitter_bin() -> PathBuf {
    static BUILD_ONCE: Once = Once::new();

    let bitterasm = PathBuf::from(env!("CARGO_BIN_EXE_bitterasm"));
    let exe_name = if cfg!(windows) { "bitter.exe" } else { "bitter" };
    let bitter = bitterasm.parent().unwrap().join(exe_name);

    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args(["build", "--package", "bitter", "--quiet"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("cargo build --package bitter should run");
        assert!(status.success(), "failed to build the `bitter` binary");
    });

    bitter
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

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("book directory should be readable") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

/// Every fenced code block in `page`, in order. Fences may be indented (inside
/// a list item); the body keeps its lines relative to that indentation.
fn fenced_blocks(root: &Path, page: &Path) -> Vec<Block> {
    let text = std::fs::read_to_string(page).expect("book page should be readable");
    let display = page.strip_prefix(root).unwrap_or(page).display().to_string();

    let mut blocks = Vec::new();
    let mut open: Option<(usize, usize, String, String)> = None;

    for (number, line) in text.lines().enumerate() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();

        match &mut open {
            None => {
                if let Some(info) = trimmed.strip_prefix("```") {
                    open = Some((number + 1, indent, info.trim().to_owned(), String::new()));
                }
            }
            Some((start, fence_indent, info, body)) => {
                if trimmed.starts_with("```") {
                    blocks.push(Block {
                        location: format!("{display}:{start}"),
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

    assert!(open.is_none(), "{display}: unclosed code fence");
    blocks
}
