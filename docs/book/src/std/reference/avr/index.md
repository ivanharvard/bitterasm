# `avr`

[`std`](../index.md) › `avr`

| Name | Summary |
|---|---|
| [`atmega328p`](atmega328p.md) | The ATmega328P, the chip on an Arduino Uno: its I/O registers, memory sizes and interrupt vectors, named the way its datasheet and avr-libc name them. |
| [`impl`](impl.md) | AVR: the 8-bit microcontrollers of the ATmega and ATtiny families. Their 32 registers, and every instruction of the ATmega328P (the `avr5` family), each emitted as one or two little-endian 16-bit words. |
| [`native`](native.md) | AVR in its standard assembly syntax, the one GNU `as` and Atmel's assembler read: `ld r16, X+`, `st -Y, r0`, `ldd r16, Z+5`, `in r16, 0x3f`. |
