#!/usr/bin/env bash

# Companion to run.sh, which only measures bitterasm's compile-time
# throughput (macro expansion has no runtime of its own to compare). This
# script instead builds something that *does* have a runtime: a real WASM
# module (via std/wasm) computing the 32-bit-wrapping sum of i*i for i in
# 0..N, the same computation sum_squares.cpp/sum_squares.py do directly.
# For bitterasm that means three phases -- `bitterasm compile` (.basm ->
# .em), `bitter encode` (.em -> raw WASM bytes), and `wasmtime run`
# actually executing the result -- compared against python3's and
# g++ -O2's single runtime figure each (they have no separate "compile
# this specific N" step: g++ compiles once, outside the timed loop, into a
# binary that takes N as an argument; python3 has no compile phase at
# all).
#
# Two tables, from two different .basm shapes:
#
#   - sum_squares_N.basm: a WASM *runtime* loop (like a normal `for`
#     loop) that adds i*i to an accumulator N times. wasmtime JITs this
#     down to a handful of native instructions, so it runs in fractions of
#     a millisecond no matter how big N is -- and, since the module
#     bitterasm compiles is the same fixed ~60 emitted values regardless
#     of N, bitterasm's own compile/encode time barely changes with N
#     either. This table mostly answers "how fast is bitterasm's build
#     pipeline plus a JIT'd native loop", not "does bitterasm's compile
#     time scale with N".
#
#   - sum_squares_unrolled_N.basm: the same sum, but fully unrolled at
#     *compile* time via `@for` -- N pairs of `i32_const(i*i); i32_add`
#     emitted directly, no WASM loop at all (the same shape
#     for_loop_N.basm uses). This is the one that actually stresses
#     bitterasm's compile time as N grows, which is also why it stops at
#     N=100000 rather than N=1000000: careful (median-of-repeats) timing
#     shows compile time here is linear in N with a large constant, not
#     quadratic -- but that constant is still big enough that N=1000000
#     would take minutes, versus for_loop_1000000.basm's flat Int
#     emission, which compiles in single-digit seconds. The size of that
#     constant traced back to a real bitterasm characteristic this
#     benchmark surfaced: `AliasResolver::resolve_macro_overload` and
#     friends used to deep-clone a whole `MacroDeclaration` (including its
#     entire body) on every single macro call, so a call chain a dozen
#     macros deep (`i32_const` -> `op_with_sleb128` -> `sleb128_length`/
#     `sleb128_padded` -> ... ) paid for that clone at every link, on
#     every one of the N unrolled statements. Fixed by caching each
#     declaration behind an `Rc` (see `macro_decl_cache` in
#     `src/resolver/aliases.rs`) so repeat lookups are a refcount bump
#     instead of an AST clone -- roughly a 35-40% wall-time cut on this
#     benchmark, not a change in its growth shape.
#
# Every number reported (time and memory alike) is the median of several
# repeated runs, not a single sample -- the first exec of a freshly built
# binary in a run pays for disk/page-cache warm-up and dynamic-linker work
# that has nothing to do with N, and at these timescales (sub-millisecond
# to low-millisecond for the loop table) that one-time cost can dwarf the
# real number. Taking a median (rather than a mean) throws that kind of
# one-off outlier away without needing to identify it explicitly.
#
# Peak memory is each process's own maximum resident set size (BSD
# `/usr/bin/time -l`'s "maximum resident set size", in bytes) -- how much
# RAM that one process ever held at once, not memory shared across the
# three sequential bitterasm/bitter/wasmtime phases (they never run
# concurrently, so summing their peaks the way `bt total` sums wall time
# wouldn't mean "peak RAM this pipeline needs"; the max of the three does).
#
# Requires `wasmtime` on PATH (e.g. `brew install wasmtime`) and BSD's
# `/usr/bin/time -l` (macOS ships this; Linux's `/usr/bin/time` needs `-v`
# and a differently-shaped output this script doesn't parse).
#
# Usage: tests/benchmarks/run_runtime.sh

set -euo pipefail

cd "$(dirname "$0")/../.."
repo_root="$(pwd)"
bench_dir="$repo_root/tests/benchmarks"
work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

if ! command -v wasmtime >/dev/null 2>&1; then
    echo "error: wasmtime not found on PATH -- install it first (e.g. \`brew install wasmtime\`" \
         "or https://wasmtime.dev/) and re-run this script." >&2
    exit 1
fi

if ! /usr/bin/time -l true >/dev/null 2>&1; then
    echo "error: /usr/bin/time -l isn't available (this script's memory measurements need" \
         "BSD time's -l flag, e.g. on macOS) -- re-run on a machine that has it." >&2
    exit 1
fi

echo "Building bitterasm and bitter (release)..." >&2
cargo build --release --quiet

echo "Building the C++ comparison program (g++ -O2)..." >&2
g++ -O2 -o "$work_dir/sum_squares_cpp" "$bench_dir/sum_squares.cpp"

# Runs one command once under `/usr/bin/time -l`, printing its wall-clock
# seconds and peak RSS (bytes) as two lines. One invocation of the target
# command, not two -- both numbers come from the same run, rather than
# timing once and separately measuring memory on a second run.
sample() {
    { /usr/bin/time -l "$@" >/dev/null; } 2>"$work_dir/time_raw"
    awk '
        /real/ && !t { t = $1 }
        /maximum resident set size/ { m = $1 }
        END { print t; print m }
    ' "$work_dir/time_raw"
}

# Median seconds are sortable as plain decimals; bytes are plain integers
# either way, so one sort/middle-pick routine covers both.
median() {
    printf '%s\n' "$@" | sort -n | sed -n "$(((${#} + 1) / 2))p"
}

