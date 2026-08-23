"""An independent RV32I encoder, written directly from the ISA spec rather
than derived from std/riscv/native.basm — the whole point of this file is
to *not* share logic (or bugs) with the BitterASM side, so a matching
result between the two actually means something.

Only understands the plain instruction-line syntax the test cases in
cases/*.s use: one mnemonic and its operands per line, comments starting
with '#', no labels, no directives, no pseudo-instructions. Branch/jump
"offset" operands are raw signed byte immediates, not symbolic labels —
std/riscv/native.basm's macros don't do label resolution, so the test
corpus doesn't lean on it either.
"""

from __future__ import annotations

import re

REGISTERS = {f"x{i}": i for i in range(32)}
REGISTERS.update({
    "zero": 0, "ra": 1, "sp": 2, "gp": 3, "tp": 4,
    "t0": 5, "t1": 6, "t2": 7,
    "s0": 8, "fp": 8, "s1": 9,
    "a0": 10, "a1": 11, "a2": 12, "a3": 13, "a4": 14, "a5": 15, "a6": 16, "a7": 17,
    "s2": 18, "s3": 19, "s4": 20, "s5": 21, "s6": 22, "s7": 23,
    "s8": 24, "s9": 25, "s10": 26, "s11": 27,
    "t3": 28, "t4": 29, "t5": 30, "t6": 31,
})


def _reg(token: str) -> int:
    token = token.strip()
    if token not in REGISTERS:
        raise ValueError(f"unknown register {token!r}")
    return REGISTERS[token]


def _imm(token: str) -> int:
    return int(token.strip(), 0)


def _mask(value: int, width: int) -> int:
    return value & ((1 << width) - 1)


def _r_type(opcode: int, funct3: int, funct7: int, rd: int, rs1: int, rs2: int) -> int:
    return (
        (_mask(funct7, 7) << 25)
        | (_mask(rs2, 5) << 20)
        | (_mask(rs1, 5) << 15)
        | (_mask(funct3, 3) << 12)
        | (_mask(rd, 5) << 7)
        | _mask(opcode, 7)
    )


def _i_type(opcode: int, funct3: int, rd: int, rs1: int, imm: int) -> int:
    return (
        (_mask(imm, 12) << 20)
        | (_mask(rs1, 5) << 15)
        | (_mask(funct3, 3) << 12)
        | (_mask(rd, 5) << 7)
        | _mask(opcode, 7)
    )


def _s_type(opcode: int, funct3: int, rs1: int, rs2: int, imm: int) -> int:
    imm = _mask(imm, 12)
    return (
        ((imm >> 5) << 25)
        | (_mask(rs2, 5) << 20)
        | (_mask(rs1, 5) << 15)
        | (_mask(funct3, 3) << 12)
        | ((imm & 0b11111) << 7)
        | _mask(opcode, 7)
    )


def _b_type(opcode: int, funct3: int, rs1: int, rs2: int, offset: int) -> int:
    imm = _mask(offset, 13)
    return (
        (((imm >> 12) & 1) << 31)
        | (((imm >> 5) & 0b111111) << 25)
        | (_mask(rs2, 5) << 20)
        | (_mask(rs1, 5) << 15)
        | (_mask(funct3, 3) << 12)
        | (((imm >> 1) & 0b1111) << 8)
        | (((imm >> 11) & 1) << 7)
        | _mask(opcode, 7)
    )


def _u_type(opcode: int, rd: int, imm: int) -> int:
    return (_mask(imm, 20) << 12) | (_mask(rd, 5) << 7) | _mask(opcode, 7)


def _j_type(opcode: int, rd: int, offset: int) -> int:
    imm = _mask(offset, 21)
    return (
        (((imm >> 20) & 1) << 31)
        | (((imm >> 1) & 0b1111111111) << 21)
        | (((imm >> 11) & 1) << 20)
        | (((imm >> 12) & 0b11111111) << 12)
        | (_mask(rd, 5) << 7)
        | _mask(opcode, 7)
    )


_R = {
    "add": (0b0110011, 0b000, 0b0000000),
    "sub": (0b0110011, 0b000, 0b0100000),
    "sll": (0b0110011, 0b001, 0b0000000),
    "slt": (0b0110011, 0b010, 0b0000000),
    "sltu": (0b0110011, 0b011, 0b0000000),
    "xor": (0b0110011, 0b100, 0b0000000),
    "srl": (0b0110011, 0b101, 0b0000000),
    "sra": (0b0110011, 0b101, 0b0100000),
    "or": (0b0110011, 0b110, 0b0000000),
    "and": (0b0110011, 0b111, 0b0000000),
}

