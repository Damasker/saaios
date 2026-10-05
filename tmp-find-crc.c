#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <zlib.h>

int main(int argc, char **argv) {
    FILE *f = fopen(argv[1], "rb");
    if (!f) return 1;
    fseek(f, 0x16c10, SEEK_SET);
    size_t size = 0x5917acc;
    uint8_t *buf = malloc(size);
    if (!buf || fread(buf, 1, size, f) != size) return 2;
    fclose(f);
    uint32_t want = 0xb0f14905u;

    /* Growing prefix */
    for (size_t n = 0x1000; n <= size; n = (n < 0x800000) ? n * 2 : n + 0x800000) {
        if (n > size) n = size;
        if (crc32(0L, buf, n) == want)
            printf("MATCH prefix len=0x%zx\n", n);
        if (n == size) break;
    }

    /* Exact full size already known miss; try init 0xffffffff via zlib equiv:
       zlib crc32(0,...) uses init 0. */
    /* Check BOOT stage CRC vs its TOC for algorithm validation */
    free(buf);

    f = fopen(argv[1], "rb");
    uint8_t toc[512];
    fread(toc, 1, 512, f);
    /* BOOT at typically idx1 */
    for (int i = 0; i < 16; i++) {
        char name[13] = {0};
        memcpy(name, toc + i * 32, 12);
        uint32_t b_off, m_off, sz, crc, idx;
        memcpy(&b_off, toc + i * 32 + 12, 4);
        memcpy(&m_off, toc + i * 32 + 16, 4);
        memcpy(&sz, toc + i * 32 + 20, 4);
        memcpy(&crc, toc + i * 32 + 24, 4);
        memcpy(&idx, toc + i * 32 + 28, 4);
        if (name[0] == 0) continue;
        printf("TOC %s b=%x sz=%x crc=%x idx=%u\n", name, b_off, sz, crc, idx);
        if (sz == 0 || sz > 0x10000000u || b_off + sz > 0x10000000u) continue;
        uint8_t *s = malloc(sz);
        fseek(f, b_off, SEEK_SET);
        if (fread(s, 1, sz, f) != sz) { free(s); continue; }
        uint32_t z = crc32(0L, s, sz);
        printf("  zlib=%x %s\n", z, z == crc ? "MATCH" : "no");
        free(s);
    }
    fclose(f);
    return 0;
}
