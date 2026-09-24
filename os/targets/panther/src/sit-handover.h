#ifndef SAAIOS_SIT_HANDOVER_H
#define SAAIOS_SIT_HANDOVER_H
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/* Pure serialization only: no filesystem access, ioctl or hardware defaults. */
#define SAAIOS_HANDOVER_SIZE 161u
struct saaios_handover_inputs {
    /* version through reserved[3], in kernel t_handover_block_info order. */
    uint32_t words[16];
    const uint8_t *identity[2];
    size_t identity_size[2];
    const uint8_t *signature;
    size_t signature_size;
};

/* Strict fixed-width form observed in CBD; not its permissive sscanf. */
static int saaios_parse_cdt(const char *s, size_t n, uint32_t out[10])
{
    static const unsigned widths[10] = {4,2,2,4,2,2,2,2,4,8};
    uint32_t parsed[10] = {0};
    size_t pos = 2;
    if (!s || !out || n != 34 || s[0] != '0' || s[1] != 'x') return -1;
    for (unsigned i = 0; i < 10; ++i) {
        for (unsigned j = 0; j < widths[i]; ++j) {
            unsigned char c = (unsigned char)s[pos++];
            unsigned v;
            if (c >= '0' && c <= '9') v = c - '0';
            else if (c >= 'a' && c <= 'f') v = c - 'a' + 10;
            else if (c >= 'A' && c <= 'F') v = c - 'A' + 10;
            else return -1;
            parsed[i] = (parsed[i] << 4) | v;
        }
    }
    memcpy(out, parsed, sizeof(parsed));
    return 0;
}

/* Fail closed on factory/reset/debug controls. Output unchanged on failure.
 * Caller must establish provenance and map every word; zero is not a fallback.
 * Successful serialization does NOT authenticate signature or hardware inputs.
 */
static int saaios_build_handover(uint8_t *out, size_t capacity,
                               const struct saaios_handover_inputs *in)
{
    if (!out || !in || capacity < SAAIOS_HANDOVER_SIZE || in->words[0] != 1 ||
        in->words[7] || in->words[8] || in->words[9] || in->words[15] ||
        !in->signature || in->signature_size != 64) return -1;
    for (unsigned i = 0; i < 2; ++i) {
        if (!in->identity[i] || in->identity_size[i] != 16 ||
            in->identity[i][15] != 0) return -1;
        for (unsigned j = 0; j < 15; ++j)
            if (in->identity[i][j] < '0' || in->identity[i][j] > '9') return -1;
    }
    uint8_t result[SAAIOS_HANDOVER_SIZE] = {0};
    for (unsigned i = 0; i < 16; ++i)
        for (unsigned j = 0; j < 4; ++j)
            result[4*i+j] = (uint8_t)(in->words[i] >> (8*j));
    memcpy(result+64, in->identity[0], 16);
    memcpy(result+80, in->identity[1], 16);
    memcpy(result+96, in->signature, 64);
    memcpy(out, result, sizeof(result));
    /* Best-effort clearing of the temporary device-bound buffer. */
    volatile uint8_t *wipe = result;
    for (size_t i = 0; i < sizeof(result); ++i) wipe[i] = 0;
    return 0;
}
#endif
