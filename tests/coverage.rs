// Instruction coverage for each architecture in `std`, measured against the
// reference lists in `tests/coverage/`. Each list names a dialect and gives
// one sample per operand form; a form is covered when its sample compiles
// through that dialect.
//
// The markers in the lists, the coverage tables in `README.md` and the
// book's `std/index.md`, and the `#! Coverage:` line in each architecture's
// `impl.basm` are all checked against the real result. After changing what
// an architecture supports, update them all with `make coverage`, which
// runs this test with `BITTERASM_BLESS=1`.

use std::path::{Path, PathBuf};
use std::process::Command;

const LISTS: &str = "tests/coverage";
const TABLE_START: &str = "<!-- coverage:start -->";
const TABLE_END: &str = "<!-- coverage:end -->";
const TABLE_PAGES: &[&str] = &["README.md", "docs/book/src/std/index.md"];

struct Form {
    covered: bool,
    form: String,
    sample: String,
}

struct Architecture {
    /// The list's file stem, which is also the `std` directory name.
    stem: String,
    name: String,
    reference: String,
    dialect: String,
    /// Everything above the forms, kept as written.
    header: String,
    forms: Vec<Form>,
}

impl Architecture {
    fn covered(&self) -> usize {
        self.forms.iter().filter(|form| form.covered).count()
    }

    fn percent(&self) -> usize {
        // Rounded down, so a list only reads 100% when it's complete.
        self.covered() * 100 / self.forms.len().max(1)
    }

    fn render(&self) -> String {
        let width = self.forms.iter().map(|form| form.form.len()).max().unwrap_or(0);
        let mut out = format!("{}\n\n", self.header.trim_end());
        for form in &self.forms {
            let marker = if form.covered { 'x' } else { ' ' };
            out.push_str(&format!("{marker} | {:width$} | {}\n", form.form, form.sample));
        }
        out
    }

    fn impl_line(&self) -> String {
        format!(
            "#! Coverage: {} of {} operand forms ({}%), through `{}`.",
            self.covered(),
            self.forms.len(),
            self.percent(),
            self.dialect
        )
    }
}

fn parse(path: &Path) -> Architecture {
    let text = std::fs::read_to_string(path).unwrap();
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .unwrap_or_else(|| panic!("{} has no `{key}:` line", path.display()))
            .trim()
            .to_string()
    };

    let mut header = Vec::new();
    let mut forms = Vec::new();
    for line in text.lines() {
        let columns: Vec<&str> = line.splitn(3, '|').collect();
        if let [marker, form, sample] = columns.as_slice()
            && !line.starts_with('#') {
                forms.push(Form {
                    covered: marker.trim() == "x",
                    form: form.trim().to_string(),
                    sample: sample.trim().to_string(),
                });
                continue;
            }
        if forms.is_empty() {
            header.push(line);
        }
    }

    Architecture {
        stem: path.file_stem().unwrap().to_string_lossy().into_owned(),
        name: field("name"),
        reference: field("reference"),
        dialect: field("dialect"),
        header: header.join("\n"),
        forms,
    }
}

