#include <cstdint>
#include <cstdio>
#include <cstdlib>

int main(int argc, char** argv) {
    long n = std::atol(argv[1]);
    uint32_t acc = 0;
    for (long i = 0; i < n; i++) {
        acc += static_cast<uint32_t>(i * i);
    }
    std::printf("%u\n", acc);
    return 0;
}
