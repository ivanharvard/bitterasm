#!/usr/bin/env python3
"""Cross-checks std/riscv/native.basm — through the real bitterasm -> bitter
pipeline — against a single independent oracle: GNU binutils' RISC-V
assembler, run inside a Docker container (tests/riscv/docker/), an
existing, independently-maintained "official" encoder this project didn't
write and shares no code with.

For each tests/riscv/cases/*.s file (plain RISC-V assembly text — '#'
comments, one instruction per line, real `label:` lines allowed) this:

1. Builds a derived .basm by prepending `from std.riscv.native import *`
   to the file's own text, unchanged otherwise — the .s file itself stays
   valid, ordinary-looking assembly, and a `label:` line is valid BitterASM
   syntax too (`ast::Statement::Label`), so it needs no translation either
   way.
2. Compiles that with `bitterasm compile`, then packs the resulting .em's
   emitted values into raw little-endian machine-code bytes with `bitter
   encode` — the actual encoder under test.
3. Independently assembles the *original* .s file's text, whole, with the
   Docker oracle (see docker/assemble.sh) and compares the two raw byte
   streams word-for-word. Real labels only, not raw-literal branch/jump
   offsets — see assemble.sh's own doc comment for why a bare literal
   can no longer agree with this oracle at all now that
   std/riscv/native.basm's branch/jump macros take a target *instruction*
   rather than a raw byte delta.

Requires Docker; this builds the oracle image itself (cached by Docker
after the first run) from tests/riscv/docker/.

Usage: python3 tests/riscv/run_tests.py
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys
import tempfile

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
CASES_DIR = pathlib.Path(__file__).resolve().parent / "cases"
DOCKER_DIR = pathlib.Path(__file__).resolve().parent / "docker"
BITTERASM_BIN = REPO_ROOT / "target" / "debug" / "bitterasm"
BITTER_BIN = REPO_ROOT / "target" / "debug" / "bitter"
ORACLE_IMAGE = "bitterasm-riscv-oracle"


def build_bitterasm() -> None:
    subprocess.run(["cargo", "build", "--bin", "bitterasm"], cwd=REPO_ROOT, check=True)


def build_bitter() -> None:
    subprocess.run(["cargo", "build", "--package", "bitter"], cwd=REPO_ROOT, check=True)


def build_oracle_image() -> None:
    subprocess.run(["docker", "build", "-t", ORACLE_IMAGE, str(DOCKER_DIR)], check=True)


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


def encode_case(em_path: pathlib.Path, workdir: pathlib.Path) -> list[int]:
    bin_path = workdir / (em_path.stem + ".bin")

    result = subprocess.run(
        [str(BITTER_BIN), "encode", str(em_path), "-o", str(bin_path)],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        raise RuntimeError(f"bitter encode failed for {em_path.name}:\n{result.stderr}")

    # RV32I words are 32 bits, little-endian — `bitter encode`'s default
    # byte order (see bitter/src/pack.rs) and the target ISA's actual one.
    data = bin_path.read_bytes()
    return [int.from_bytes(data[i : i + 4], "little") for i in range(0, len(data), 4)]


def official_encode(source_path: pathlib.Path) -> list[int]:
    with source_path.open("rb") as source:
        result = subprocess.run(
            ["docker", "run", "--rm", "-i", ORACLE_IMAGE],
            stdin=source,
            capture_output=True,
        )

    if result.returncode != 0:
        stderr = result.stderr.decode(errors="replace")
        raise RuntimeError(f"official encoder failed for {source_path.name}:\n{stderr}")

    data = result.stdout
    return [int.from_bytes(data[i : i + 4], "little") for i in range(0, len(data), 4)]


_LABEL_LINE = re.compile(r"^\w+\s*:$")


def _instruction_lines(text: str) -> list[str]:
    # A bare `label:` line is zero-width on both sides (no `@emit`, no
    # assembled bytes) — excluded here so the remaining lines stay in
    # exact 1:1 correspondence with the word streams being compared.
    lines = []

    for line in text.splitlines():
        content = line.split("#", 1)[0].strip()

        if content and not _LABEL_LINE.match(content):
            lines.append(line)

    return lines


def run_case(source_path: pathlib.Path, workdir: pathlib.Path) -> list[str]:
    em_path = compile_case(source_path, workdir)
    actual_words = encode_case(em_path, workdir)
    expected_words = official_encode(source_path)
    lines = _instruction_lines(source_path.read_text())

    failures = []

    if len(actual_words) != len(expected_words):
        failures.append(
            f"{source_path.name}: bitter encoded {len(actual_words)} word(s), "
            f"official encoder produced {len(expected_words)}"
        )
        return failures

    for line, actual, expected in zip(lines, actual_words, expected_words):
        if actual != expected:
            failures.append(
                f"{source_path.name}: {line.strip()!r} -> "
                f"bitter 0x{actual:08x}, official encoder 0x{expected:08x}"
            )

    return failures


def main() -> int:
    build_bitterasm()
    build_bitter()
    build_oracle_image()

    case_files = sorted(CASES_DIR.glob("*.s"))
    if not case_files:
        print("no test cases found", file=sys.stderr)
        return 1

    all_failures: list[str] = []
    total_instructions = 0

    with tempfile.TemporaryDirectory() as tmp:
        workdir = pathlib.Path(tmp)

        for case_file in case_files:
            instruction_count = len(_instruction_lines(case_file.read_text()))
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
