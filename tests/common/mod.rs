// Shared helpers for tests that compile a fixture through the real
// `bitterasm` CLI and inspect what it emitted. Kept in one place so a change
// to the `.em` format only has to be followed here.

#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, Output};

use bitterasm::emit::{EmittedEntry, EmittedValue};

/// Runs `bitterasm compile` on `fixture` (relative to the crate root) and
/// returns the process output plus the `.em` path it was told to write.
pub fn compile(fixture: &str) -> (Output, std::path::PathBuf) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let source = Path::new(manifest_dir).join(fixture);
    let unique = fixture.replace(['/', '.'], "_");
    let out = std::env::temp_dir().join(format!("bitterasm-test-{}-{unique}.em", std::process::id()));

    let output = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &out.display().to_string()])
        .output()
        .expect("bitterasm compile should run");

    (output, out)
}

/// Compiles `fixture`, which must succeed, and returns its `.em` entries.
pub fn compile_entries(fixture: &str) -> Vec<EmittedEntry> {
    let (output, out) = compile(fixture);
    assert!(
        output.status.success(),
        "bitterasm compile failed for {fixture}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = std::fs::read_to_string(&out).expect(".em file should exist");
    let _ = std::fs::remove_file(&out);
    serde_json::from_str(&json).expect(".em file should be valid EmittedEntry JSON")
}

/// Compiles `fixture` and returns every emitted value as a plain integer,
/// in order. Every entry must be a bare `Int`.
pub fn compile_ints(fixture: &str) -> Vec<i128> {
    compile_entries(fixture)
        .into_iter()
        .map(|entry| match entry.value {
            EmittedValue::Int { value } => value.parse().expect("an Int entry holds a decimal integer"),
            other => panic!("expected only bare Int entries in {fixture}, found {other:?}"),
        })
        .collect()
}

/// Compiles `fixture`, which must fail, and returns its stderr.
pub fn compile_error(fixture: &str) -> String {
    let (output, _) = compile(fixture);
    assert!(!output.status.success(), "bitterasm compile unexpectedly succeeded for {fixture}");
    String::from_utf8_lossy(&output.stderr).into_owned()
}
