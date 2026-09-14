# tests/wasm/run_tests.py's independent oracle: WABT (the WebAssembly
# Binary Toolkit), an independently maintained reference implementation of
# the WASM spec, sharing no code with bitterasm/bitter or with
# std/wasm/impl.basm's own opcode tables. The same role GNU binutils plays
# for tests/riscv/docker, adapted to what WASM actually needs checked (see
# validate_and_run.sh's own doc comment for why that's "validates and
# executes," not "produces byte-identical output" — WASM's own spec
# permits more than one valid encoding of the same module).
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends wabt \
    && rm -rf /var/lib/apt/lists/*

COPY validate_and_run.sh /usr/local/bin/validate_and_run.sh
RUN chmod +x /usr/local/bin/validate_and_run.sh

ENTRYPOINT ["/usr/local/bin/validate_and_run.sh"]
