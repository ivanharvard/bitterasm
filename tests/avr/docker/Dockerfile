# The "official encoder" tests/avr/run_tests.py checks bitter's output
# against: GNU binutils' AVR assembler, packaged and maintained
# independently of this project.
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends binutils-avr \
    && rm -rf /var/lib/apt/lists/*

COPY assemble.sh /usr/local/bin/assemble.sh
RUN chmod +x /usr/local/bin/assemble.sh

ENTRYPOINT ["/usr/local/bin/assemble.sh"]
