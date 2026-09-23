#ifndef SAAIOS_SIT_BOOT_PREAMBLE_H
#define SAAIOS_SIT_BOOT_PREAMBLE_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

/* S5100SIT descriptor order from Pixel factory CBD: BOOT=0, TOC=1.
 * Call only AFTER LOAD BOOT / START_CP_BOOTLOADER, BEFORE MAIN START.
 * Callback sends exactly one packet and validates exactly one expected ACK;
 * it returns zero on success. No retries or device access in this helper.
 * Panther's reviewed images have a 0x410-byte TOC. Fail closed on other layouts.
 */
typedef int (*saaios_sit_exchange)(void *context, const uint8_t *packet,
                                  size_t length, uint32_t expected_ack);

static inline uint32_t saaios_sit_get_le32(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static inline void saaios_sit_put_le32(uint8_t *p, uint32_t v) {
    p[0] = (uint8_t)v; p[1] = (uint8_t)(v >> 8);
    p[2] = (uint8_t)(v >> 16); p[3] = (uint8_t)(v >> 24);
}

static inline int saaios_sit_boot_preamble(const uint8_t *image, size_t size,
                                          saaios_sit_exchange exchange,
                                          void *context) {
    if (!image || !exchange || size < 0x410 || memcmp(image, "TOC\0", 4) ||
        saaios_sit_get_le32(image + 12) != 0 ||
        saaios_sit_get_le32(image + 20) != 0x410 ||
        saaios_sit_get_le32(image + 28) < 2 ||
        saaios_sit_get_le32(image + 28) > 16) return -1;

    uint8_t word[4];
    saaios_sit_put_le32(word, 0xa00b); /* Initial BOOT READY, NOT final FIN. */
    if (exchange(context, word, sizeof(word), 0xc00b) != 0) return -1;
    saaios_sit_put_le32(word, 0xa110);
    if (exchange(context, word, sizeof(word), 0xc110) != 0) return -1;

    uint8_t packet[12 + 0x410];
    /* u16 command, u16 length=payload+8; u32 total, u32 offset. */
    packet[0] = 0x1b; packet[1] = 0xa1;
    packet[2] = 0x18; packet[3] = 0x04;
    saaios_sit_put_le32(packet + 4, 0x410);
    saaios_sit_put_le32(packet + 8, 0);
    memcpy(packet + 12, image, 0x410);
    if (exchange(context, packet, sizeof(packet), 0xc11b) != 0) return -1;

    saaios_sit_put_le32(word, 0xa11d); /* TOC DONE; no TOC CRC. */
    return exchange(context, word, sizeof(word), 0xc11d) == 0 ? 0 : -1;
}

#endif
