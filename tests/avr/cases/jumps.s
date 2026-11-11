# Long jumps and calls, to labels and to fixed addresses, and indirect
# ones through Z.
start:
jmp start
jmp far
call start
call far
jmp 0x100
call 0x7ffe
ijmp
icall
reti
ldi r30, pm_lo8(far)
ldi r31, pm_hi8(far)
nop
nop
far:
ret
