#!/usr/bin/env python3
"""Cross-checks a full `std/wasm` module — through the real bitterasm ->
bitter pipeline — against a single independent oracle: WABT (the
WebAssembly Binary Toolkit), run inside a Docker container
(tests/wasm/docker/), an existing, independently-maintained reference
implementation of the WASM spec this project didn't write and shares no
code with.

Unlike tests/riscv/run_tests.py's oracle (GNU binutils), this doesn't
compare raw bytes: WASM's own binary format spec permits more than one
valid encoding of the same module (`std/wasm/module.basm`'s
`deferred_uleb128` deliberately emits non-minimal, fixed-width LEB128
length prefixes — see that file's own doc comment for why), so two
correct encoders can legitimately disagree byte-for-byte while still
being equally valid WASM. Instead, for each tests/wasm/cases/*.basm file
that represents a *complete module* (magic/version and every section, not
a bare instruction snippet — see tests/wasm_encoding.rs for those), this:

1. Compiles it with `bitterasm compile`, then packs the resulting .em's
   emitted values into raw bytes with `bitter encode` — the actual
   encoder under test.
2. Feeds those bytes to the Docker oracle (see docker/validate_and_run.sh),
   which validates the module structurally with `wasm-validate` and then
   runs every zero-argument exported function with
   `wasm-interp --run-all-exports`.
3. Checks the oracle's output against this script's own expected line(s)
   for that case — proof that every length prefix `span()`/
   `deferred_uleb128` computed is correct (a wrong one fails
   `wasm-validate` outright) *and* that the instruction encoding actually
   computes the right thing (a wrong opcode or branch depth fails at
   execution, not just at parsing).

Requires Docker; this builds the oracle image itself (cached by Docker
after the first run) from tests/wasm/docker/.

Usage: python3 tests/wasm/run_tests.py
"""

from __future__ import annotations

import pathlib
import subprocess
import sys
import tempfile

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
CASES_DIR = pathlib.Path(__file__).resolve().parent / "cases"
DOCKER_DIR = pathlib.Path(__file__).resolve().parent / "docker"
BITTERASM_BIN = REPO_ROOT / "target" / "debug" / "bitterasm"
BITTER_BIN = REPO_ROOT / "target" / "debug" / "bitter"
ORACLE_IMAGE = "bitterasm-wasm-oracle"

# Each entry is a full-module case (basm filename, relative to CASES_DIR)
# paired with the exact stdout the oracle should produce for it — one line
# per zero-argument export `wasm-interp --run-all-exports` runs, in the
# order the module declares its exports.
MODULE_CASES: list[tuple[str, list[str]]] = [
    ("module.basm", ["sum_to_ten() => i32:45"]),
]


def build_bitterasm() -> None:
    subprocess.run(["cargo", "build", "--bin", "bitterasm"], cwd=REPO_ROOT, check=True)


def build_bitter() -> None:
    subprocess.run(["cargo", "build", "--package", "bitter"], cwd=REPO_ROOT, check=True)


def build_oracle_image() -> None:
    subprocess.run(["docker", "build", "-t", ORACLE_IMAGE, str(DOCKER_DIR)], check=True)


def compile_and_encode(basm_path: pathlib.Path, workdir: pathlib.Path) -> bytes:
    em_path = workdir / (basm_path.stem + ".em")
    bin_path = workdir / (basm_path.stem + ".wasm")

    # cwd=REPO_ROOT matters: an absolute `from ... import *` is resolved
    # against the current working directory, not against basm_path's own
    # (temp-directory) location.
    compile_result = subprocess.run(
        [str(BITTERASM_BIN), "compile", str(basm_path), "-o", str(em_path)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )
    if compile_result.returncode != 0:
        raise RuntimeError(f"bitterasm compile failed for {basm_path.name}:\n{compile_result.stderr}")

    encode_result = subprocess.run(
        [str(BITTER_BIN), "encode", str(em_path), "-o", str(bin_path)],
        capture_output=True,
        text=True,
    )
    if encode_result.returncode != 0:
        raise RuntimeError(f"bitter encode failed for {em_path.name}:\n{encode_result.stderr}")

    return bin_path.read_bytes()


def run_oracle(wasm_bytes: bytes) -> str:
    result = subprocess.run(
        ["docker", "run", "--rm", "-i", ORACLE_IMAGE],
        input=wasm_bytes,
        capture_output=True,
    )

    if result.returncode != 0:
        stderr = result.stderr.decode(errors="replace")
        raise RuntimeError(f"oracle rejected the module:\n{stderr}")

    return result.stdout.decode(errors="replace")


def run_case(name: str, expected_lines: list[str], workdir: pathlib.Path) -> list[str]:
    basm_path = CASES_DIR / name
    if not basm_path.exists():
        return [f"{name}: no such case file"]

    wasm_bytes = compile_and_encode(basm_path, workdir)
    actual_lines = [line for line in run_oracle(wasm_bytes).splitlines() if line]

    if actual_lines != expected_lines:
        return [f"{name}: expected {expected_lines!r}, oracle produced {actual_lines!r}"]

    return []


def main() -> int:
    build_bitterasm()
    build_bitter()
    build_oracle_image()

    all_failures: list[str] = []

    with tempfile.TemporaryDirectory() as tmp:
        workdir = pathlib.Path(tmp)

        for name, expected_lines in MODULE_CASES:
            try:
                failures = run_case(name, expected_lines, workdir)
            except Exception as error:  # noqa: BLE001 - report and keep going
                failures = [f"{name}: {error}"]

            all_failures.extend(failures)

            status = "ok" if not failures else f"{len(failures)} FAILED"
            print(f"{name}: {status}")

    print()

    if all_failures:
        print(f"{len(all_failures)} failure(s) out of {len(MODULE_CASES)} case(s):")
        for failure in all_failures:
            print(f"  {failure}")
        return 1

    print(f"all {len(MODULE_CASES)} case(s) validated and ran correctly")
    return 0


if __name__ == "__main__":
    sys.exit(main())
