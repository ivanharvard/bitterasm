"""Wraps the decoder/ Rust binary — a thin CLI over the `riscv-decode`
crate (an existing, independently-maintained RV32I decoder, not anything
derived from std/riscv/native.basm or reference.py). Feeding bitterasm's
own output through a real external tool and checking the decoded fields
match the source line's actual operands is a second, differently-sourced
check than reference.py's own bit-for-bit re-encoding.
"""

from __future__ import annotations

import pathlib
import subprocess

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]


def build() -> None:
    subprocess.run(
        ["cargo", "build", "--package", "riscv_decode_check"], cwd=REPO_ROOT, check=True
    )


def _binary_path() -> pathlib.Path:
    return REPO_ROOT / "target" / "debug" / "riscv_decode_check"


def decode_words(words: list[int]) -> list[dict[str, int | str]]:
    stdin = "\n".join(f"{word:#010x}" for word in words)

    result = subprocess.run(
        [str(_binary_path())], input=stdin, capture_output=True, text=True, check=True
    )

    return [_parse_line(line) for line in result.stdout.splitlines()]


def _parse_line(line: str) -> dict[str, int | str]:
    parts = line.split()
    fields: dict[str, int | str] = {"mnemonic": parts[0]}

    for part in parts[1:]:
        key, _, value = part.partition("=")
        fields[key] = int(value)

    return fields