_I_ARITH = {
    "addi": (0b0010011, 0b000),
    "slti": (0b0010011, 0b010),
    "sltiu": (0b0010011, 0b011),
    "xori": (0b0010011, 0b100),
    "ori": (0b0010011, 0b110),
    "andi": (0b0010011, 0b111),
}

_I_SHIFT = {
    "slli": (0b0010011, 0b001, 0b0000000),
    "srli": (0b0010011, 0b101, 0b0000000),
    "srai": (0b0010011, 0b101, 0b0100000),
}

_LOADS = {
    "lb": (0b0000011, 0b000),
    "lh": (0b0000011, 0b001),
    "lw": (0b0000011, 0b010),
    "lbu": (0b0000011, 0b100),
    "lhu": (0b0000011, 0b101),
}

_STORES = {
    "sb": (0b0100011, 0b000),
    "sh": (0b0100011, 0b001),
    "sw": (0b0100011, 0b010),
}

_BRANCHES = {
    "beq": (0b1100011, 0b000),
    "bne": (0b1100011, 0b001),
    "blt": (0b1100011, 0b100),
    "bge": (0b1100011, 0b101),
    "bltu": (0b1100011, 0b110),
    "bgeu": (0b1100011, 0b111),
}

_OFFSET_PAREN = re.compile(r"^\s*(-?\w+)\s*\(\s*(\w+)\s*\)\s*$")


def encode(mnemonic: str, operands: list[str]) -> int:
    mnemonic = mnemonic.lower()

    if mnemonic in _R:
        opcode, funct3, funct7 = _R[mnemonic]
        rd, rs1, rs2 = (_reg(o) for o in operands)
        return _r_type(opcode, funct3, funct7, rd, rs1, rs2)

    if mnemonic in _I_ARITH:
        opcode, funct3 = _I_ARITH[mnemonic]
        rd, rs1, imm = operands
        return _i_type(opcode, funct3, _reg(rd), _reg(rs1), _imm(imm))

    if mnemonic in _I_SHIFT:
        opcode, funct3, funct7 = _I_SHIFT[mnemonic]
        rd, rs1, shamt = operands
        imm = (funct7 << 5) | _mask(_imm(shamt), 5)
        return _i_type(opcode, funct3, _reg(rd), _reg(rs1), imm)

    if mnemonic in _LOADS:
        opcode, funct3 = _LOADS[mnemonic]
        rd, offset_base = operands
        offset, base = _split_offset(offset_base)
        return _i_type(opcode, funct3, _reg(rd), _reg(base), offset)

    if mnemonic in _STORES:
        opcode, funct3 = _STORES[mnemonic]
        rs2, offset_base = operands
        offset, base = _split_offset(offset_base)
        return _s_type(opcode, funct3, _reg(base), _reg(rs2), offset)

    if mnemonic in _BRANCHES:
        opcode, funct3 = _BRANCHES[mnemonic]
        rs1, rs2, offset = operands
        return _b_type(opcode, funct3, _reg(rs1), _reg(rs2), _imm(offset))

    if mnemonic == "lui":
        rd, imm = operands
        return _u_type(0b0110111, _reg(rd), _imm(imm))

    if mnemonic == "auipc":
        rd, imm = operands
        return _u_type(0b0010111, _reg(rd), _imm(imm))

    if mnemonic == "jal":
        rd, offset = operands
        return _j_type(0b1101111, _reg(rd), _imm(offset))

    if mnemonic == "jalr":
        rd, rs1, offset = operands
        return _i_type(0b1100111, 0b000, _reg(rd), _reg(rs1), _imm(offset))

    if mnemonic == "ecall":
        return _i_type(0b1110011, 0b000, 0, 0, 0)

    if mnemonic == "ebreak":
        return _i_type(0b1110011, 0b000, 0, 0, 1)

    raise ValueError(f"unknown mnemonic {mnemonic!r}")


def _split_offset(token: str) -> tuple[int, str]:
    match = _OFFSET_PAREN.match(token)
    if not match:
        raise ValueError(f"expected offset(reg), found {token!r}")
    return _imm(match.group(1)), match.group(2)


def parse_line(line: str) -> tuple[str, list[str]] | None:
    line = line.split("#", 1)[0].strip()
    if not line:
        return None

    mnemonic, _, rest = line.partition(" ")
    operands = [operand.strip() for operand in rest.split(",")] if rest.strip() else []
    return mnemonic.strip(), operands


def encode_file(path: str) -> list[int]:
    words = []

    with open(path) as handle:
        for line in handle:
            parsed = parse_line(line)
            if parsed is None:
                continue

            mnemonic, operands = parsed
            words.append(encode(mnemonic, operands))

    return words
