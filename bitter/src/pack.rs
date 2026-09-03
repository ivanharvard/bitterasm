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

use bitterasm::ast::{BinaryOp, Expr};
use bitterasm::emit::{EmittedGenericArg, EmittedValue};
use bitterasm::eval;
use bitterasm::token::Span;
use num_bigint::BigInt;
use std::collections::HashMap;

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

    for (here_index, value) in values.iter().enumerate() {
        let packed = pack_value(value, here_index)?;

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

// `here_index` is which top-level emitted entry (0-based, in emission
// order) is currently being packed — the same count `std.bitter.deferred`'s
// `here()` used to mean back when `@here` computed it eagerly at compile
// time (see `Deferred::Here`'s resolution below). It's threaded unchanged
// through every recursive call within one top-level value's packing, since
// a `Positioned<N>` can nest arbitrarily deep inside another struct's
// fields but always means "here" relative to the *top-level* instruction
// it was emitted as part of.
fn pack_value(value: &EmittedValue, here_index: usize) -> Result<Packed, String> {
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

        // `std.bitter.deferred`'s `Positioned<width>` — a `Deferred` value
        // (never a plain Int; that's the point of `here()`) paired with
        // the bit width it's meant to occupy. Resolved against this call's
        // `here_index`, then masked exactly like a `bits<N>` leaf.
        EmittedValue::Struct { name, args, fields } if name == "Positioned" => {
            let width_bits = bits_width(args)?;

            let (_, deferred) = fields
                .iter()
                .find(|(field_name, _)| field_name == "value")
                .ok_or_else(|| "a `Positioned<N>` value is missing its `value` field".to_string())?;

            let raw = resolve_deferred(deferred, here_index)?;

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
                let packed = pack_value(field, here_index)?;
                value = (value << packed.width_bits) | packed.value;
                width_bits += packed.width_bits;
            }

            Ok(Packed { value, width_bits })
        }

        EmittedValue::Int { value } => Err(format!(
            "can't tell how many bits a bare Int (`{value}`) should occupy — \
             wrap it in a `bits<N>` struct"
        )),

        EmittedValue::Enum { name, variant, .. } => Err(format!(
            "can't infer a machine-code layout for enum value `{name}.{variant}`"
        )),
    }
}

