// Compiles every BitterASM example in the book (`docs/book/src`), so the docs
// can't drift from the language. The rules, shared with `bitterasm doc
// --test`, live in `bitterasm::doc::doctest`; authors will find them in
// `docs/book/src/contributing/writing-docs.md`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

use bitterasm::doc::doctest::{fenced_blocks, Runner};

const BOOK: &str = "docs/book/src";

#[test]
fn book_examples_compile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut pages = Vec::new();
    markdown_files(&root.join(BOOK), &mut pages);
    pages.sort();
    assert!(!pages.is_empty(), "no pages found under {BOOK}");

    let runner = Runner {
        bitterasm: PathBuf::from(env!("CARGO_BIN_EXE_bitterasm")),
        bitter: Some(bitter_bin()),
        cwd: root.to_path_buf(),
    };

    let mut failures = Vec::new();
    let mut checked = 0;

    for (page_index, page) in pages.iter().enumerate() {
        let text = std::fs::read_to_string(page).expect("book page should be readable");
        let display = page.strip_prefix(root).unwrap_or(page).display().to_string();
        let blocks = match fenced_blocks(&text, |line| format!("{display}:{}", line + 1)) {
            Ok(blocks) => blocks,
            Err(message) => {
                failures.push(message);
                continue;
            }
        };

        let scratch = std::env::temp_dir().join(format!("bitterasm-book-{}-{page_index}", std::process::id()));
        let report = runner.run(&blocks, &scratch);
        checked += report.checked;
        failures.extend(report.failures);
    }

    assert!(checked > 0, "no ```basm examples found under {BOOK}");
    assert!(
        failures.is_empty(),
        "{} of {checked} book example(s) failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
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
