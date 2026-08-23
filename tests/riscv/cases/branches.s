# B-type: branches, with real backward and forward label targets — see
# std/riscv/native.basm's `target`/`@here` design. `add` lines are filler,
# just to give branch offsets some nontrivial magnitude/variety; they're
# not testing anything on their own (R-type already has its own case).
loop_start:
add t0, t0, t1
add t0, t0, t1
beq t0, t1, loop_start
bne t2, t3, loop_start
blt a0, a1, forward_target
add t0, t0, t1
bge a2, a3, forward_target
add t0, t0, t1
add t0, t0, t1
bltu s0, s1, loop_start
bgeu s2, s3, forward_target
forward_target:
