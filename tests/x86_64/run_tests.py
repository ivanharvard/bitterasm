#!/usr/bin/env python3
"""Cross-checks std/x86_64/native.basm — through the real bitterasm -> bitter
pipeline — against a single independent oracle: GNU binutils' x86-64
assembler, run inside a Docker container (tests/x86_64/docker/), an existing,
independently-maintained "official" encoder this project didn't write and
shares no code with. Mirrors tests/riscv/run_tests.py's own standard and
structure, simplified where x86-64 doesn't need what RV32I's harness did:
no cross-toolchain package (plain Debian's `binutils` already targets x86-64
natively — see docker/Dockerfile), no `-march`/`-mabi` flags, no linker
`--relax` (x86-64 has no post-link jump-shrinking pass the way RISC-V's `ld`
does), and only one dialect so far (no second cases/*.c_like.basm-style
pairing to also check).

For each tests/x86_64/cases/*.s file (plain x86-64 Intel-syntax assembly
text — '#' comments, one instruction per line, real `label:` lines allowed)
this:

1. Builds a derived .basm by prepending `from std.x86_64.native import *` to
   the file's own text, unchanged otherwise — the .s file itself stays
   valid, ordinary-looking Intel-syntax assembly, and a `label:` line is
   valid BitterASM syntax too, so it needs no translation either way.
2. Compiles that with `bitterasm compile`, then packs the resulting .em's
   emitted values into raw machine-code bytes with `bitter encode` — the
   actual encoder under test.
3. Independently assembles the *original* .s file's text, whole, with the
   Docker oracle (see docker/assemble.sh) and compares the two raw byte
   streams byte-for-byte.

Every cases/*.s file deliberately keeps every jump/Jcc/call target more than
127 bytes from the instruction referencing it (see control_flow.s's own
comment) — GNU as auto-selects the short (rel8) encoding whenever a target
is close enough, but this project's own encoder is deliberately near-only
(rel32) by explicit design, so a close-enough target would make the two
sides disagree on encoding *length*, not correctness.

Every case sticks to this dialect's own real surface: real reg,reg and
bracket-memory forms share their real x86 mnemonic (`mov`/`add`/...), but a
reg,imm form doesn't — this project's own `movi`/`addi`/etc. naming (see
std/x86_64/PROGRESS.md's Phase 2 and Phase 6 notes on why `Reg` being a
plain `int` alias rules out overloading them onto `mov`/`add`) has no
counterpart in real Intel syntax, which spells both forms identically. GNU
as doesn't recognize `movi`/`addi` as instructions at all, and a bare
`mov rax, 100` silently means something different under this dialect (a
reg,reg `mov` misreading `100` as a register number — see
std/x86_64/native.basm's `assert_valid_reg`) than it does to a real
assembler, so reg,imm forms are outside what a shared `.s` file can safely
express and are excluded from this harness entirely; they're already
byte-verified by hand against the Intel SDM in tests/x86_64_encoding.rs.

Requires Docker; this builds the oracle image itself (cached by Docker after
the first run, `--platform linux/amd64` so it's genuine x86-64 binutils
regardless of the host machine's own architecture).

Usage: python3 tests/x86_64/run_tests.py
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
ORACLE_IMAGE = "bitterasm-x86_64-oracle"
ORACLE_PLATFORM = "linux/amd64"


def build_bitterasm() -> None:
    subprocess.run(["cargo", "build", "--bin", "bitterasm"], cwd=REPO_ROOT, check=True)


def build_bitter() -> None:
    subprocess.run(["cargo", "build", "--package", "bitter"], cwd=REPO_ROOT, check=True)


def build_oracle_image() -> None:
    subprocess.run(
        ["docker", "build", "--platform", ORACLE_PLATFORM, "-t", ORACLE_IMAGE, str(DOCKER_DIR)],
        check=True,
    )


def compile_case(source_path: pathlib.Path, workdir: pathlib.Path) -> pathlib.Path:
    basm_path = workdir / (source_path.stem + ".basm")
    basm_path.write_text("from std.x86_64.native import *\n\n" + source_path.read_text())
    em_path = workdir / (basm_path.stem + ".em")

    # cwd=REPO_ROOT matters: an absolute `from ... import *` is resolved
    # against the current working directory, not against basm_path's own
    # (temp-directory) location.
    result = subprocess.run(
        [str(BITTERASM_BIN), "compile", str(basm_path), "-o", str(em_path)],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        raise RuntimeError(f"bitterasm compile failed for {source_path.name}:\n{result.stderr}")

    return em_path


def encode_case(em_path: pathlib.Path, workdir: pathlib.Path) -> bytes:
    bin_path = workdir / (em_path.stem + ".bin")

    result = subprocess.run(
        [str(BITTER_BIN), "encode", str(em_path), "-o", str(bin_path)],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        raise RuntimeError(f"bitter encode failed for {em_path.name}:\n{result.stderr}")

    return bin_path.read_bytes()


def official_encode(source_path: pathlib.Path) -> bytes:
    with source_path.open("rb") as source:
        result = subprocess.run(
            ["docker", "run", "--rm", "-i", "--platform", ORACLE_PLATFORM, ORACLE_IMAGE],
            stdin=source,
            capture_output=True,
        )

    if result.returncode != 0:
        stderr = result.stderr.decode(errors="replace")
        raise RuntimeError(f"official encoder failed for {source_path.name}:\n{stderr}")

    return result.stdout


_LABEL_LINE = re.compile(r"^\w+\s*:$")


def _instruction_lines(text: str) -> list[str]:
    # A bare `label:` line is zero-width on both sides (no `@emit`, no
    # assembled bytes) and a comment line is invisible to both encoders —
    # excluded here so the count printed alongside each case's result means
    # "instructions", not "lines".
    lines = []

    for line in text.splitlines():
        content = line.split("#", 1)[0].strip()

        if content and not _LABEL_LINE.match(content):
            lines.append(line)

    return lines


def run_case(source_path: pathlib.Path, workdir: pathlib.Path) -> list[str]:
    em_path = compile_case(source_path, workdir)
    actual_bytes = encode_case(em_path, workdir)
    expected_bytes = official_encode(source_path)

    if actual_bytes != expected_bytes:
        return [
            f"{source_path.name}: byte mismatch — bitter encoded {len(actual_bytes)} byte(s), "
            f"official encoder produced {len(expected_bytes)} byte(s)\n"
            f"  bitter:   {actual_bytes.hex()}\n"
            f"  official: {expected_bytes.hex()}"
        ]

    return []


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
