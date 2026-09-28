//! `bitterasm doc`: reference pages generated from `##` and `#!` doc
//! comments, and the examples in them run as tests. std's reference is
//! built by the same command anyone documenting their own architecture
//! runs; nothing about it is special to std.
//!
//! Almost everything on a page comes from the code itself: which macros a
//! module offers (following `pub from` re-exports), how each is written in
//! that module (following `syntax` facets and lines), its parameters, and
//! what it returns or emits. Doc comments add only the prose.

pub mod doctest;
mod layout;
mod model;
mod render;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::loader::LoadError;
use layout::Layout;
use model::Modules;

/// One generated Markdown file.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Relative to the output directory, `/`-separated: `x86_64/nasm.md`.
    pub path: String,
    /// The module it documents, or `None` for a folder's index.
    pub module: Option<String>,
    pub markdown: String,
}

/// Every page, and the module tree they follow.
pub struct Reference {
    pub pages: Vec<Page>,
    layout: Layout,
}

impl Reference {
    /// The pages as a nested Markdown list, for mdBook's `SUMMARY.md`: each
    /// link starts with `base` (the output directory as `SUMMARY.md` sees
    /// it) and each line with `indent`.
    pub fn summary(&self, base: &str, indent: &str) -> String {
        self.layout.summary(base, indent)
    }
}

/// The lines between these two, in a `SUMMARY.md`, are `bitterasm doc`'s.
pub const SUMMARY_BEGIN: &str = "<!-- bitterasm doc: begin -->";
pub const SUMMARY_END: &str = "<!-- bitterasm doc: end -->";

/// `summary` with the lines between its `SUMMARY_BEGIN` and `SUMMARY_END`
/// markers replaced by `reference`'s pages. The list is indented like the
/// begin marker, so the markers go under the entry the pages belong to.
pub fn update_summary(summary: &str, reference: &Reference, base: &str) -> Result<String, String> {
    let lines: Vec<&str> = summary.lines().collect();
    let begin = lines.iter().position(|line| line.trim() == SUMMARY_BEGIN);
    let end = lines.iter().position(|line| line.trim() == SUMMARY_END);
    let (Some(begin), Some(end)) = (begin, end) else {
        return Err(format!("no `{SUMMARY_BEGIN}` ... `{SUMMARY_END}` lines to put the pages between"));
    };
    if end < begin {
        return Err(format!("`{SUMMARY_END}` comes before `{SUMMARY_BEGIN}`"));
    }
    let indent = &lines[begin][..lines[begin].len() - lines[begin].trim_start().len()];
    let mut out: Vec<String> = lines[..=begin].iter().map(|line| line.to_string()).collect();
    out.extend(reference.summary(base, indent).lines().map(str::to_string));
    out.extend(lines[end..].iter().map(|line| line.to_string()));
    Ok(out.join("\n") + "\n")
}

/// Every `.basm` file named by `paths`, searching directories recursively,
/// in a stable order.
pub fn collect_files(paths: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                walk(&entry?.path(), out)?;
            }
        } else if path.extension().is_some_and(|extension| extension == "basm") {
            out.push(path.to_path_buf());
        }
        Ok(())
    }

    let mut files = Vec::new();
    for path in paths {
        if !path.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} doesn't exist", path.display())));
        }
        walk(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// A page for each of `files`, placed by module path, and an `index.md` for
/// each folder. Links to modules outside `files` are left as plain names.
pub fn generate(files: &[PathBuf]) -> Result<Reference, LoadError> {
    let mut modules = Modules::default();
    let loaded = files.iter().map(|file| modules.load(file)).collect::<Result<Vec<_>, _>>()?;
    let layout = Layout::new(loaded.iter().map(|module| module.name.as_str()));
    let summaries: HashMap<String, String> = loaded
        .iter()
        .map(|module| (module.name.clone(), render::summary(module.doc.as_ref())))
        .collect();

    let mut pages: Vec<Page> = loaded
        .iter()
        .map(|module| {
            let path = layout.module_page(&module.name).expect("every loaded module is laid out");
            let links = render::Links { layout: &layout, from: path.clone(), summaries: &summaries };
            Page { markdown: render::page(module, &links), module: Some(module.name.clone()), path }
        })
        .collect();
    for folder in layout.folders() {
        let path = layout.page_at(&folder);
        let links = render::Links { layout: &layout, from: path.clone(), summaries: &summaries };
        pages.push(Page { markdown: render::folder(&folder, &links), module: None, path });
    }
    pages.sort_by(|a, b| a.path.cmp(&b.path));
    pages.dedup_by(|a, b| a.path == b.path);
    Ok(Reference { pages, layout })
}

/// The examples in one file's doc comments, as fenced blocks located at
/// the source lines they come from.
#[derive(Debug)]
pub struct Examples {
    pub module: String,
    pub blocks: Vec<doctest::Block>,
    /// Docs whose examples couldn't be read, e.g. an unclosed fence.
    pub errors: Vec<String>,
}

pub fn examples(files: &[PathBuf]) -> Result<Vec<Examples>, LoadError> {
    let mut modules = Modules::default();
    let mut out = Vec::new();
    for file in files {
        let module = modules.load(file)?;
        let mut blocks = Vec::new();
        let mut errors = Vec::new();
        for doc in &module.docs {
            let first_line = module.source[..doc.span.start.min(module.source.len())].matches('\n').count();
            let location = |line: usize| format!("{}:{}", file.display(), first_line + line + 1);
            match doctest::fenced_blocks(&doc.text, location) {
                Ok(found) => blocks.extend(found.into_iter().map(|mut block| {
                    block.info = doctest::doc_comment_info(&block.info);
                    block
                })),
                Err(message) => errors.push(message),
            }
        }
        out.push(Examples { module: module.name.clone(), blocks, errors });
    }
    Ok(out)
}
