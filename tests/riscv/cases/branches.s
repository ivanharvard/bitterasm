# B-type: branches
# Offsets are deliberately non-round bit patterns (not powers of two or
# all-ones/all-zero runs) so a sub-field boundary or shift-amount bug in
# the scrambled B-type immediate would actually change the result.
beq t0, t1, 3510
bne t2, t3, -3510
blt a0, a1, 2726
bge a2, a3, -1234
bltu s0, s1, 890
bgeu s2, s3, -678
