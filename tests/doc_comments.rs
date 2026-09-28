// `##`/`#!` doc comments on real std modules: they attach to what they
// document through the loader, and none of std's are stray (which the
// `unused_doc_comments` lint would report to anyone importing it).

use std::path::{Path, PathBuf};

use bitterasm::ast::{Doc, Statement};
use bitterasm::loader::load_entry_program;

fn text(doc: &Option<Doc>) -> Option<&str> {
    doc.as_ref().map(|doc| doc.text.as_str())
}

#[test]
fn nasm_dialect_docs_attach_to_its_declarations() {
    let program = load_entry_program(Path::new("std/x86_64/nasm.basm")).unwrap();

    let module = text(&program.doc).expect("nasm.basm has a `#!` block");
    assert!(module.starts_with("NASM-flavored x86-64 assembly"), "{module}");
    assert!(module.contains("```\nfrom std.x86_64.nasm import *\n\nlea rsi, [rel msg]\n"), "{module}");
    // Maintainer notes stay out of it.
    assert!(!module.contains("impl.basm"), "{module}");

    let macro_doc = |name: &str| {
        program.statements.iter().find_map(|statement| match statement {
            Statement::Macro(declaration)
                if bitterasm::ast::literal_name(&declaration.name).as_deref() == Some(name) =>
            {
                Some(text(&declaration.doc).map(str::to_string))
            }
            _ => None,
        })
    };
    let rel = macro_doc("rel").flatten().expect("`rel` is documented");
    assert!(rel.starts_with("`[rel label]`"), "{rel}");
    // `data_string`'s `##` line sits above a `#` note; only the `##` is doc.
    assert_eq!(
        macro_doc("data_string").flatten().as_deref(),
        Some("A string literal's UTF-8 bytes, with no terminator."),
    );
}

#[test]
fn every_public_std_item_is_documented() {
    let files = bitterasm::doc::collect_files(&[PathBuf::from("std")]).unwrap();
    let mut failures = Vec::new();
    for file in files {
        let output = bitterasm(&["check", file.to_str().unwrap(), "-D", "missing_docs", "--color", "never"]);
        if !output.status.success() {
            failures.push(String::from_utf8_lossy(&output.stderr).into_owned());
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn std_has_no_stray_doc_comments() {
    fn basm_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                basm_files(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "basm") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    basm_files(Path::new("std"), &mut files);
    assert!(!files.is_empty());
    for file in files {
        let program = load_entry_program(&file).unwrap();
        assert!(program.stray_docs.is_empty(), "{}: {:?}", file.display(), program.stray_docs);
    }
}

// ---- `bitterasm doc` ----

use std::process::Command;

const REFERENCE: &str = "docs/book/src/std/reference";
const SUMMARY: &str = "docs/book/src/SUMMARY.md";

fn bitterasm(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_bitterasm"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args)
        .output()
        .expect("bitterasm should run")
}

// The pages, and their list in the book's `SUMMARY.md`.
#[test]
fn std_reference_is_up_to_date() {
    let output = bitterasm(&["doc", "std", "-o", REFERENCE, "--summary", SUMMARY, "--check"]);
    assert!(
        output.status.success(),
        "{}\nregenerate it with `bitterasm doc std -o {REFERENCE} --summary {SUMMARY}`",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn std_doc_examples_pass() {
    // `bytes` checks need `bitter` next to `bitterasm`.
    let status = Command::new("cargo")
        .args(["build", "--package", "bitter", "--quiet"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap();
    assert!(status.success());

    let output = bitterasm(&["doc", "std", "--test"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(!stdout.starts_with("0 of 0"), "no doc examples ran: {stdout}");
}

// A page of the whole std's reference, so links to other modules resolve.
fn page(module_file: &str) -> String {
    let files = bitterasm::doc::collect_files(&[PathBuf::from("std")]).unwrap();
    let module = bitterasm::loader::module_path_of(Path::new(module_file));
    let reference = bitterasm::doc::generate(&files).unwrap();
    reference.pages.into_iter().find(|page| page.module.as_ref() == Some(&module)).unwrap().markdown
}

#[test]
fn pages_show_each_dialects_own_spelling() {
    // The same `impl` instruction, re-exported by two dialects.
    assert!(page("std/riscv/native.basm").contains("| `add rd, rs1, rs2` |"));
    assert!(page("std/riscv/c_like.basm").contains("| `rd = rs1 + rs2` |"));

    let nasm = page("std/x86_64/nasm.basm");
    // An operand pattern, as its author wrote it.
    assert!(nasm.contains("| `[rel target]` | `target: int` | returns `RipLabel` |"), "{nasm}");
    // Re-exported overloads keep their own module's name...
    assert!(nasm.contains("| `mov rd, rs` | `rd: Reg`, `rs: Reg` |  | `rd = rs`. | [`std.x86_64.intel`](intel.md) |"));
    // ...and re-exported types are linked, not repeated.
    assert!(nasm.contains("From [`std.x86_64.impl`](impl.md): `Reg`, `r0`"));
    assert!(!nasm.contains("## Types"));
    // Pages follow the folders: the title is the module's own name.
    assert!(nasm.starts_with("# `nasm`\n\n[`std`](../index.md) › [`x86_64`](index.md) › `nasm`\n"), "{nasm}");
    // Doc examples become compiled `basm` blocks.
    assert!(nasm.contains("```basm\nfrom std.x86_64.nasm import *\n\nlea rsi, [rel msg]"));
}
