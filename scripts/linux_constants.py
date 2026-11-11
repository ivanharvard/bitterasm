#!/usr/bin/env python3
"""Generates std/linux/x86_64.basm from the Linux kernel's own userspace
headers, the ones C programs include: syscall numbers, errno values, and
the flags the common syscalls take.

The headers come from Debian's `linux-libc-dev`, copied out of a
container, so the numbers are the kernel's, not retyped by hand.

Usage: python3 scripts/linux_constants.py
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import tempfile
import textwrap

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]
OUTPUT = REPO_ROOT / "std" / "linux" / "x86_64.basm"

HEADERS = {
    "unistd_64.h": "x86_64-linux-gnu/asm/unistd_64.h",
    "errno-base.h": "asm-generic/errno-base.h",
    "errno.h": "asm-generic/errno.h",
    "fcntl.h": "asm-generic/fcntl.h",
    "linux_fcntl.h": "linux/fcntl.h",
    "mman-common.h": "asm-generic/mman-common.h",
    "mman.h": "linux/mman.h",
    "x86_mman.h": "x86_64-linux-gnu/asm/mman.h",
    "signal.h": "x86_64-linux-gnu/asm/signal.h",
    "time.h": "linux/time.h",
}

# What each flag means, where the header's own comment doesn't say it in
# one line. An alias says what it's an alias of instead.
DESCRIPTIONS = {
    "O_ACCMODE": "The bits that say how a file is opened: `O_RDONLY`, `O_WRONLY` or `O_RDWR`",
    "O_RDONLY": "Open for reading only",
    "O_WRONLY": "Open for writing only",
    "O_RDWR": "Open for reading and writing",
    "O_CREAT": "Create the file if it doesn't exist, with the mode in the next argument",
    "O_EXCL": "With `O_CREAT`, fail with `EEXIST` if the file exists",
    "O_NOCTTY": "Don't make a terminal this opens the controlling terminal",
    "O_TRUNC": "Truncate an existing file to length 0",
    "O_APPEND": "Write at the end of the file every time",
    "O_NONBLOCK": "Don't block: an operation that would fails with `EAGAIN`",
    "O_DSYNC": "Each write waits until its data reaches the disk",
    "O_DIRECT": "Bypass the page cache where possible",
    "O_LARGEFILE": "Allow files larger than 2 GiB; always on for 64-bit programs",
    "O_DIRECTORY": "Fail with `ENOTDIR` unless the path is a directory",
    "O_NOFOLLOW": "Fail with `ELOOP` if the path's last part is a symbolic link",
    "O_NOATIME": "Don't update the file's access time",
    "O_CLOEXEC": "Close the descriptor when the program calls `execve`",
    "O_SYNC": "Each write waits until its data and metadata reach the disk",
    "O_PATH": "Open only a reference to the path, for `*at` calls and `fstat`",
    "O_TMPFILE": "Create an unnamed file in the directory given",
    "AT_FDCWD": "As a directory descriptor: the current working directory",
    "AT_SYMLINK_NOFOLLOW": "Don't follow a symbolic link at the end of the path",
    "AT_EACCESS": "For `faccessat`: check with the effective user and group IDs",
    "AT_REMOVEDIR": "For `unlinkat`: remove a directory, like `rmdir`",
    "AT_SYMLINK_FOLLOW": "For `linkat`: follow a symbolic link at the end of the path",
    "AT_NO_AUTOMOUNT": "Don't trigger an automount at the end of the path",
    "AT_EMPTY_PATH": "With an empty path, act on the directory descriptor itself",
    "AT_STATX_SYNC_TYPE": "For `statx`: the bits that say how to sync with a remote filesystem",
    "AT_STATX_SYNC_AS_STAT": "For `statx`: sync the way `stat` does",
    "AT_STATX_FORCE_SYNC": "For `statx`: sync the attributes with the server",
    "AT_STATX_DONT_SYNC": "For `statx`: don't sync the attributes with the server",
    "AT_RECURSIVE": "Apply to the whole subtree",
    "PROT_NONE": "The pages can't be accessed",
    "PROT_SEM": "The pages may be used for atomic operations",
    "PROT_GROWSDOWN": "For `mprotect`: extend the change down a stack that grows down",
    "PROT_GROWSUP": "For `mprotect`: extend the change up a mapping that grows up",
    "MAP_TYPE": "The bits that say how a mapping is shared",
    "MAP_FIXED": "Map at exactly the address given, replacing what's there",
    "MAP_ANONYMOUS": "Map zeroed memory, not a file; the descriptor is ignored",
    "MAP_POPULATE": "Fault the pages in now",
    "MAP_NONBLOCK": "With `MAP_POPULATE`, don't block on reading the file",
    "MAP_STACK": "The mapping is for a stack",
    "MAP_HUGETLB": "Map with huge pages",
    "MAP_SYNC": "Writes to a mapped persistent-memory file reach it synchronously",
    "MAP_FIXED_NOREPLACE": "Map at exactly the address given, failing with `EEXIST` if something is there",
    "MAP_UNINITIALIZED": "Don't zero anonymous pages, where the kernel allows it",
    "MAP_FILE": "No effect: mapping a file is the default",
    "MAP_SHARED": "Share changes with other mappings of the same file",
    "MAP_PRIVATE": "Keep changes private, copied on write",
    "MAP_SHARED_VALIDATE": "`MAP_SHARED`, failing on flags the kernel doesn't know",
    "MAP_32BIT": "Map in the first 2 GiB of the address space",
    "SIGHUP": "Hangup: the terminal closed, or reload configuration",
    "SIGINT": "Interrupt from the keyboard, Ctrl-C",
    "SIGQUIT": "Quit from the keyboard, Ctrl-\\",
    "SIGILL": "Illegal instruction",
    "SIGTRAP": "Breakpoint or trace trap",
    "SIGABRT": "Abort, as from `abort()`",
    "SIGIOT": "Another name for `SIGABRT`",
    "SIGBUS": "Bus error: a bad memory access, like past the end of a mapped file",
    "SIGFPE": "Arithmetic error, like division by zero",
    "SIGKILL": "Kill; can't be caught or ignored",
    "SIGUSR1": "User-defined signal 1",
    "SIGSEGV": "Invalid memory reference",
    "SIGUSR2": "User-defined signal 2",
    "SIGPIPE": "Write to a pipe with no reader",
    "SIGALRM": "Timer from `alarm`",
    "SIGTERM": "Termination request",
    "SIGSTKFLT": "Coprocessor stack fault; unused",
    "SIGCHLD": "A child process stopped or exited",
    "SIGCONT": "Continue if stopped",
    "SIGSTOP": "Stop; can't be caught or ignored",
    "SIGTSTP": "Stop from the terminal, Ctrl-Z",
    "SIGTTIN": "Terminal input for a background process",
    "SIGTTOU": "Terminal output from a background process",
    "SIGURG": "Urgent data on a socket",
    "SIGXCPU": "CPU time limit exceeded",
    "SIGXFSZ": "File size limit exceeded",
    "SIGVTALRM": "Virtual timer from `setitimer`",
    "SIGPROF": "Profiling timer",
    "SIGWINCH": "The terminal window changed size",
    "SIGIO": "I/O is possible on a descriptor",
    "SIGPWR": "Power failure",
    "SIGSYS": "Bad system call",
    "CLOCK_REALTIME": "Wall-clock time, which can jump when it's set",
    "CLOCK_MONOTONIC": "Time since some fixed point, never set, not counting suspend",
    "CLOCK_PROCESS_CPUTIME_ID": "CPU time used by the process",
    "CLOCK_THREAD_CPUTIME_ID": "CPU time used by the thread",
    "CLOCK_MONOTONIC_RAW": "`CLOCK_MONOTONIC`, without NTP's adjustments",
    "CLOCK_REALTIME_COARSE": "`CLOCK_REALTIME`, faster to read and less precise",
    "CLOCK_MONOTONIC_COARSE": "`CLOCK_MONOTONIC`, faster to read and less precise",
    "CLOCK_BOOTTIME": "`CLOCK_MONOTONIC`, counting time suspended",
    "CLOCK_REALTIME_ALARM": "`CLOCK_REALTIME`, waking the system from suspend",
    "CLOCK_BOOTTIME_ALARM": "`CLOCK_BOOTTIME`, waking the system from suspend",
    "CLOCK_TAI": "International Atomic Time",
}

# Defined but obsolete, or only inside a comment.
SKIPPED = {"SIGLOST", "SIGUNUSED", "SIGRTMIN", "SIGRTMAX", "SIGSTKSZ", "CLOCK_SGI_CYCLE"}

DEFINE = re.compile(r"^#define\s+(\w+)\s+(.+?)\s*(?:/\*\s*(.*?)\s*(?:\*/)?)?$")


def fetch_headers(directory: pathlib.Path) -> str:
    copies = " && ".join(f"cp /usr/include/{source} /out/{name}" for name, source in HEADERS.items())
    script = (
        "apt-get update -qq >/dev/null"
        " && apt-get install -y -qq --no-install-recommends linux-libc-dev >/dev/null 2>&1"
        f" && {copies}"
        " && dpkg-query -W -f '${Version}' linux-libc-dev"
    )
    result = subprocess.run(
        ["docker", "run", "--rm", "-v", f"{directory}:/out", "debian:bookworm-slim", "sh", "-c", script],
        check=True, capture_output=True, text=True,
    )
    return result.stdout.strip()


def defines(path: pathlib.Path) -> list[tuple[str, str, str]]:
    found = []
    for line in path.read_text().splitlines():
        match = DEFINE.match(line.strip())
        if match:
            name, value, comment = match.groups()
            found.append((name, value.strip(), (comment or "").strip()))
    return found


def evaluate(value: str, known: dict[str, int]) -> int | None:
    """A header's value as a number: a literal (C octal included), or an
    expression over names already seen."""
    expression = re.sub(r"\b0([0-7]+)\b", lambda m: str(int(m.group(1), 8)), value)
    expression = re.sub(r"\b(\w+)\b", lambda m: str(known[m.group(1)]) if m.group(1) in known else m.group(1), expression)
    if not re.fullmatch(r"[0-9xa-fA-F()|+\- ]+", expression):
        return None
    try:
        return int(eval(expression, {"__builtins__": {}}))  # digits and operators only, checked above
    except (SyntaxError, NameError):
        return None


def literal(number: int) -> str:
    return str(number) if -1000 < number < 1000 else hex(number)


def doc(comment: str, fallback: str) -> str:
    text = comment.rstrip(" .*/") or fallback
    if text[0].islower() and not text.startswith(("e.g", "i.e")):
        text = text[0].upper() + text[1:]
    return text + "."


def section(title: str, intro: list[str]) -> list[str]:
    return ["", "# =================", f"# {title}", "# =================", *[f"# {line}" for line in intro], ""]


def main() -> None:
    with tempfile.TemporaryDirectory() as temp:
        directory = pathlib.Path(temp)
        version = fetch_headers(directory)
        headers = {name: defines(directory / name) for name in HEADERS}

    known: dict[str, int] = {}
    out = [
        "#! Linux on x86-64: its system call numbers, errno values, and the flags",
        "#! the common system calls take, named the way the kernel's C headers",
        "#! name them.",
        "#!",
        "#! ```",
        "#! from std.x86_64.nasm import *",
        "#! from std.linux.x86_64 import *",
        "#!",
        "#! mov eax, SYS_exit",
        "#! mov edi, 0",
        "#! syscall",
        "#! ```",
        "#!",
        "#! ```bytes",
        "#! b8 3c 00 00 00",
        "#! bf 00 00 00 00",
        "#! 0f 05",
        "#! ```",
        "#!",
        "#! A system call takes its number in `rax` and its arguments in `rdi`,",
        "#! `rsi`, `rdx`, `r10`, `r8` and `r9`, and returns in `rax`: a result, or",
        "#! minus an errno value.",
        "#!",
        f"#! Generated by `scripts/linux_constants.py` from linux-libc-dev {version}.",
    ]

    def emit(name: str, value: str, comment: str, fallback: str) -> None:
        number = evaluate(value, known)
        if number is None or name in SKIPPED:
            return
        known[name] = number
        if re.fullmatch(r"\w+", value) and value in known:
            text = f"Another name for `{value}`."
        else:
            text = doc(DESCRIPTIONS.get(name, comment), fallback)
        out.extend(f"## {line}" for line in textwrap.wrap(text, 74))
        out.append(f"pub const {name} = {literal(number)}")

    out += section("system calls", ["Each system call's number, for `rax`."])
    for name, value, _ in headers["unistd_64.h"]:
        if name.startswith("__NR_"):
            call = name.removeprefix("__NR_")
            emit(f"SYS_{call}", value, "", f"The `{call}` system call")

    out += section("standard streams", [])
    for name, number, comment in [
        ("STDIN_FILENO", "0", "Standard input"),
        ("STDOUT_FILENO", "1", "Standard output"),
        ("STDERR_FILENO", "2", "Standard error"),
    ]:
        emit(name, number, comment, "")

    out += section("errno values", ["A failed system call returns minus one of these."])
    for header in ["errno-base.h", "errno.h"]:
        for name, value, comment in headers[header]:
            if re.fullmatch(r"E[A-Z0-9]+", name):
                emit(name, value, comment, f"`{name}`")

    out += section("open flags", ["For `open` and `openat`. Exactly one of the first three is set."])
    for name, value, comment in headers["fcntl.h"]:
        if name.startswith("__O_"):
            number = evaluate(value, known)
            if number is not None:
                known[name] = number
        elif name.startswith("O_"):
            emit(name, value, comment, f"`{name}`")

    out += section("*at flags", ["For `openat`, `fstatat`, `unlinkat` and the other `*at` calls."])
    for name, value, comment in headers["linux_fcntl.h"]:
        if name.startswith("AT_"):
            emit(name, value, comment, f"`{name}`")

    out += section("lseek", ["Where `lseek`'s offset counts from."])
    for name, number, comment in [
        ("SEEK_SET", "0", "From the start of the file"),
        ("SEEK_CUR", "1", "From the current position"),
        ("SEEK_END", "2", "From the end of the file"),
    ]:
        emit(name, number, comment, "")

    out += section("memory mapping", ["For `mmap`, `mprotect` and `munmap`."])
    for header in ["mman-common.h", "mman.h", "x86_mman.h"]:
        for name, value, comment in headers[header]:
            if re.fullmatch(r"(PROT|MAP)_[A-Z0-9_]+", name) and not name.startswith("MAP_HUGE_"):
                emit(name, value, comment, f"`{name}`")

    out += section("signals", ["For `kill`, `rt_sigaction` and the other signal calls."])
    for name, value, comment in headers["signal.h"]:
        if re.fullmatch(r"SIG[A-Z0-9]+", name):
            emit(name, value, comment, f"`{name}`")

    out += section("clocks", ["For `clock_gettime` and `clock_nanosleep`."])
    for name, value, comment in headers["time.h"]:
        if name.startswith("CLOCK_"):
            emit(name, value, comment, f"`{name}`")

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text("\n".join(out) + "\n")
    print(f"wrote {len(known)} constants to {OUTPUT.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
