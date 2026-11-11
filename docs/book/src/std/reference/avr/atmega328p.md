# `atmega328p`

[`std`](../index.md) › [`avr`](index.md) › `atmega328p`

The ATmega328P, the chip on an Arduino Uno: its I/O registers, memory
sizes and interrupt vectors, named the way its datasheet and avr-libc
name them.

```basm
from std.avr.native import *
from std.avr.atmega328p import *

sbi DDRB, 5
sbi PORTB, 5
```

```bytes
25 9a
2d 9a
```

Registers below 0x40 are in I/O space, for `in_`, `out`, and, below
0x20, `sbi`, `cbi`, `sbic` and `sbis`. The ones above are only in data
memory, for `lds` and `sts`. An I/O register is also in data memory, at
its I/O address plus `IO_OFFSET`.

## Constants

| Constant | Type | Value | Description |
|---|---|---|---|
| `FLASHEND` |  | `0x7FFF` | The last byte address of flash. Flash holds 16 KiB words. |
| `RAMSTART` |  | `0x0100` | The first byte of SRAM, after the registers and I/O. |
| `RAMEND` |  | `0x08FF` | The last byte of SRAM, where the stack starts. |
| `E2END` |  | `0x03FF` | The last byte address of EEPROM. |
| `IO_OFFSET` |  | `0x20` | Where I/O space starts in data memory: I/O address `a` is data address `a + IO_OFFSET`. |
| `PINB` |  | `0x03` | Port B input pins. Writing a 1 to a bit toggles that pin's output. |
| `DDRB` |  | `0x04` | Port B direction: 1 for output. |
| `PORTB` |  | `0x05` | Port B output. |
| `PINC` |  | `0x06` | Port C input pins. |
| `DDRC` |  | `0x07` | Port C direction. |
| `PORTC` |  | `0x08` | Port C output. |
| `PIND` |  | `0x09` | Port D input pins. |
| `DDRD` |  | `0x0A` | Port D direction. |
| `PORTD` |  | `0x0B` | Port D output. |
| `TIFR0` |  | `0x15` | Timer 0 interrupt flags. |
| `TIFR1` |  | `0x16` | Timer 1 interrupt flags. |
| `TIFR2` |  | `0x17` | Timer 2 interrupt flags. |
| `PCIFR` |  | `0x1B` | Pin change interrupt flags. |
| `EIFR` |  | `0x1C` | External interrupt flags. |
| `EIMSK` |  | `0x1D` | External interrupt mask. |
| `GPIOR0` |  | `0x1E` | General purpose I/O register 0. |
| `EECR` |  | `0x1F` | EEPROM control. |
| `EEDR` |  | `0x20` | EEPROM data. |
| `EEARL` |  | `0x21` | EEPROM address, low byte. |
| `EEARH` |  | `0x22` | EEPROM address, high byte. |
| `GTCCR` |  | `0x23` | General timer control. |
| `TCCR0A` |  | `0x24` | Timer 0 control A. |
| `TCCR0B` |  | `0x25` | Timer 0 control B. |
| `TCNT0` |  | `0x26` | Timer 0 counter. |
| `OCR0A` |  | `0x27` | Timer 0 output compare A. |
| `OCR0B` |  | `0x28` | Timer 0 output compare B. |
| `GPIOR1` |  | `0x2A` | General purpose I/O register 1. |
| `GPIOR2` |  | `0x2B` | General purpose I/O register 2. |
| `SPCR` |  | `0x2C` | SPI control. |
| `SPSR` |  | `0x2D` | SPI status. |
| `SPDR` |  | `0x2E` | SPI data. |
| `ACSR` |  | `0x30` | Analog comparator control and status. |
| `SMCR` |  | `0x33` | Sleep mode control. |
| `MCUSR` |  | `0x34` | MCU status: what caused the last reset. |
| `MCUCR` |  | `0x35` | MCU control. |
| `SPMCSR` |  | `0x37` | Self-programming control and status. |
| `SPL` |  | `0x3D` | Stack pointer, low byte. |
| `SPH` |  | `0x3E` | Stack pointer, high byte. |
| `SREG` |  | `0x3F` | The status register. |
| `WDTCSR` |  | `0x60` | Watchdog timer control. |
| `CLKPR` |  | `0x61` | Clock prescaler. |
| `PRR` |  | `0x64` | Power reduction. |
| `TIMSK0` |  | `0x6E` | Timer 0 interrupt mask. |
| `TIMSK1` |  | `0x6F` | Timer 1 interrupt mask. |
| `TIMSK2` |  | `0x70` | Timer 2 interrupt mask. |
| `ADCL` |  | `0x78` | ADC result, low byte. |
| `ADCH` |  | `0x79` | ADC result, high byte. |
| `ADCSRA` |  | `0x7A` | ADC control and status A. |
| `ADMUX` |  | `0x7C` | ADC input and reference. |
| `UCSR0A` |  | `0xC0` | USART 0 control and status A. |
| `UCSR0B` |  | `0xC1` | USART 0 control and status B. |
| `UCSR0C` |  | `0xC2` | USART 0 control and status C. |
| `UBRR0L` |  | `0xC4` | USART 0 baud rate, low byte. |
| `UBRR0H` |  | `0xC5` | USART 0 baud rate, high byte. |
| `UDR0` |  | `0xC6` | USART 0 data. |
| `VECTORS` |  | `26` | How many vectors there are. |
| `RESET_VECTOR` |  | `0` | Reset. |
| `INT0_VECTOR` |  | `1` | External interrupt 0. |
| `INT1_VECTOR` |  | `2` | External interrupt 1. |
| `PCINT0_VECTOR` |  | `3` | Pin change interrupt 0. |
| `PCINT1_VECTOR` |  | `4` | Pin change interrupt 1. |
| `PCINT2_VECTOR` |  | `5` | Pin change interrupt 2. |
| `WDT_VECTOR` |  | `6` | Watchdog timeout. |
| `TIMER2_COMPA_VECTOR` |  | `7` | Timer 2 compare match A. |
| `TIMER2_COMPB_VECTOR` |  | `8` | Timer 2 compare match B. |
| `TIMER2_OVF_VECTOR` |  | `9` | Timer 2 overflow. |
| `TIMER1_CAPT_VECTOR` |  | `10` | Timer 1 capture. |
| `TIMER1_COMPA_VECTOR` |  | `11` | Timer 1 compare match A. |
| `TIMER1_COMPB_VECTOR` |  | `12` | Timer 1 compare match B. |
| `TIMER1_OVF_VECTOR` |  | `13` | Timer 1 overflow. |
| `TIMER0_COMPA_VECTOR` |  | `14` | Timer 0 compare match A. |
| `TIMER0_COMPB_VECTOR` |  | `15` | Timer 0 compare match B. |
| `TIMER0_OVF_VECTOR` |  | `16` | Timer 0 overflow. |
| `SPI_STC_VECTOR` |  | `17` | SPI transfer complete. |
| `USART_RX_VECTOR` |  | `18` | USART receive complete. |
| `USART_UDRE_VECTOR` |  | `19` | USART data register empty. |
| `USART_TX_VECTOR` |  | `20` | USART transmit complete. |
| `ADC_VECTOR` |  | `21` | ADC conversion complete. |
| `EE_READY_VECTOR` |  | `22` | EEPROM ready. |
| `ANALOG_COMP_VECTOR` |  | `23` | Analog comparator. |
| `TWI_VECTOR` |  | `24` | Two-wire serial interface. |
| `SPM_READY_VECTOR` |  | `25` | Self-programming ready. |
