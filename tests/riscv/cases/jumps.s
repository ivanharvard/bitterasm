# J-type / I-type: jumps
# Non-round offsets for the same reason as branches.s: jal's immediate is
# scrambled across four sub-fields, so a boundary/shift bug there needs a
# mixed bit pattern to actually surface.
jal ra, 561094
jal t0, -845926
jalr ra, t1, 1685
jalr zero, ra, -1370
