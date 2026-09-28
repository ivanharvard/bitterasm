.PHONY: install

install:
	./install.sh

compare:
	uv run tests/riscv/run_tests.py

fmt:
	bitterasm fmt .

mdbook:
	bitterasm doc std -o docs/book/src/std/reference --summary docs/book/src/SUMMARY.md
	mdbook serve docs/book --open