# Relative jumps and calls, conditional branches, skips, with backward and
# forward label targets.
back:
nop
rjmp back
rcall forward
breq back
brne forward
brcs back
brcc forward
brlo back
brsh forward
brmi back
brpl forward
brlt back
brge forward
brhs back
brhc forward
brts back
brtc forward
brvs back
brvc forward
brie back
brid forward
brbs 2, back
brbc 6, forward
cp r1, r2
cpc r3, r4
cpi r16, 0x42
cpse r5, r6
sbrc r7, 0
sbrs r8, 7
sbic 0x03, 1
sbis 0x1f, 7
nop
forward:
ret
