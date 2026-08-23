//! Reads one hex-encoded 32-bit RISC-V instruction word per line on stdin
//! (e.g. `0x02a00293`) and prints one line of `mnemonic key=value ...` per
//! word to stdout, decoded via the `riscv-decode` crate — an existing,
//! independently-maintained decoder, not anything derived from
//! std/riscv/native.basm. Immediates are printed as signed, base-10
//! integers (matching what a `.s` test case's operand looks like), and
//! `lui`/`auipc`'s is the raw 20-bit value (`word >> 12`), not the
//! pre-shifted `word & 0xfffff000` `riscv_decode::types::UType::imm`
//! itself returns.
//!
//! This exists so tests/riscv/run_tests.py can cross-check bitterasm's
//! output against a second, genuinely external tool — not just against
//! reference.py, which this project also wrote.

use std::io::{self, BufRead};

use riscv_decode::types::{BType, IType, JType, RType, SType, ShiftType, UType};
use riscv_decode::Instruction;

fn sign_extend(value: u32, bits: u32) -> i64 {
    let shift = 32 - bits;
    (((value << shift) as i32) >> shift) as i64
}

fn r(mnemonic: &str, i: RType) {
    println!("{mnemonic} rd={} rs1={} rs2={}", i.rd(), i.rs1(), i.rs2());
}

fn i_arith(mnemonic: &str, i: IType) {
    println!("{mnemonic} rd={} rs1={} imm={}", i.rd(), i.rs1(), sign_extend(i.imm(), 12));
}

fn i_shift(mnemonic: &str, i: ShiftType) {
    println!("{mnemonic} rd={} rs1={} shamt={}", i.rd(), i.rs1(), i.shamt());
}

fn load(mnemonic: &str, i: IType) {
    println!("{mnemonic} rd={} rs1={} imm={}", i.rd(), i.rs1(), sign_extend(i.imm(), 12));
}

fn store(mnemonic: &str, i: SType) {
    println!("{mnemonic} rs1={} rs2={} imm={}", i.rs1(), i.rs2(), sign_extend(i.imm(), 12));
}

fn branch(mnemonic: &str, i: BType) {
    println!("{mnemonic} rs1={} rs2={} imm={}", i.rs1(), i.rs2(), sign_extend(i.imm(), 13));
}

fn upper(mnemonic: &str, i: UType) {
    println!("{mnemonic} rd={} imm={}", i.rd(), i.imm() >> 12);
}

fn jump(mnemonic: &str, i: JType) {
    println!("{mnemonic} rd={} imm={}", i.rd(), sign_extend(i.imm(), 21));
}

fn jump_reg(mnemonic: &str, i: IType) {
    println!("{mnemonic} rd={} rs1={} imm={}", i.rd(), i.rs1(), sign_extend(i.imm(), 12));
}

fn decode_and_print(word: u32) {
    match riscv_decode::decode(word) {
        Ok(Instruction::Add(i)) => r("add", i),
        Ok(Instruction::Sub(i)) => r("sub", i),
        Ok(Instruction::Sll(i)) => r("sll", i),
        Ok(Instruction::Slt(i)) => r("slt", i),
        Ok(Instruction::Sltu(i)) => r("sltu", i),
        Ok(Instruction::Xor(i)) => r("xor", i),
        Ok(Instruction::Srl(i)) => r("srl", i),
        Ok(Instruction::Sra(i)) => r("sra", i),
        Ok(Instruction::Or(i)) => r("or", i),
        Ok(Instruction::And(i)) => r("and", i),

        Ok(Instruction::Addi(i)) => i_arith("addi", i),
        Ok(Instruction::Slti(i)) => i_arith("slti", i),
        Ok(Instruction::Sltiu(i)) => i_arith("sltiu", i),
        Ok(Instruction::Xori(i)) => i_arith("xori", i),
        Ok(Instruction::Ori(i)) => i_arith("ori", i),
        Ok(Instruction::Andi(i)) => i_arith("andi", i),

        Ok(Instruction::Slli(i)) => i_shift("slli", i),
        Ok(Instruction::Srli(i)) => i_shift("srli", i),
        Ok(Instruction::Srai(i)) => i_shift("srai", i),

        Ok(Instruction::Lb(i)) => load("lb", i),
        Ok(Instruction::Lh(i)) => load("lh", i),
        Ok(Instruction::Lw(i)) => load("lw", i),
        Ok(Instruction::Lbu(i)) => load("lbu", i),
        Ok(Instruction::Lhu(i)) => load("lhu", i),

        Ok(Instruction::Sb(i)) => store("sb", i),
        Ok(Instruction::Sh(i)) => store("sh", i),
        Ok(Instruction::Sw(i)) => store("sw", i),

        Ok(Instruction::Beq(i)) => branch("beq", i),
        Ok(Instruction::Bne(i)) => branch("bne", i),
        Ok(Instruction::Blt(i)) => branch("blt", i),
        Ok(Instruction::Bge(i)) => branch("bge", i),
        Ok(Instruction::Bltu(i)) => branch("bltu", i),
        Ok(Instruction::Bgeu(i)) => branch("bgeu", i),

        Ok(Instruction::Lui(i)) => upper("lui", i),
        Ok(Instruction::Auipc(i)) => upper("auipc", i),

        Ok(Instruction::Jal(i)) => jump("jal", i),
        Ok(Instruction::Jalr(i)) => jump_reg("jalr", i),

        Ok(Instruction::Ecall) => println!("ecall"),
        Ok(Instruction::Ebreak) => println!("ebreak"),

        Ok(other) => println!("unsupported {other:?}"),
        Err(error) => println!("decode-error {error:?}"),
    }
}

fn main() {
    for line in io::stdin().lock().lines() {
        let line = line.expect("failed to read stdin");
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        let word = u32::from_str_radix(without_prefix, 16)
            .unwrap_or_else(|error| panic!("invalid hex word {trimmed:?}: {error}"));

        decode_and_print(word);
    }
}
