//! Where each page goes: the module tree, as folders. `std.x86_64.nasm`
//! becomes `x86_64/nasm.md`, every folder gets an `index.md` listing what's
//! in it, and a module with a folder of the same name beneath it
//! (`x86_64.basm` next to `x86_64/`) becomes that folder's index. The
//! folders every module shares (`std/`) are left off, so the output
//! directory is the top of the tree.

use std::collections::{BTreeSet, HashMap};

/// Page paths are relative to the output directory, `/`-separated.
pub(super) struct Layout {
    /// The folders every module is under, left off every path.
    prefix: Vec<String>,
    /// Each module's path below `prefix`, by module name.
    modules: HashMap<String, Vec<String>>,
}

/// One entry in a folder: a module, a folder, or a module that is also a
/// folder.
pub(super) struct Child {
    pub name: String,
    /// The module, if there is one by this name.
    pub module: Option<String>,
    pub is_folder: bool,
    pub page: String,
}

impl Layout {
    pub fn new<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let all: Vec<(String, Vec<String>)> = names
            .into_iter()
            .map(|name| (name.to_string(), segments(name)))
            .collect();

        // The longest folder path every module is in, never a module itself.
        let mut prefix: Vec<String> = all.first().map(|(_, segments)| segments.clone()).unwrap_or_default();
        for (_, segments) in &all {
            let folder = &segments[..segments.len() - 1];
            let shared = prefix.iter().zip(folder).take_while(|(a, b)| a == b).count();
            prefix.truncate(shared);
        }
        let modules = all
            .into_iter()
            .map(|(name, segments)| (name, segments[prefix.len()..].to_vec()))
            .collect();
        Layout { prefix, modules }
    }

    /// `module`'s path below the shared prefix.
    pub fn path_of(&self, module: &str) -> Option<&[String]> {
        self.modules.get(module).map(Vec::as_slice)
    }

    /// Where `module`'s page goes.
    pub fn module_page(&self, module: &str) -> Option<String> {
        self.modules.get(module).map(|path| self.page_at(path))
    }

    /// Every folder that isn't also a module, root first: each needs an
    /// index page of its own.
    pub fn folders(&self) -> Vec<Vec<String>> {
        let mut folders = BTreeSet::new();
        folders.insert(Vec::new());
        for path in self.modules.values() {
            for end in 1..path.len() {
                folders.insert(path[..end].to_vec());
            }
        }
        folders.into_iter().filter(|folder| self.module_at(folder).is_none()).collect()
    }

    /// The module whose path is `path`, if any.
    pub fn module_at(&self, path: &[String]) -> Option<&str> {
        self.modules.iter().find(|(_, own)| own.as_slice() == path).map(|(name, _)| name.as_str())
    }

    pub fn is_folder(&self, path: &[String]) -> bool {
        path.is_empty() || self.modules.values().any(|own| own.len() > path.len() && own.starts_with(path))
    }

    /// The page at `path`: a folder's index, or a module's own page.
    pub fn page_at(&self, path: &[String]) -> String {
        if self.is_folder(path) {
            path.iter().map(|segment| format!("{segment}/")).collect::<String>() + "index.md"
        } else {
            format!("{}.md", path.join("/"))
        }
    }

    /// What's directly in the folder at `path`, by name.
    pub fn children(&self, path: &[String]) -> Vec<Child> {
        let names: BTreeSet<&String> = self
            .modules
            .values()
            .filter(|own| own.len() > path.len() && own.starts_with(path))
            .map(|own| &own[path.len()])
            .collect();
        names
            .into_iter()
            .map(|name| {
                let child: Vec<String> = path.iter().chain(std::iter::once(name)).cloned().collect();
                Child {
                    name: name.clone(),
                    module: self.module_at(&child).map(str::to_string),
                    is_folder: self.is_folder(&child),
                    page: self.page_at(&child),
                }
            })
            .collect()
    }

    /// A folder's title: its own name, or the shared prefix for the root.
    pub fn folder_title(&self, path: &[String]) -> String {
        path.last().or(self.prefix.last()).cloned().unwrap_or_else(|| "Reference".to_string())
    }

    /// The path to `path`'s module or folder, from the top: `std › x86_64`.
    /// Every part but the last links to its folder, relative to `from`. It's
    /// empty at the top, where it would only repeat the title.
    pub fn breadcrumb(&self, path: &[String], from: &str) -> String {
        let mut parts = Vec::new();
        let full: Vec<&String> = self.prefix.iter().chain(path).collect();
        if full.len() <= 1 {
            return String::new();
        }
        for (index, segment) in full.iter().enumerate() {
            let code = format!("`{segment}`");
            // Folders in the prefix aren't pages; the root index stands for
            // all of them.
            let below_prefix = index + 1 >= self.prefix.len();
            if index + 1 == full.len() || !below_prefix {
                parts.push(code);
            } else {
                let folder = &path[..index + 1 - self.prefix.len()];
                parts.push(format!("[{code}]({})", relative(from, &self.page_at(folder))));
            }
        }
        parts.join(" › ")
    }

    /// The nested Markdown list `SUMMARY.md` needs, each link prefixed with
    /// `base` (the output directory, relative to `SUMMARY.md`) and indented
    /// by `indent`.
    pub fn summary(&self, base: &str, indent: &str) -> String {
        let mut out = String::new();
        self.summary_of(&[], base, indent, &mut out);
        out
    }

    fn summary_of(&self, path: &[String], base: &str, indent: &str, out: &mut String) {
        for child in self.children(path) {
            out.push_str(&format!("{indent}- [`{}`]({base}{})\n", child.name, child.page));
            if child.is_folder {
                let inner: Vec<String> = path.iter().chain(std::iter::once(&child.name)).cloned().collect();
                self.summary_of(&inner, base, &format!("{indent}    "), out);
            }
        }
    }
}

