#include "sit-boot-preamble.h"
#include <assert.h>
#include <stdio.h>

struct recorder { int calls, fail_at; const uint8_t *image; };
static int exchange(void *ctx, const uint8_t *p, size_t n, uint32_t ack) {
    struct recorder *r = ctx;
    static const uint32_t acks[] = {0xc00b, 0xc110, 0xc11b, 0xc11d};
    static const uint32_t cmds[] = {0xa00b, 0xa110, 0x0418a11b, 0xa11d};
    int i = r->calls++;
    assert(i < 4 && ack == acks[i]);
    assert(n == (i == 2 ? 0x41c : 4));
    assert(saaios_sit_get_le32(p) == cmds[i]);
    if (i == 2) {
        assert(saaios_sit_get_le32(p + 4) == 0x410);
        assert(saaios_sit_get_le32(p + 8) == 0);
        assert(memcmp(p + 12, r->image, 0x410) == 0);
    }
    return i == r->fail_at ? -1 : 0;
}

int main(void) {
    uint8_t image[0x410] = {'T', 'O', 'C', 0};
    saaios_sit_put_le32(image + 20, sizeof(image));
    saaios_sit_put_le32(image + 28, 7);
    for (int fail = -1; fail < 4; fail++) {
        struct recorder r = {0, fail, image};
        int result = saaios_sit_boot_preamble(image, sizeof(image), exchange, &r);
        assert((result == 0) == (fail == -1));
        assert(r.calls == (fail == -1 ? 4 : fail + 1));
    }
    struct recorder r = {0, -1, image};
    assert(saaios_sit_boot_preamble(NULL, sizeof(image), exchange, &r) < 0);
    assert(saaios_sit_boot_preamble(image, sizeof(image), NULL, &r) < 0);
    assert(saaios_sit_boot_preamble(image, sizeof(image) - 1, exchange, &r) < 0);
    const size_t corrupt[] = {0, 12, 20, 28};
    for (size_t i = 0; i < sizeof(corrupt)/sizeof(corrupt[0]); i++) {
        uint8_t save = image[corrupt[i]];
        image[corrupt[i]] = 0xff;
        assert(saaios_sit_boot_preamble(image, sizeof(image), exchange, &r) < 0);
        image[corrupt[i]] = save;
    }
    assert(r.calls == 0);
    puts("PASS: packet bytes/order, every failure boundary, invalid inputs");
}
