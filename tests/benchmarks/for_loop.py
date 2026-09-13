import sys

n = int(sys.argv[1])
out_path = sys.argv[2]

with open(out_path, "w") as f:
    f.write("[\n")
    for i in range(n):
        f.write('  {\n    "kind": "Int",\n    "value": "%d"\n  }' % (i * i))
        f.write(",\n" if i + 1 < n else "\n")
    f.write("]\n")
