#!/bin/bash
# Reads a raw `.wasm` module on stdin. First checks it's structurally
# valid with WABT's own `wasm-validate`, then runs every zero-argument
# exported function with `wasm-interp --run-all-exports` and prints its
# result — for `tests/wasm/cases/module.basm`'s `sum_to_ten`, this prints
# `sum_to_ten() => i32:45`.
#
# This checks *behavior*, not byte-for-byte output, unlike
# tests/riscv/docker's oracle: RV32I has exactly one correct encoding per
# instruction, so byte identity with GNU binutils is the right test. WASM
# doesn't — its own binary format spec explicitly permits non-minimal
# LEB128 (padded with extra continuation groups up to a value's max byte
# count), which is exactly what `std/wasm/module.basm`'s `deferred_uleb128`
# deliberately produces for a length prefix that isn't known until
# everything after it has been emitted (see that file's own doc comment).
# A byte-for-byte diff against WABT's own (minimal-LEB128) output would
# fail on bytes that are still perfectly valid WASM, so the oracle instead
# confirms what actually matters: WABT accepts the module as valid, and
# running it produces the value the instructions are supposed to compute.
set -euo pipefail

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

cat >"$workdir/in.wasm"

if ! wasm-validate "$workdir/in.wasm" 2>"$workdir/err"; then
    cat "$workdir/err" >&2
    echo "oracle: module failed wasm-validate" >&2
    exit 1
fi

wasm-interp --run-all-exports "$workdir/in.wasm"
