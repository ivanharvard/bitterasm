// `std/strings/`: its walkers are loops, so string length is limited only
// by `@for`'s bounds.

mod common;

use bitterasm::emit::EmittedValue;
use common::{compile_entries, compile_error};

// Every `bits<N>` leaf in `value`, in order, as bytes.
fn bytes_of(value: &EmittedValue, out: &mut Vec<u8>) {
    match value {
        EmittedValue::Struct { id, fields, .. } if id == "std.binary.bits" => {
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

#[test]
fn utf16_and_c_strings_match_python() {
    let expected = [
        "29619741131678333868357109381979919840581111034145555461158338593", // "Hi, wörld €𝄞!" as UTF-16
        "14",           // its length in code units
        "14",           // utf16_struct_unit_len
        "1",            // validate_utf16
        "13",           // utf16_codepoint_count
        "72",           // unit 0, 'H'
        "55348",        // unit 11, 0xD834: 𝄞's high surrogate
        "56606",        // unit 12, 0xDD1E: its low surrogate
        "4718697",      // "Hi" ASCII to UTF-16, big-endian
        "6881352",      // and little-endian
        "4745472",      // nul_terminated ASCII "Hi": 0x486900
        "3",            // its length
        "18537",        // nul_terminated little-endian UTF-8: the value is unchanged
        "3",            // its length
        "309244526592", // nul_terminated UTF-16 "Hi": 0x004800690000
        "6881352",      // nul_terminated little-endian UTF-16: unchanged
        "1",            // nul_terminated "" is just the NUL
    ];

    let actual: Vec<String> = compile_entries("tests/fixtures/strings/utf16_cstring.basm")
        .into_iter()
        .map(|entry| match entry.value {
            EmittedValue::Int { value } => value,
            other => panic!("expected a bare Int, found {other:?}"),
        })
        .collect();

    assert_eq!(actual, expected);
}

#[test]
fn invalid_utf16_and_c_strings_are_rejected() {
    for (fixture, message) in [
        ("err_utf16_lone_high", "UTF-16 string ends in a high surrogate"),
        ("err_utf16_lone_low", "UTF-16 low surrogate without a high surrogate before it"),
        ("err_utf16_unpaired_high", "UTF-16 high surrogate without a low surrogate after it"),
        ("err_cstring_nul", "a C string cannot contain NUL"),
    ] {
        let stderr = compile_error(&format!("tests/fixtures/strings/{fixture}.basm"));
        assert!(stderr.contains(message), "{fixture}: expected {message:?} in:\n{stderr}");
    }
}
