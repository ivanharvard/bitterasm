// Broad-but-shallow "does this std fixture still compile" checks, through
// the real CLI (`CARGO_BIN_EXE_bitterasm`) — these fixtures existed before
// this test file did, but nothing ever actually ran them, so they'd been
// silently orphaned. Both surfaced real cross-module non-`pub` field
// accesses once struct-field `pub` started being enforced against the
// module that declared it (`std.bitter.deferred`'s `BinOp`, `std.decimal`'s
// `Decimal`/`Fraction` — fixed alongside this test, not worked around by
// it): `bitterasm check` failing here again would mean that enforcement
// regressed against std's own use of `pub`, not that these fixtures are
// wrong.

use std::path::Path;
use std::process::Command;

fn assert_checks_cleanly(relative_path: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let bitterasm = env!("CARGO_BIN_EXE_bitterasm");
    let source = Path::new(manifest_dir).join(relative_path);

    let output = Command::new(bitterasm)
        .current_dir(manifest_dir)
        .args(["check", &source.display().to_string()])
        .output()
        .expect("bitterasm check should run");

    assert!(
        output.status.success(),
        "bitterasm check failed for {relative_path}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn bitfield_all_ops_fixture_checks_cleanly() {
    assert_checks_cleanly("tests/fixtures/bitfield/all_ops.basm");
}

#[test]
fn deferred_dispatch_fixture_checks_cleanly() {
    assert_checks_cleanly("tests/fixtures/bitter/deferred_dispatch.basm");
}

#[test]
fn math_all_ops_fixture_checks_cleanly() {
    assert_checks_cleanly("tests/fixtures/math/all_ops.basm");
}

#[test]
fn pdp10_all_instructions_fixture_checks_cleanly() {
    assert_checks_cleanly("tests/fixtures/pdp10/all_instructions.basm");
}

#[test]
fn pdp10_registers_fixture_checks_cleanly() {
    assert_checks_cleanly("tests/fixtures/pdp10/registers.basm");
}
