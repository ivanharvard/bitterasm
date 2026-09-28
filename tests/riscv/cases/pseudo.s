# Pseudo-instructions: li, la
back:
li a0, 1
li a1, -2048
li a2, 2047
li a3, 0x800
li a4, 0x1000
li a5, 0x12345
li a6, -0x12345
li a7, 0x7FFFF800
la t0, back
la t1, fwd
fwd:
ecall