// Resolves a `std.bitter.deferred.Deferred` value (an `EmittedValue::Enum`
// named `Deferred`) into a concrete `BigInt`, using `here_index` for any
// `Here` marker found inside it. Reuses `bitterasm::eval::eval` for the
// actual arithmetic on `Sub`/`Mul`/`Shr`/`Band` nodes rather than
// re-deriving BigInt shift/mask semantics by hand — this guarantees
// identical behavior (negative-offset shifts included) to what the
// resolver used to compute eagerly for `@here`-based offsets.
fn resolve_deferred(deferred: &EmittedValue, here_index: usize) -> Result<BigInt, String> {
    let EmittedValue::Enum { name, variant, payload, .. } = deferred else {
        return Err(format!("expected a `Deferred` value, found {deferred:?}"));
    };
    if name != "Deferred" {
        return Err(format!("expected a `Deferred` value, found enum `{name}`"));
    }

    match variant.as_str() {
        "Leaf" => {
            let Some(payload) = payload else {
                return Err("`Deferred.Leaf` is missing its payload".to_string());
            };
            let EmittedValue::Int { value } = payload.as_ref() else {
                return Err(format!("`Deferred.Leaf`'s payload should be an Int, found {payload:?}"));
            };
            value.parse::<BigInt>().map_err(|error| format!("`{value}` isn't a valid integer: {error}"))
        }

        "Here" => Ok(BigInt::from(here_index)),

        "Node" => {
            let Some(payload) = payload else {
                return Err("`Deferred.Node` is missing its payload".to_string());
            };
            let EmittedValue::Struct { name, fields, .. } = payload.as_ref() else {
                return Err(format!("`Deferred.Node`'s payload should be a `BinOp` struct, found {payload:?}"));
            };
            if name != "BinOp" {
                return Err(format!("`Deferred.Node`'s payload should be a `BinOp` struct, found `{name}`"));
            }

            let field = |field_name: &str| {
                fields
                    .iter()
                    .find(|(n, _)| n == field_name)
                    .map(|(_, v)| v)
                    .ok_or_else(|| format!("a `BinOp` value is missing its `{field_name}` field"))
            };

            let EmittedValue::Enum { name: op_name, variant: op_variant, .. } = field("op")? else {
                return Err(format!("a `BinOp`'s `op` field should be an `Op` enum, found {:?}", field("op")?));
            };
            if op_name != "Op" {
                return Err(format!("a `BinOp`'s `op` field should be an `Op` enum, found `{op_name}`"));
            }

            let op = match op_variant.as_str() {
                "Sub" => BinaryOp::Subtract,
                "Mul" => BinaryOp::Multiply,
                "Shr" => BinaryOp::ShiftRight,
                "Band" => BinaryOp::BitAnd,
                other => return Err(format!("unknown `Op` variant `{other}`")),
            };

            let left = resolve_deferred(field("left")?, here_index)?;
            let right = resolve_deferred(field("right")?, here_index)?;

            let span = Span::new(0, 0);
            let expr = Expr::Binary {
                left: Box::new(Expr::Integer { raw: left.to_string(), span }),
                op,
                right: Box::new(Expr::Integer { raw: right.to_string(), span }),
                span,
            };

            eval::eval(&expr, &HashMap::new())
                .map_err(|error| format!("failed to resolve a `Deferred` expression: {error:?}"))
        }

        other => Err(format!("unknown `Deferred` variant `{other}`")),
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
        let packed = pack_value(&value, 0).unwrap();
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

    // --- std.bitter.deferred: Positioned<N> / Deferred resolution ---

    fn deferred_leaf(value: &str) -> EmittedValue {
        EmittedValue::Enum {
            name: "Deferred".to_string(),
            args: vec![],
            variant: "Leaf".to_string(),
            payload: Some(Box::new(EmittedValue::Int { value: value.to_string() })),
        }
    }

    fn deferred_here() -> EmittedValue {
        EmittedValue::Enum {
            name: "Deferred".to_string(),
            args: vec![],
            variant: "Here".to_string(),
            payload: None,
        }
    }

    fn deferred_node(op: &str, left: EmittedValue, right: EmittedValue) -> EmittedValue {
        EmittedValue::Enum {
            name: "Deferred".to_string(),
            args: vec![],
            variant: "Node".to_string(),
            payload: Some(Box::new(EmittedValue::Struct {
                name: "BinOp".to_string(),
                args: vec![],
                fields: vec![
                    (
                        "op".to_string(),
                        EmittedValue::Enum {
                            name: "Op".to_string(),
                            args: vec![],
                            variant: op.to_string(),
                            payload: None,
                        },
                    ),
                    ("left".to_string(), left),
                    ("right".to_string(), right),
                ],
            })),
        }
    }

    fn positioned(width: &str, value: EmittedValue) -> EmittedValue {
        EmittedValue::Struct {
            name: "Positioned".to_string(),
            args: vec![EmittedGenericArg::Const { value: width.to_string() }],
            fields: vec![("value".to_string(), value)],
        }
    }

    #[test]
    fn resolves_a_bare_here_marker_to_its_top_level_index() {
        let value = positioned("8", deferred_here());
        assert_eq!(pack_value(&value, 5).unwrap().value, BigInt::from(5));
    }

    #[test]
    fn resolves_a_bare_leaf_regardless_of_here_index() {
        let value = positioned("8", deferred_leaf("42"));
        assert_eq!(pack_value(&value, 999).unwrap().value, BigInt::from(42));
    }

    #[test]
    fn resolves_the_exact_tree_beqs_offset_computation_builds() {
        // mirrors `mul(sub(target, here()), 4)` for a branch whose label
        // target is instruction 3, packed as the 10th emitted instruction
        // (here_index = 9) -> (3 - 9) * 4 = -24.
        let offset = deferred_node(
            "Mul",
            deferred_node("Sub", deferred_leaf("3"), deferred_here()),
            deferred_leaf("4"),
        );
        // then `band(shr(offset, 1), 0b1111)` (BType's imm4_1 field; 0b1111 = 15).
        let imm4_1 = deferred_node(
            "Band",
            deferred_node("Shr", offset, deferred_leaf("1")),
            deferred_leaf("15"),
        );
        let value = positioned("4", imm4_1);

        // -24 >> 1 = -12; -12 & 0b1111 (two's-complement) = 0b0100, then
        // masked again to 4 bits by `pack_value` itself (a no-op here).
        assert_eq!(pack_value(&value, 9).unwrap().value, BigInt::from(0b0100));
    }

    #[test]
    fn masks_a_resolved_deferred_value_wider_than_its_declared_width() {
        let value = positioned("4", deferred_leaf("255"));
        let packed = pack_value(&value, 0).unwrap();
        assert_eq!(packed.value, BigInt::from(0b1111));
        assert_eq!(packed.width_bits, 4);
    }

    #[test]
    fn rejects_a_deferred_value_with_an_unknown_variant() {
        let bogus = EmittedValue::Enum {
            name: "Deferred".to_string(),
            args: vec![],
            variant: "Bogus".to_string(),
            payload: None,
        };
        let error = resolve_deferred(&bogus, 0).unwrap_err();
        assert!(error.contains("unknown"), "{error}");
    }
}
