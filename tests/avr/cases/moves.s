# Moves, loads and stores through every pointer form, flash loads, and
# I/O.
mov r0, r31
movw r24, r30
movw r0, r2
ldi r16, 0
ldi r31, 0xff
ldi r30, lo8(table)
ldi r31, hi8(table)
ld r0, X
ld r1, X+
ld r2, -X
ld r3, Y
ld r4, Y+
ld r5, -Y
ld r6, Z
ld r7, Z+
ld r8, -Z
ldd r9, Y+1
ldd r10, Z+63
ldd r11, Y+32
st X, r12
st X+, r13
st -X, r14
st Y, r15
st Y+, r16
st -Y, r17
st Z, r18
st Z+, r19
st -Z, r20
std Y+5, r21
std Z+62, r22
lds r23, 0x0100
sts 0x08ff, r24
lpm
lpm r25, Z
lpm r26, Z+
spm
in r27, 0x3f
out 0x3e, r28
push r29
pop r30
nop
sleep
wdr
break
table:
nop
