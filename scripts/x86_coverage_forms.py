"""Derive x86-64 general-purpose operand forms from asmjit's isa_x86.json.

This is how `tests/coverage/x86_64.txt` was made. A form is a mnemonic plus
the classes of its written operands (reg, mem, imm, rel, cl, dx, sreg).
Operand sizes are folded together, implicit operands are dropped, and an
r/m operand becomes two forms, one for each. Each form gets one NASM-syntax
sample, preferring 64-bit operands; every sample should assemble with
`nasm -f bin` under `bits 64`.

Usage:
    curl -O https://raw.githubusercontent.com/asmjit/asmjit/master/db/isa_x86.json
    python3 -I scripts/x86_coverage_forms.py isa_x86.json > forms.tsv

Each output line is "<form>\t<sample>". In the coverage list, each becomes
"  | <form> | <sample>" under the list's header; then run `make coverage`
to fill in which are covered.
"""

import json
import re
import sys

# CPUID features every x86-64 CPU has, plus the 486 additions asmjit tags.
BASELINE_EXTS = {None, "CMOV", "I486", "CMPXCHG8B", "CMPXCHG16B", "LAHFSAHF"}

REG_NAMES = [
    {64: "rax", 32: "eax", 16: "ax", 8: "al"},
    {64: "rbx", 32: "ebx", 16: "bx", 8: "bl"},
    {64: "rcx", 32: "ecx", 16: "cx", 8: "cl"},
]
MEM_SIZE = {8: "byte", 16: "word", 32: "dword", 64: "qword", 128: "oword"}

# asmjit names that NASM spells differently, or that are another form of a
# mnemonic we already count.
RENAME = {"movabs": "mov", "lcall": "call far", "ljmp": "jmp far", "xlatb": "xlatb"}

# System instructions (Intel SDM Vol. 1, 5.20) and the privileged ones from
# its 64-bit list (5.21): not part of what a user program runs. `syscall` is
# in 5.21 too, and stays.
SYSTEM = {
    "clts", "hlt", "invd", "invlpg", "invpcid", "lar", "lgdt", "lidt", "lldt",
    "lmsw", "lsl", "ltr", "rdmsrlist", "wrmsrlist", "rdpmc", "sgdt", "sidt",
    "sldt", "smsw", "str", "swapgs", "sysenter", "sysexit", "sysexitq",
    "sysret", "sysretq", "verr", "verw", "wbinvd",
}

# String instructions: the operands are implicit, NASM writes a size suffix.
STRING_OPS = {"movs", "cmps", "scas", "lods", "stos", "ins", "outs"}


def size_of(spec):
    """Operand size in bits for a register/memory spec, largest choice."""
    if spec in ("rv", "mv", "ry", "my", "axv"):
        return 64
    named = {"al": 8, "ax": 16, "eax": 32, "rax": 64}
    if spec in named:
        return named[spec]
    m = re.search(r"(\d+)", spec)
    return int(m.group(1)) if m else 64


def classify(op):
    """[(class, size)] for one written operand; two entries for r/m."""
    op = re.sub(r"^[A-Za-z?]+:", "", op.strip()).lstrip("~")
    if op.startswith("<") or op.startswith("dfv"):
        return None  # implicit
    if "/" in op and op.split("/")[0].startswith("r"):
        reg, mem = op.split("/", 1)
        return [("reg", size_of(reg)), ("mem", size_of(mem))]
    if op in ("cl",):
        return [("cl", 8)]
    if op in ("dx",):
        return [("dx", 16)]
    if op in ("al", "ax", "eax", "rax", "axv"):
        return [("reg", size_of(op) if op != "axv" else 64)]
    if op in ("sreg", "ds", "es", "fs", "gs", "ss", "cs"):
        return [("sreg", 16)]
    if op.startswith("rel"):
        return [("rel", 32)]
    if op.startswith("imm") or op == "1":
        return [("imm", size_of(op) if op != "1" else 8)]
    if re.match(r"^r(8|16|32|64|v|y)(@8)?$", op):
        return [("reg", size_of(op))]
    if op.startswith("m"):
        size = None if op in ("m",) or "_" in op else size_of(op)
        return [("mem", size)]
    raise ValueError(f"unknown operand {op!r}")


def expand(ops):
    """Every combination of classes, splitting r/m operands."""
    combos = [[]]
    for op in ops:
        choices = classify(op)
        if choices is None:
            continue
        combos = [c + [choice] for c in combos for choice in choices]
    return combos


def sample(name, combo):
    regs = iter(REG_NAMES)
    reg_sizes = [size for cls, size in combo if cls == "reg"]
    parts = []
    for cls, size in combo:
        if cls == "reg":
            parts.append(next(regs)[size])
        elif cls == "mem":
            # NASM needs a size only when no register already fixes it.
            needs_size = not reg_sizes or size != reg_sizes[0]
            sized = MEM_SIZE.get(size) if needs_size else None
            parts.append(f"{sized} [rbx]" if sized else "[rbx]")
        elif cls == "imm":
            parts.append("1")
        elif cls == "rel":
            parts.append("target")
        elif cls == "cl":
            parts.append("cl")
        elif cls == "dx":
            parts.append("dx")
        elif cls == "sreg":
            parts.append("fs")
    return f"{name} {', '.join(parts)}" if parts else name


def main():
    db = json.load(open(sys.argv[1]))
    forms = {}
    for group in db["instructions"]:
        if group["category"] != "GP":
            continue
        for inst in group["instructions"]:
            if inst.get("alt") or inst.get("cpl") == 0:
                continue
            if not ("any" in inst or "x64" in inst):
                continue  # 32-bit only, or APX only
            if inst.get("ext") not in BASELINE_EXTS:
                continue
            text = re.sub(r"^(\[[^\]]*\]\s*)+", "", inst["inst"])
            name, _, rest = text.partition(" ")
            if "{" in name:
                continue  # APX {nf} variants
            if name in SYSTEM:
                continue
            name = RENAME.get(name, name)
            ops = [op for op in rest.split(",") if op.strip()] if rest else []
            if name == "pushw":
                continue  # the 16-bit size of `push imm`, already counted
            if name == "nop":
                ops = ops[:1]  # the hint register isn't written in NASM

            if name in STRING_OPS:
                forms.setdefault((name, ()), f"{name}q" if name not in ("ins", "outs") else f"{name}d")
                continue

            for combo in expand(ops):
                key = (name, tuple(cls for cls, _ in combo))
                # Prefer the sample with the widest operands.
                width = sum(size or 0 for _, size in combo)
                current = forms.get(key)
                if current is None or width > current[0]:
                    forms[key] = (width, sample(name, combo))

    for (name, classes), value in sorted(forms.items()):
        text = value if isinstance(value, str) else value[1]
        form = f"{name} {', '.join(classes)}" if classes else name
        print(f"{form}\t{text}")


main()
