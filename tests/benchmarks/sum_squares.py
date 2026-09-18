import sys

n = int(sys.argv[1])
acc = 0
for i in range(n):
    acc = (acc + (i * i & 0xFFFFFFFF)) & 0xFFFFFFFF
print(acc)
