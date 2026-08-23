#!/usr/bin/env python3
"""Cross-checks std/riscv/native.basm against two independently-sourced
oracles: a hand-written Python reference encoder, and an existing,
external RV32I decoder crate.

For each tests/riscv/cases/*.s file (plain RISC-V assembly text — no
labels or directives, one instruction per line, '#' comments) this:

1. Builds a derived .basm by prepending `from std.riscv.native import *`
   to the file's own text, unchanged otherwise — the .s file itself stays
   valid, ordinary-looking assembly.
2. Compiles that with `bitterasm compile` and packs the resulting .em's
   emitted values with packer.py — a generic bits<N>-field concatenator
   standing in for bitter's own (still unbuilt) encoder.
3. Independently encodes the *original* .s file's text with reference.py,
   an RV32I encoder written directly from the ISA spec, sharing no code
   with std/riscv/native.basm — and compares word-for-word.
4. Separately decodes bitterasm's own words with decoder.py (a thin
   wrapper over the `riscv-decode` crate — existing, external, not written
   for this project) and checks the decoded fields match what the source
   line actually asked for.

An early version of this suite tried a *third*-party crate,
`riscv_assembler`, as the reference encoder instead of reference.py. It
turned out to have real bugs — non-standard LUI semantics and an
absolute-vs-relative mixup for branch/jump immediates — caught by
cross-checking it against riscv-decode. Existing isn't the same as
correct; both checks below stay because agreeing with a genuinely
independent implementation is what actually catches something.

Usage: python3 tests/riscv/run_tests.py
"""

from __future__ import annotations

import pathlib
import subprocess
import sys
import tempfile

import decoder
import packer
import reference

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
CASES_DIR = pathlib.Path(__file__).resolve().parent / "cases"
BITTERASM_BIN = REPO_ROOT / "target" / "debug" / "bitterasm"


def build_bitterasm() -> None:
    subprocess.run(["cargo", "build", "--bin", "bitterasm"], cwd=REPO_ROOT, check=True)


def compile_case(source_path: pathlib.Path, workdir: pathlib.Path) -> pathlib.Path:
    basm_path = workdir / (source_path.stem + ".basm")
    em_path = workdir / (source_path.stem + ".em")

    basm_path.write_text("from std.riscv.native import *\n\n" + source_path.read_text())

    # cwd=REPO_ROOT matters: `from std.riscv.native import *` is an
    # absolute import, resolved against the current working directory,
    # not against basm_path's own (temp-directory) location.
    result = subprocess.run(
        [str(BITTERASM_BIN), "compile", str(basm_path), "-o", str(em_path)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        raise RuntimeError(f"bitterasm compile failed for {source_path.name}:\n{result.stderr}")

    return em_path


def run_case(source_path: pathlib.Path, workdir: pathlib.Path) -> list[str]:
    em_path = compile_case(source_path, workdir)
    actual_words = packer.pack_file(str(em_path))
    expected_words = reference.encode_file(str(source_path))

    parsed = [
        (line, reference.parse_line(line))
        for line in source_path.read_text().splitlines()
        if reference.parse_line(line) is not None
    ]

    failures = []

    if len(actual_words) != len(expected_words):
        failures.append(
            f"{source_path.name}: bitterasm emitted {len(actual_words)} value(s), "
            f"reference expected {len(expected_words)}"
        )
        return failures

    for (line, _parsed), actual, expected in zip(parsed, actual_words, expected_words):
        if actual != expected:
            failures.append(
                f"{source_path.name}: {line.strip()!r} -> "
                f"bitterasm 0x{actual:08x}, reference.py 0x{expected:08x}"
            )

    # Second, differently-sourced check: decode bitterasm's own words with
    # an external crate and confirm the fields it reports match what the
    # line actually asked for — catches a bug reference.py might share
    # with std/riscv/native.basm (both being this project's own code),
    # since riscv-decode is neither.
    decoded = decoder.decode_words(actual_words)

    for (line, parsed_line), decoding in zip(parsed, decoded):
        mnemonic, operands = parsed_line
        mnemonic = mnemonic.lower()
        expected_fields = reference.expected_fields(mnemonic, operands)

        if decoding.get("mnemonic") != mnemonic:
            failures.append(
                f"{source_path.name}: {line.strip()!r} -> riscv-decode read it back as "
                f"{decoding.get('mnemonic')!r}, not {mnemonic!r}"
            )
            continue

        for field, expected_value in expected_fields.items():
            if decoding.get(field) != expected_value:
                failures.append(
                    f"{source_path.name}: {line.strip()!r} -> riscv-decode's {field}="
                    f"{decoding.get(field)}, expected {expected_value}"
                )

    return failures


def main() -> int:
    build_bitterasm()
    decoder.build()

    case_files = sorted(CASES_DIR.glob("*.s"))
    if not case_files:
        print("no test cases found", file=sys.stderr)
        return 1

    all_failures: list[str] = []
    total_instructions = 0

    with tempfile.TemporaryDirectory() as tmp:
        workdir = pathlib.Path(tmp)

        for case_file in case_files:
            instruction_count = sum(
                1 for line in case_file.read_text().splitlines() if reference.parse_line(line) is not None
            )
            total_instructions += instruction_count

            try:
                failures = run_case(case_file, workdir)
            except Exception as error:  # noqa: BLE001 - report and keep going
                failures = [f"{case_file.name}: {error}"]

            all_failures.extend(failures)

            status = "ok" if not failures else f"{len(failures)} FAILED"
            print(f"{case_file.name}: {instruction_count} instruction(s) - {status}")

    print()

    if all_failures:
        print(f"{len(all_failures)} failure(s) out of {total_instructions} instruction(s):")
        for failure in all_failures:
            print(f"  {failure}")
        return 1

    print(f"all {total_instructions} instructions match")
    return 0


if __name__ == "__main__":
    sys.exit(main())
