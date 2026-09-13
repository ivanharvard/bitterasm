#include <cstdio>
#include <cstdlib>

int main(int argc, char** argv) {
    long n = std::atol(argv[1]);
    FILE* f = std::fopen(argv[2], "w");

    std::fputs("[\n", f);
    for (long i = 0; i < n; i++) {
        std::fprintf(f, "  {\n    \"kind\": \"Int\",\n    \"value\": \"%ld\"\n  }", i * i);
        std::fputs(i + 1 < n ? ",\n" : "\n", f);
    }
    std::fputs("]\n", f);

    std::fclose(f);
    return 0;
}