/// Whether each sample compiles, checked in parallel.
fn compile_all(root: &Path, jobs: &[(String, String)]) -> Vec<bool> {
    let bitterasm = PathBuf::from(env!("CARGO_BIN_EXE_bitterasm"));
    let scratch = std::env::temp_dir().join(format!("bitterasm-coverage-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();

    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut results = vec![false; jobs.len()];
    std::thread::scope(|scope| {
        for (chunk_index, chunk) in results.chunks_mut(jobs.len().div_ceil(threads)).enumerate() {
            let (bitterasm, scratch) = (&bitterasm, &scratch);
            let start = chunk_index * jobs.len().div_ceil(threads);
            scope.spawn(move || {
                for (offset, result) in chunk.iter_mut().enumerate() {
                    let (dialect, sample) = &jobs[start + offset];
                    let file = scratch.join(format!("{}.basm", start + offset));
                    // `target` is there for samples that jump somewhere.
                    std::fs::write(&file, format!("from {dialect} import *\n\ntarget:\n    {sample}\n")).unwrap();
                    *result = Command::new(bitterasm)
                        .current_dir(root)
                        .arg("check")
                        .arg(&file)
                        .output()
                        .is_ok_and(|output| output.status.success());
                }
            });
        }
    });

    let _ = std::fs::remove_dir_all(&scratch);
    results
}

fn table(architectures: &[Architecture]) -> String {
    let mut out = String::from(
        "| Architecture | Reference set | Operand forms covered |\n|---|---|---|\n",
    );
    for arch in architectures {
        out.push_str(&format!(
            "| {} | {} | {} of {} ({}%) |\n",
            arch.name,
            arch.reference,
            arch.covered(),
            arch.forms.len(),
            arch.percent()
        ));
    }
    out
}

/// `text` with the coverage table between its markers replaced.
fn with_table(text: &str, table: &str) -> Option<String> {
    let start = text.find(TABLE_START)? + TABLE_START.len();
    let end = start + text[start..].find(TABLE_END)?;
    Some(format!("{}\n{table}{}", &text[..start], &text[end..]))
}

/// `text` with its `#! Coverage:` line replaced.
fn with_impl_line(text: &str, line: &str) -> Option<String> {
    let start = text.find("#! Coverage:")?;
    let end = start + text[start..].find('\n')?;
    Some(format!("{}{line}{}", &text[..start], &text[end..]))
}

#[test]
fn coverage_markers_and_tables_are_up_to_date() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root.join(LISTS))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    paths.sort();
    let mut architectures: Vec<Architecture> = paths.iter().map(|path| parse(path)).collect();
    assert!(!architectures.is_empty(), "no coverage lists in {LISTS}");

    let jobs: Vec<(String, String)> = architectures
        .iter()
        .flat_map(|arch| arch.forms.iter().map(|form| (arch.dialect.clone(), form.sample.clone())))
        .collect();
    let mut results = compile_all(root, &jobs).into_iter();

    let mut stale = Vec::new();
    for arch in &mut architectures {
        for form in &mut arch.forms {
            let covered = results.next().unwrap();
            if covered != form.covered {
                let now = if covered { "now compiles" } else { "no longer compiles" };
                stale.push(format!("{LISTS}/{}.txt: `{}` {now}", arch.stem, form.sample));
                form.covered = covered;
            }
        }
    }

    // Every file this test owns, as it should read.
    let mut expected: Vec<(PathBuf, String)> = Vec::new();
    for (arch, path) in architectures.iter().zip(&paths) {
        expected.push((path.clone(), arch.render()));
        let impl_path = root.join("std").join(&arch.stem).join("impl.basm");
        let text = std::fs::read_to_string(&impl_path).unwrap();
        let updated = with_impl_line(&text, &arch.impl_line())
            .unwrap_or_else(|| panic!("{} has no `#! Coverage:` line", impl_path.display()));
        expected.push((impl_path, updated));
    }
    let table = table(&architectures);
    for page in TABLE_PAGES {
        let path = root.join(page);
        let text = std::fs::read_to_string(&path).unwrap();
        let updated = with_table(&text, &table)
            .unwrap_or_else(|| panic!("{page} has no `{TABLE_START}` ... `{TABLE_END}` block"));
        expected.push((path, updated));
    }

    let outdated: Vec<&(PathBuf, String)> = expected
        .iter()
        .filter(|(path, text)| std::fs::read_to_string(path).unwrap() != *text)
        .collect();

    if std::env::var_os("BITTERASM_BLESS").is_some() {
        for (path, text) in outdated {
            std::fs::write(path, text).unwrap();
        }
        return;
    }

    assert!(
        outdated.is_empty(),
        "coverage is out of date; run `make coverage` to update it.\n{}\nFiles to update:\n{}",
        stale.join("\n"),
        outdated
            .iter()
            .map(|(path, _)| format!("  {}", path.strip_prefix(root).unwrap().display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
