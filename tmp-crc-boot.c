#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <zlib.h>

static uint32_t crc_mpeg2(const uint8_t *data, size_t len) {
    uint32_t crc = 0xffffffffu;
    for (size_t i = 0; i < len; i++) {
        crc ^= ((uint32_t)data[i]) << 24;
        for (int b = 0; b < 8; b++)
            crc = (crc & 0x80000000u) ? ((crc << 1) ^ 0x04C11DB7u) : (crc << 1);
    }
    return crc;
}

static uint32_t crc32c_sw(const uint8_t *data, size_t len) {
    /* Castagnoli reflected */
    static uint32_t table[256];
    static int init;
    if (!init) {
        for (uint32_t i = 0; i < 256; i++) {
            uint32_t c = i;
            for (int k = 0; k < 8; k++)
                c = (c & 1) ? (0x82F63B78u ^ (c >> 1)) : (c >> 1);
            table[i] = c;
        }
        init = 1;
    }
    uint32_t crc = 0xffffffffu;
    for (size_t i = 0; i < len; i++)
        crc = table[(crc ^ data[i]) & 0xff] ^ (crc >> 8);
    return crc ^ 0xffffffffu;
}

int main(int argc, char **argv) {
    FILE *f = fopen(argv[1], "rb");
    uint8_t *boot = malloc(0x16800);
    fseek(f, 0x410, SEEK_SET);
    fread(boot, 1, 0x16800, f);
    uint32_t want = 0x101b4a2c;
    printf("zlib=%08x mpeg2=%08x mpeg2x=%08x crc32c=%08x want=%08x\n",
           (unsigned)crc32(0L, boot, 0x16800),
           crc_mpeg2(boot, 0x16800),
           crc_mpeg2(boot, 0x16800) ^ 0xffffffffu,
           crc32c_sw(boot, 0x16800), want);

    /* Try every prefix length for zlib/mpeg2/crc32c on BOOT - fast */
    for (size_t n = 4; n <= 0x16800; n += 4) {
        if (crc32(0L, boot, n) == want) printf("zlib MATCH n=%zx\n", n);
        if (crc_mpeg2(boot, n) == want) printf("mpeg2 MATCH n=%zx\n", n);
        if ((crc_mpeg2(boot, n) ^ 0xffffffffu) == want) printf("mpeg2x MATCH n=%zx\n", n);
        if (crc32c_sw(boot, n) == want) printf("crc32c MATCH n=%zx\n", n);
    }
    free(boot);
    fclose(f);
    return 0;
}
