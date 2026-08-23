# S-type: stores
# Non-round offsets: S-type splits its immediate into a 7-bit and a 5-bit
# group, so a boundary bug between them needs a mixed pattern to surface.
sb t0, 987(sp)
sh t1, -654(sp)
sw t2, 1685(sp)
