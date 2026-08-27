.PHONY: install

install:
	./install.sh

compare:
	uv run tests/riscv/run_tests.py

fmt:
	bitterasm fmt .

