# J-type / I-type: jumps. `jal`'s target is a real backward and forward
# label, same reasoning as branches.s. `jalr`'s immediate is a plain
# I-type field, not PC-relative, so it keeps taking a raw literal.
loop_start:
add t0, t0, t1
jal ra, loop_start
jal t0, forward_target
add t0, t0, t1
forward_target:
jalr ra, t1, 1685
jalr zero, ra, -1370