// `std.x86_64.nasm` -> `std`, `x86_64`, `nasm`. A module outside every
// search root is named relative to the current directory (`..lib.x`); the
// leading dots aren't a folder.
fn segments(module: &str) -> Vec<String> {
    module.trim_start_matches('.').split('.').map(str::to_string).collect()
}

/// A link from the page at `from` to the page at `to`, both relative to the
/// output directory.
pub(super) fn relative(from: &str, to: &str) -> String {
    let from_dir: Vec<&str> = from.split('/').collect::<Vec<_>>().split_last().map(|(_, dir)| dir.to_vec()).unwrap_or_default();
    let to_parts: Vec<&str> = to.split('/').collect();
    let shared = from_dir.iter().zip(&to_parts).take_while(|(a, b)| a == b).count();
    let ups = "../".repeat(from_dir.len() - shared);
    format!("{ups}{}", to_parts[shared..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        Layout::new(["std.array", "std.x86_64.impl", "std.x86_64.nasm", "std.bitter.link"])
    }

    #[test]
    fn drops_the_shared_folder_and_nests_the_rest() {
        let layout = layout();
        assert_eq!(layout.module_page("std.array").as_deref(), Some("array.md"));
        assert_eq!(layout.module_page("std.x86_64.nasm").as_deref(), Some("x86_64/nasm.md"));
        assert_eq!(layout.folders(), vec![vec![], vec!["bitter".to_string()], vec!["x86_64".to_string()]]);
        assert_eq!(layout.folder_title(&[]), "std");
    }

    #[test]
    fn a_module_named_like_a_folder_is_its_index() {
        let layout = Layout::new(["lib.x86_64", "lib.x86_64.nasm", "lib.util"]);
        assert_eq!(layout.module_page("lib.x86_64").as_deref(), Some("x86_64/index.md"));
        assert_eq!(layout.folders(), vec![Vec::<String>::new()]);
    }

    #[test]
    fn links_and_breadcrumbs_are_relative() {
        assert_eq!(relative("x86_64/nasm.md", "x86_64/impl.md"), "impl.md");
        assert_eq!(relative("x86_64/nasm.md", "bitter/link.md"), "../bitter/link.md");
        assert_eq!(relative("array.md", "x86_64/index.md"), "x86_64/index.md");
        assert_eq!(
            layout().breadcrumb(&["x86_64".to_string(), "nasm".to_string()], "x86_64/nasm.md"),
            "[`std`](../index.md) › [`x86_64`](index.md) › `nasm`"
        );
    }

    #[test]
    fn summary_nests_folders() {
        assert_eq!(
            layout().summary("ref/", ""),
            concat!(
                "- [`array`](ref/array.md)\n",
                "- [`bitter`](ref/bitter/index.md)\n",
                "    - [`link`](ref/bitter/link.md)\n",
                "- [`x86_64`](ref/x86_64/index.md)\n",
                "    - [`impl`](ref/x86_64/impl.md)\n",
                "    - [`nasm`](ref/x86_64/nasm.md)\n",
            )
        );
    }
}
