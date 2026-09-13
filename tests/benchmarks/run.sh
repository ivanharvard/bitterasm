#!/usr/bin/env bash

# Compares bitterasm's compile-time throughput against Python and C++ doing
# the same generative work: compute i*i for i in 0..N and write each as a
# {"kind":"Int","value":...} record, the shape bitterasm's own .em output
# uses. bitterasm has no runtime of its own -- `compile` (parse, resolve
# every macro/@for/`in`, emit) is its only execution phase -- so this is a
# compile-time macro-expansion throughput benchmark, not a measurement of
# how fast programs written in it run once emitted.
#
# Usage: tests/benchmarks/run.sh

set -euo pipefail

cd "$(dirname "$0")/../.."
repo_root="$(pwd)"
bench_dir="$repo_root/tests/benchmarks"
work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

echo "Building bitterasm (release)..." >&2
cargo build --release --quiet

echo "Building the C++ comparison program (g++ -O2)..." >&2
g++ -O2 -o "$work_dir/for_loop_cpp" "$bench_dir/for_loop.cpp"

# Real (wall-clock) seconds for one command, via bash's own `time` builtin
# rather than an external `time`/`bc` dependency.
elapsed() {
    TIMEFORMAT='%R'
    { time "$@" >/dev/null; } 2>"$work_dir/t"
    cat "$work_dir/t"
}

printf '%-10s %12s %12s %12s\n' "N" "bitterasm" "python3" "c++ (-O2)"

for n in 1000 100000 1000000; do
    basm_file="$bench_dir/for_loop_${n}.basm"

    bt=$(elapsed "$repo_root/target/release/bitterasm" compile "$basm_file" -o "$work_dir/out_$n.em")
    py=$(elapsed python3 "$bench_dir/for_loop.py" "$n" "$work_dir/out_py_$n.em")
    cpp=$(elapsed "$work_dir/for_loop_cpp" "$n" "$work_dir/out_cpp_$n.em")

    printf '%-10s %11ss %11ss %11ss\n' "$n" "$bt" "$py" "$cpp"
done
