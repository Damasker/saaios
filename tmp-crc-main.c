#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <zlib.h>

int main(int argc, char **argv) {
    FILE *f = fopen(argv[1], "rb");
    if (!f) return 1;
    if (fseek(f, 0x16c10, SEEK_SET)) return 2;
    size_t size = 0x5917acc;
    uint8_t *buf = malloc(size);
    if (!buf || fread(buf, 1, size, f) != size) return 3;
    fclose(f);
    uint32_t want = 0xb0f14905u;
    uint32_t z = crc32(0L, buf, size);
    printf("zlib_crc32 = 0x%08x %s\n", z, z == want ? "MATCH" : "");
    /* IEEE reflected already via zlib */

    /* Try CRC of MAIN as big-endian words / other windows */
    /* Checksum used by some Shannon dumps: rolling add */
    uint32_t add = 0;
    for (size_t i = 0; i + 4 <= size; i += 4) {
        uint32_t w = (uint32_t)buf[i] | ((uint32_t)buf[i+1] << 8) |
                     ((uint32_t)buf[i+2] << 16) | ((uint32_t)buf[i+3] << 24);
        add += w;
    }
    printf("sum32_le = 0x%08x %s\n", add, add == want ? "MATCH" : "");

    /* Maybe CRC is only over first page or excludes header — sample */
    for (size_t n = 0x1000; n <= 0x100000; n <<= 1) {
        z = crc32(0L, buf, n);
        if (z == want) printf("zlib first 0x%zx MATCH\n", n);
    }
    /* Differential: patch MOVS and see CRC delta with zlib (wrong algo but for structure) */
    uint8_t save = buf[0x14fb404 - 0x16c10];
    printf("byte at patch off in MAIN: 0x%02x (expect 0x02)\n", save);
    buf[0x14fb404 - 0x16c10] = 0x05;
    uint32_t z2 = crc32(0L, buf, size);
    printf("zlib after patch = 0x%08x delta=0x%08x\n", z2, z ^ z2);
    free(buf);
    return 0;
}
