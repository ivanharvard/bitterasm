// Formatting is stable: running `bitterasm format` on its own output changes
// nothing. Checked over every `.basm` file in the repository, since those
// cover far more layouts than hand-written cases do.

use std::path::{Path, PathBuf};

use bitterasm::formatter::{format_source, FormatConfig};

fn basm_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            basm_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "basm") {
            out.push(path);
        }
    }
}

#[test]
fn formatting_is_idempotent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["std", "examples", "tests/fixtures"] {
        basm_files(&root.join(dir), &mut files);
    }

    // The repository's own width, and the default.
    let mut unstable = Vec::new();
    for max_width in [80, 100] {
        let config = FormatConfig { max_width, ..Default::default() };
        for path in &files {
            let source = std::fs::read_to_string(path).unwrap();
            let Ok(once) = format_source(&source, &config) else { continue };
            let twice = format_source(&once, &config).unwrap();
            if once != twice {
                unstable.push(format!("{} (max_width {max_width})", path.strip_prefix(root).unwrap().display()));
            }
        }
    }

    assert!(!files.is_empty());
    assert!(unstable.is_empty(), "formatting these twice changes them again: {unstable:#?}");
}
