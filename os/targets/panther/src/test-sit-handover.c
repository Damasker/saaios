#include "sit-handover.h"
#include <assert.h>
#include <stdio.h>

int main(void)
{
    /* Deliberately synthetic; no authentic device identity/signature fixtures. */
    const char cdt[] = "0x123456789abcdef0123456789abcdef012";
    const uint32_t expected[10] = {
        0x1234,0x56,0x78,0x9abc,0xde,0xf0,0x12,0x34,0x5678,0x9abcdef0
    };
    /* Construct exact width input, with no trailing bytes. */
    const char valid[] = "0x123456789abcdef0123456789abcdef0";
    uint32_t fields[10], before[10];
    assert(saaios_parse_cdt(valid, sizeof(valid)-1, fields) == 0);
    assert(memcmp(fields, expected, sizeof(fields)) == 0);
    memcpy(before, fields, sizeof(fields));
    for (size_t n = 0; n < sizeof(valid)-1; ++n)
        assert(saaios_parse_cdt(valid, n, fields) == -1);
    assert(saaios_parse_cdt(cdt, sizeof(cdt)-1, fields) == -1);
    char bad[sizeof(valid)];
    memcpy(bad, valid, sizeof(valid)); bad[10] = 'g';
    assert(saaios_parse_cdt(bad, sizeof(bad)-1, fields) == -1);
    assert(memcmp(fields, before, sizeof(fields)) == 0);
    assert(saaios_parse_cdt(NULL, 34, fields) == -1);

    uint8_t id[16] = "000000000000000", sig[64];
    for (unsigned i = 0; i < 64; ++i) sig[i] = (uint8_t)i;
    struct saaios_handover_inputs in = {
        .words = {1,0x12345678,2,3,4,5,6,0,0,0,10,11,12,13,14,0},
        .identity = {id,id}, .identity_size = {16,16},
        .signature = sig, .signature_size = 64
    };
    uint8_t out[163], golden[161] = {0};
    const uint8_t word_bytes[64] = {
        1,0,0,0, 0x78,0x56,0x34,0x12, 2,0,0,0, 3,0,0,0,
        4,0,0,0, 5,0,0,0, 6,0,0,0, 0,0,0,0,
        0,0,0,0, 0,0,0,0, 10,0,0,0, 11,0,0,0,
        12,0,0,0, 13,0,0,0, 14,0,0,0, 0,0,0,0
    };
    memcpy(golden, word_bytes, 64);
    memcpy(golden+64, id, 16); memcpy(golden+80, id, 16);
    memcpy(golden+96, sig, 64);
    memset(out, 0xa5, sizeof(out));
    assert(saaios_build_handover(out+1, 161, &in) == 0);
    assert(out[0] == 0xa5 && out[162] == 0xa5);
    assert(memcmp(out+1, golden, 161) == 0);
    memset(out, 0xa5, sizeof(out));
    for (size_t n = 0; n < 161; ++n)
        assert(saaios_build_handover(out, n, &in) == -1);
    const unsigned forbidden[] = {7,8,9,15};
    for (unsigned i = 0; i < 4; ++i) {
        in.words[forbidden[i]] = 1;
        assert(saaios_build_handover(out, sizeof(out), &in) == -1);
        in.words[forbidden[i]] = 0;
    }
    in.signature_size = 63;
    assert(saaios_build_handover(out, sizeof(out), &in) == -1);
    in.signature_size = 64; id[15] = '1';
    assert(saaios_build_handover(out, sizeof(out), &in) == -1);
    id[15] = 0; id[3] = 'x';
    assert(saaios_build_handover(out, sizeof(out), &in) == -1);
    for (unsigned i = 0; i < sizeof(out); ++i) assert(out[i] == 0xa5);
    puts("PASS: CDT parsing, 161-byte layout, bounds, control guards, atomic failure");
    return 0;
}
