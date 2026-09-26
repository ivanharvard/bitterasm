// Exercises `main::resolve_and_expand`'s driver-level short-circuit
// (`AliasResolver::used_forward_label_placeholder`): a program is only
// walked twice (position-discovery, then the real pass) when some label is
// actually referenced before its own position is known. Real `bitterasm
// compile`, through the actual CLI binary (`CARGO_BIN_EXE_bitterasm`, not a
// hand-rolled call into the library) — same standard `tests/riscv_dialects.rs`
// holds itself to.

use std::path::Path;
use std::process::Command;

use bitterasm::emit::EmittedValue;

fn compile_to_emitted(fixture: &str) -> Vec<EmittedValue> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");

    let source = Path::new(manifest_dir).join(fixture);
    let stem = Path::new(fixture).file_stem().unwrap().to_string_lossy();
    let out = std::env::temp_dir().join(format!("bitterasm-label-passes-{stem}.em"));

    let status = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["compile", &source.display().to_string(), "-o", &out.display().to_string()])
        .status()
        .expect("bitterasm compile should run");
    assert!(status.success(), "bitterasm compile failed for {fixture}");

    let json = std::fs::read_to_string(&out).expect(".em file should exist");
    bitterasm::emit::EmFile::parse(&json, bitterasm::emit::features::ALL)
        .expect("a valid .em file")
        .entries
        .into_iter()
        .map(|entry| entry.value)
        .collect()
}

fn int(value: &str) -> EmittedValue {
    EmittedValue::Int { value: value.to_string() }
}

#[test]
fn a_forward_label_reference_still_resolves_correctly_through_the_fallback_pass() {
    // `skip_target` is referenced (line 12) before its own declaration
    // (line 14) — a genuine forward reference, forcing the fallback
    // two-pass path. Same fixture and expected sequence as
    // `resolver::macro_body::tests::forward_and_backward_label_references_resolve_via_two_pass_discovery`:
    // noop -> 1; reads_target loop_start -> 0 (backward, loop_start's own
    // position); reads_target skip_target -> 4 (forward — the case a
    // single, never-revisited pass would get wrong); noop -> 1.
    let emitted = compile_to_emitted("tests/fixtures/emit/labels_forward_and_backward.basm");
    assert_eq!(emitted, vec![int("1"), int("0"), int("4"), int("1")]);
}

#[test]
fn a_backward_only_label_reference_resolves_correctly_through_the_fast_single_pass() {
    // `loop_start` is only ever referenced *after* its own declaration, so
    // `used_forward_label_placeholder` never goes true and
    // `main::resolve_and_expand` should take the new single-pass path —
    // this asserts that path still resolves the label to its real
    // position (0), not a leftover placeholder.
    let emitted = compile_to_emitted("tests/fixtures/emit/backward_label_only.basm");
    assert_eq!(emitted, vec![int("1"), int("0"), int("1")]);
}

#[test]
fn a_pub_label_resolves_identically_to_a_non_pub_one() {
    // Phase 4 (see `docs/sections-and-linking/PROGRESS.md`): `pub` on a
    // label parses and is recorded, but changes nothing about how it
    // resolves within a single file — same expected sequence as the
    // otherwise-identical `backward_label_only.basm`.
    let emitted = compile_to_emitted("tests/fixtures/emit/pub_label.basm");
    assert_eq!(emitted, vec![int("1"), int("0"), int("1")]);
}
