# `strings`

[`std`](../index.md) › `strings`

| Name | Summary |
|---|---|
| [`ascii`](ascii.md) | ASCII strings: one byte per character, each below `0x80`. |
| [`cstring`](cstring.md) | NUL-terminated strings, the way C (and every API built on it) expects them: the characters, then one zero code unit. `nul_terminated` works on any encoding and returns the same string type, one unit longer. A zero unit is one byte for ASCII and UTF-8, and two bytes for UTF-16, as in Windows' wide strings. |
| [`packed`](packed.md) | The shared base of `std.strings`: strings packed into one integer. A string literal like `"hi"` is a struct of code points. Each encoding packs it as a run of fixed-width code units in a single `int`: |
| [`utf16`](utf16.md) | UTF-16 strings: code points as 16-bit code units, the encoding Windows APIs and Java use. A character past U+FFFF takes two units, a surrogate pair. |
| [`utf8`](utf8.md) | UTF-8 strings, and `string_from_struct`, which encodes a string literal as UTF-8 bytes packed into one `int`. |
