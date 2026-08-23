//! Packs a `.em` file's [`EmittedValue`] stream into raw machine-code
//! bytes — the job `bitter` exists to do, and the one place in this crate
//! allowed to know what a `bits<N>`-shaped struct means.
//!
//! Deliberately architecture-blind, the same way `tests/riscv/packer.py`
//! (this module's Python prototype, now retired) was: it has no idea what
//! an `RType` or an `opcode` is, it just walks a struct's fields in
//! declaration order and concatenates whatever `bits<N>` leaves it finds,
//! most-significant field first — matching how every format struct in
//! `std/riscv/native.basm` documents its own field order.

use bitterasm::emit::{EmittedGenericArg, EmittedValue};
use num_bigint::BigInt;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Endian {
    Little,
    Big,
}

/// A value with a known bit width, produced by walking one `EmittedValue`
/// down to its `bits<N>` leaves and concatenating them.
struct Packed {
    value: BigInt,
    width_bits: usize,
}

pub fn pack_stream(values: &[EmittedValue], endian: Endian) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();

    for value in values {
        let packed = pack_value(value)?;

        if packed.width_bits % 8 != 0 {
            return Err(format!(
                "emitted value is {} bit(s) wide, not a whole number of bytes",
                packed.width_bits
            ));
        }

        bytes.extend(to_bytes(&packed.value, packed.width_bits / 8, endian));
    }

    Ok(bytes)
}

fn pack_value(value: &EmittedValue) -> Result<Packed, String> {
    match value {
        EmittedValue::Struct { name, args, fields } if name == "bits" => {
            let width_bits = bits_width(args)?;

            let (_, inner) = fields
                .iter()
                .find(|(field_name, _)| field_name == "value")
                .ok_or_else(|| "a `bits<N>` value is missing its `value` field".to_string())?;

            let raw = match inner {
                EmittedValue::Int { value } => value
                    .parse::<BigInt>()
                    .map_err(|error| format!("`{value}` isn't a valid integer: {error}"))?,

                other => {
                    return Err(format!(
                        "a `bits<N>`'s `value` field should be an Int, found {other:?}"
                    ))
                }
            };

            let mask = (BigInt::from(1) << width_bits) - BigInt::from(1);
            Ok(Packed { value: raw & mask, width_bits })
        }

        // Any other struct is walked one level deeper: its own fields,
        // concatenated in declaration order, the same way a `bits<N>`
        // struct's fields would be if this were the top-level call.
        EmittedValue::Struct { fields, .. } => {
            let mut value = BigInt::from(0);
            let mut width_bits = 0usize;

            for (_, field) in fields {
                let packed = pack_value(field)?;
                value = (value << packed.width_bits) | packed.value;
                width_bits += packed.width_bits;
            }

            Ok(Packed { value, width_bits })
        }

        EmittedValue::Int { value } => Err(format!(
            "can't tell how many bits a bare Int (`{value}`) should occupy — \
             wrap it in a `bits<N>` struct"
        )),
    }
}

fn bits_width(args: &[EmittedGenericArg]) -> Result<usize, String> {
    let width_arg = args
        .first()
        .ok_or_else(|| "a `bits<N>` struct is missing its width argument".to_string())?;

    let EmittedGenericArg::Const { value } = width_arg else {
        return Err("a `bits<N>` struct's width argument should be a const, found a type".to_string());
    };

    value
        .parse::<usize>()
        .map_err(|error| format!("`{value}` isn't a valid bit width: {error}"))
}

/// `width_bytes` is `value`'s exact byte width (already checked to be a
/// whole number of bytes by the caller) — `value` is non-negative here
/// (masked to width by `pack_value`), so this is just BigInt's minimal
/// big-endian encoding, left-padded with zero bytes to that width and
/// reversed for `Endian::Little`.
fn to_bytes(value: &BigInt, width_bytes: usize, endian: Endian) -> Vec<u8> {
    let (_, mut be_bytes) = value.to_bytes_be();

    while be_bytes.len() < width_bytes {
        be_bytes.insert(0, 0);
    }

    match endian {
        Endian::Big => be_bytes,
        Endian::Little => {
            be_bytes.reverse();
            be_bytes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(width: &str, value: &str) -> EmittedValue {
        EmittedValue::Struct {
            name: "bits".to_string(),
            args: vec![EmittedGenericArg::Const { value: width.to_string() }],
            fields: vec![("value".to_string(), EmittedValue::Int { value: value.to_string() })],
        }
    }

    fn r_type(funct7: &str, rs2: &str, rs1: &str, funct3: &str, rd: &str, opcode: &str) -> EmittedValue {
        EmittedValue::Struct {
            name: "RType".to_string(),
            args: vec![],
            fields: vec![
                ("funct7".to_string(), bits("7", funct7)),
                ("rs2".to_string(), bits("5", rs2)),
                ("rs1".to_string(), bits("5", rs1)),
                ("funct3".to_string(), bits("3", funct3)),
                ("rd".to_string(), bits("5", rd)),
                ("opcode".to_string(), bits("7", opcode)),
            ],
        }
    }

    #[test]
    fn packs_add_x1_x2_x3_little_endian() {
        // add x1, x2, x3 -> 0x003100b3 (funct7=0, rs2=3, rs1=2, funct3=0, rd=1, opcode=0b0110011=51)
        let value = r_type("0", "3", "2", "0", "1", "51");
        let bytes = pack_stream(&[value], Endian::Little).unwrap();
        assert_eq!(bytes, vec![0xb3, 0x00, 0x31, 0x00]);
    }

    #[test]
    fn packs_big_endian_as_the_reverse_of_little() {
        let value = r_type("0", "3", "2", "0", "1", "51");
        let little = pack_stream(&[value.clone()], Endian::Little).unwrap();
        let big = pack_stream(&[value], Endian::Big).unwrap();
        assert_eq!(big, little.into_iter().rev().collect::<Vec<u8>>());
    }

    #[test]
    fn masks_a_value_wider_than_its_declared_bit_width() {
        let value = bits("4", "255");
        let packed = pack_value(&value).unwrap();
        assert_eq!(packed.value, BigInt::from(0b1111));
        assert_eq!(packed.width_bits, 4);
    }

    #[test]
    fn rejects_a_non_byte_aligned_stream() {
        let value = bits("5", "1");
        let error = pack_stream(&[value], Endian::Little).unwrap_err();
        assert!(error.contains("not a whole number of bytes"), "{error}");
    }

    #[test]
    fn rejects_a_bare_int_with_no_declared_width() {
        let value = EmittedValue::Int { value: "3".to_string() };
        let error = pack_stream(&[value], Endian::Little).unwrap_err();
        assert!(error.contains("bare Int"), "{error}");
    }
}
