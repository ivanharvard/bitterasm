.PHONY: install coverage

install:
	./install.sh

compare:
	uv run tests/riscv/run_tests.py

fmt:
	bitterasm fmt .

mdbook:
	bitterasm doc std -o docs/book/src/std/reference --summary docs/book/src/SUMMARY.md
	mdbook serve docs/book --open

# Recompute instruction coverage (`tests/coverage/`), update the tables and
# `impl.basm` headers that show it, then the std reference pages built from
# those headers.
coverage:
	BITTERASM_BLESS=1 cargo test --test coverage
	cargo run --quiet -- doc std -o docs/book/src/std/reference --summary docs/book/src/SUMMARY.md