# Runs $reps repetitions of one command, setting the globals ROW_TIME and
# ROW_MEM to the median wall-clock seconds and median peak-RSS bytes
# across them. $reps must be odd, so "middle" is unambiguous -- and a
# median (rather than a mean) is what actually discards a one-off cold-
# start outlier (the first exec of a freshly built binary paying for
# disk/page-cache warm-up) instead of just diluting it.
ROW_TIME=
ROW_MEM=
measure_median() {
    local reps="$1"
    shift
    local times=() mems=() result t m
    for ((rep = 0; rep < reps; rep++)); do
        result=$(sample "$@")
        t=$(sed -n 1p <<<"$result")
        m=$(sed -n 2p <<<"$result")
        times+=("$t")
        mems+=("$m")
    done
    ROW_TIME=$(median "${times[@]}")
    ROW_MEM=$(median "${mems[@]}")
}

# Peak RSS in bytes -> a short human-readable string (KB below 1000K,
# MB above), matched to the ~8-char column width the memory tables use.
human_bytes() {
    awk -v b="$1" 'BEGIN {
        if (b >= 1000 * 1000) printf "%.1fMB", b / (1000 * 1000)
        else printf "%.0fKB", b / 1000
    }'
}

# One untimed run, capturing stdout into "$work_dir/out" -- for the
# cross-check below, where we care about the answer, not the clock.
capture() {
    "$@" >"$work_dir/out" 2>/dev/null
    cat "$work_dir/out"
}

sum3() {
    awk -v a="$1" -v b="$2" -v c="$3" 'BEGIN { printf "%.2f", a + b + c }'
}

max3() {
    printf '%s\n' "$1" "$2" "$3" | sort -n | tail -1
}

# Runs one N's row for either table: $1 the basm file's path, $2 the
# module's own name-stem for temp files, $3 N (passed to python3/g++),
# $4 the repeat count for this table. Prints one time line and one memory
# line.
run_row() {
    local basm_file="$1" stem="$2" n="$3" reps="$4"
    local em_file="$work_dir/$stem.em"
    local wasm_file="$work_dir/$stem.wasm"

    local compile_t compile_m encode_t encode_m run_t run_m total_t peak_m py_t py_m cpp_t cpp_m

    measure_median "$reps" "$repo_root/target/release/bitterasm" compile "$basm_file" -o "$em_file"
    compile_t=$ROW_TIME compile_m=$ROW_MEM

    measure_median "$reps" "$repo_root/target/release/bitter" encode "$em_file" -o "$wasm_file"
    encode_t=$ROW_TIME encode_m=$ROW_MEM

    measure_median "$reps" wasmtime run --invoke sum_squares "$wasm_file"
    run_t=$ROW_TIME run_m=$ROW_MEM

    total_t=$(sum3 "$compile_t" "$encode_t" "$run_t")
    peak_m=$(max3 "$compile_m" "$encode_m" "$run_m")

    measure_median "$reps" python3 "$bench_dir/sum_squares.py" "$n"
    py_t=$ROW_TIME py_m=$ROW_MEM

    measure_median "$reps" "$work_dir/sum_squares_cpp" "$n"
    cpp_t=$ROW_TIME cpp_m=$ROW_MEM

    # Cross-check every path agrees on the actual sum, not just its
    # timing -- one untimed run each, separate from the medians above.
    # `wasmtime --invoke` prints an i32 result as signed decimal (WASM's
    # i32 has no inherent signedness; that's just the CLI's choice), while
    # python3/g++ print the same 32-bit pattern unsigned -- so a negative
    # reading here is folded back into [0, 2^32) before comparing, not an
    # actual mismatch.
    local wasm_result py_result cpp_result
    wasm_result=$(capture wasmtime run --invoke sum_squares "$wasm_file")
    if [ "$wasm_result" -lt 0 ]; then
        wasm_result=$((wasm_result + 4294967296))
    fi
    py_result=$(capture python3 "$bench_dir/sum_squares.py" "$n")
    cpp_result=$(capture "$work_dir/sum_squares_cpp" "$n")
    if [ "$wasm_result" != "$py_result" ] || [ "$cpp_result" != "$py_result" ]; then
        echo "MISMATCH at N=$n: wasmtime=$wasm_result python3=$py_result c++=$cpp_result" >&2
        exit 1
    fi

    printf 'time     %-10s %11ss %11ss %11ss %11ss %11ss %11ss\n' \
        "$n" "$compile_t" "$encode_t" "$run_t" "$total_t" "$py_t" "$cpp_t"
    printf 'mem      %-10s %12s %12s %12s %12s %12s %12s\n' \
        "$n" "$(human_bytes "$compile_m")" "$(human_bytes "$encode_m")" "$(human_bytes "$run_m")" \
        "$(human_bytes "$peak_m")" "$(human_bytes "$py_m")" "$(human_bytes "$cpp_m")"
}

header_row() {
    printf '%-9s %-10s %12s %12s %12s %12s %12s %12s\n' \
        "" "N" "bt compile" "bt encode" "bt run" "bt total/peak" "python3" "c++ (-O2)"
}

echo
echo "== runtime loop (sum_squares_N.basm), median of 5 runs =="
header_row
for n in 1000 100000 1000000; do
    run_row "$bench_dir/sum_squares_${n}.basm" "sum_squares_${n}" "$n" 5
done

echo
echo "== unrolled at compile time (sum_squares_unrolled_N.basm), median of 3 runs =="
header_row
for n in 1000 10000 100000; do
    run_row "$bench_dir/sum_squares_unrolled_${n}.basm" "sum_squares_unrolled_${n}" "$n" 3
done
