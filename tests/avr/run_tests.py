#!/usr/bin/env python3
"""Cross-checks std/avr/native.basm, through the real bitterasm -> bitter
pipeline, against GNU binutils' AVR assembler run inside a Docker container
(tests/avr/docker/), an independent encoder this project didn't write.

For each tests/avr/cases/*.s file (plain AVR assembly, `#` comment lines,
one instruction per line, real `label:` lines allowed) this:

1. Builds a .basm by prepending `from std.avr.native import *` to the
   file's text, unchanged otherwise.
2. Compiles it with `bitterasm compile` and packs it with `bitter encode`.
3. Assembles the original .s file with the Docker oracle (see
   docker/assemble.sh) and compares the two byte streams instruction by
   instruction.

Requires Docker; this builds the oracle image itself (cached by Docker
after the first run).

Usage: python3 tests/avr/run_tests.py
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
ORACLE_IMAGE = "bitterasm-avr-oracle"

# The instructions two words long. Every other one is one word.
LONG = {"jmp", "call", "lds", "sts"}

_LABEL_LINE = re.compile(r"^\w+\s*:$")


def build() -> None:
    subprocess.run(["cargo", "build", "--workspace"], cwd=REPO_ROOT, check=True)
    subprocess.run(["docker", "build", "-q", "-t", ORACLE_IMAGE, str(DOCKER_DIR)], check=True, stdout=subprocess.DEVNULL)


def bitter_encode(source_path: pathlib.Path, workdir: pathlib.Path) -> bytes:
    text = source_path.read_text()
    basm_path = workdir / (source_path.stem + ".basm")
    basm_path.write_text("from std.avr.native import *\n\n" + text)
    em_path = workdir / (source_path.stem + ".em")
    bin_path = workdir / (source_path.stem + ".bin")

    # cwd=REPO_ROOT: `from std...` resolves against the working directory.
    result = subprocess.run(
        [str(BITTERASM_BIN), "compile", str(basm_path), "-o", str(em_path)],
        cwd=REPO_ROOT, capture_output=True, text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"bitterasm compile failed for {source_path.name}:\n{result.stderr}")

    result = subprocess.run([str(BITTER_BIN), "encode", str(em_path), "-o", str(bin_path)], capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"bitter encode failed for {source_path.name}:\n{result.stderr}")

    return bin_path.read_bytes()


def official_encode(source_path: pathlib.Path) -> bytes:
    with source_path.open("rb") as source:
        result = subprocess.run(["docker", "run", "--rm", "-i", ORACLE_IMAGE], stdin=source, capture_output=True)
    if result.returncode != 0:
        raise RuntimeError(f"official encoder failed for {source_path.name}:\n{result.stderr.decode(errors='replace')}")
    return result.stdout


def instructions(text: str) -> list[str]:
    lines = []
    for line in text.splitlines():
        content = line.split("#", 1)[0].strip()
        if content and not _LABEL_LINE.match(content):
            lines.append(content)
    return lines


def run_case(source_path: pathlib.Path, workdir: pathlib.Path) -> list[str]:
    actual = bitter_encode(source_path, workdir)
    expected = official_encode(source_path)

    failures = []
    offset = 0
    for line in instructions(source_path.read_text()):
        size = 4 if line.split()[0] in LONG else 2
        got, want = actual[offset:offset + size], expected[offset:offset + size]
        if got != want:
            failures.append(f"{source_path.name}: {line!r} -> bitter {got.hex(' ')}, official encoder {want.hex(' ')}")
        offset += size

    if len(actual) != len(expected):
        failures.append(f"{source_path.name}: bitter encoded {len(actual)} byte(s), official encoder {len(expected)}")

    return failures


def main() -> int:
    build()
    cases = sorted(CASES_DIR.glob("*.s"))
    failures = []
    with tempfile.TemporaryDirectory() as workdir:
        for case in cases:
            try:
                failures.extend(run_case(case, pathlib.Path(workdir)))
            except RuntimeError as error:
                failures.append(str(error))

    for failure in failures:
        print(failure)
    count = sum(len(instructions(case.read_text())) for case in cases)
    print(f"{len(cases)} case file(s), {count} instruction(s), {len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
