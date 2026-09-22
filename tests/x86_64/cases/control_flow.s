# The 50-instruction (150-byte) blocks of `add rax, rbx` below aren't padding
# for its own sake — see tests/x86_64/run_tests.py's own doc comment: GNU as
# auto-selects the short (rel8) jump/Jcc encoding whenever a target is close
# enough, but this project's own encoder is deliberately near-only (rel32),
# by explicit design. Every label
# below sits more than 127 bytes from every jump/Jcc/call that references it,
# so the short form is never even reachable — both sides are then forced into
# genuine agreement on rel32, not a coincidence of one side happening to pick
# the same size as the other.
label_a:
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
label_b:
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
add rax, rbx
jmp label_a
je label_b
call label_a
ret
syscall
