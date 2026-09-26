// `std/string.basm` (Part C of `docs/1.0/PROGRESS.md`): its walkers are
// loops now, so string length is limited only by `@for`'s bounds.

mod common;

use bitterasm::emit::EmittedValue;
use common::compile_entries;

// Every `bits<N>` leaf in `value`, in order, as bytes.
fn bytes_of(value: &EmittedValue, out: &mut Vec<u8>) {
    match value {
        EmittedValue::Struct { name, fields, .. } if name == "bits" => {
            let (_, EmittedValue::Int { value }) = &fields[0] else { panic!("bits holds an Int") };
            out.push(value.parse().expect("a byte"));
        }
        EmittedValue::Struct { fields, .. } => fields.iter().for_each(|(_, field)| bytes_of(field, out)),
        other => panic!("unexpected value in a data directive: {other:?}"),
    }
}

#[test]
fn long_strings_encode_to_their_utf8_bytes() {
    let mut bytes = Vec::new();
    for entry in compile_entries("tests/fixtures/strings/long.basm") {
        bytes_of(&entry.value, &mut bytes);
    }

    let ascii: String = (0..1000).map(|i| (b'a' + (i % 26) as u8) as char).collect();
    let utf8 = "héllo wörld € 𝄞 ".repeat(40);
    let mut expected = ascii.into_bytes();
    expected.extend(utf8.as_bytes());

    assert_eq!(bytes.len(), 1920);
    assert_eq!(bytes, expected);
}

#[test]
fn every_public_macro_matches_the_previous_implementation() {
    // Decimal strings: the packed string doesn't fit in an `i128`.
    let expected = [
        "120",                                            // 'x'
        "1614815459898332929474429657537854944603053601", // "Hi, wörld €𝄞!" packed
        "19",                                             // its byte length
        "19",                                             // utf8_struct_byte_len
        "1",                                              // validate_utf8
        "13",                                             // utf8_codepoint_count
        "0",                                              // utf8_is_ascii
        "1",                                              // validate_ascii
        "6123267631887428960763487728690",                // ascii_upper
        "8668511189990430778798632875058",                // ascii_lower
        "310939249775",                                   // ascii_title("hello")
        "6123267631887428960763487728690",                // ascii_upper, little-endian
        "1",                                              // utf8_is_ascii
        "13",                                             // utf8_codepoint_count
    ];

    let actual: Vec<String> = compile_entries("tests/fixtures/strings/api.basm")
        .into_iter()
        .map(|entry| match entry.value {
            EmittedValue::Int { value } => value,
            other => panic!("expected a bare Int, found {other:?}"),
        })
        .collect();

    assert_eq!(actual, expected);
}
