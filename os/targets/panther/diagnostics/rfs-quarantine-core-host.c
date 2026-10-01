/* Host-only, pure C RFS state machine. No file/device/network operations. */
#include "rfs-quarantine-core-host.h"

#include <stdlib.h>
#include <string.h>

struct rq_sha256 {
    uint32_t h[8];
    uint64_t bits;
    uint8_t block[64];
    size_t used;
};

struct rq_core {
    uint8_t *baseline;
    uint8_t *candidate;
    uint8_t baseline_digest[32];
    struct rq_sha256 received;
    enum rq_state state;
    size_t offset;
    size_t expected_chunk;
    unsigned grants;
};

static const uint8_t request_7[] =
    {0x07,0,0,0, 0x04,0,0,0, 0x03,0,0,0};
static const uint8_t request_3[] =
    {0x03,0,0,0, 0x0c,0,0,0, 0,0,0,0, 0x03,0,0,0, 0,0,0,0};
static const uint8_t request_6[] =
    {0x06,0,0x01,0, 0x10,0,0,0, 0x03,0,0,0, 0,0,0,0,
     0x06,0xe4,0x02,0, 0x02,0,0,0};
static const uint8_t status_7[] =
    {0x03,0,0,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0};
static const uint8_t status_complete[] =
    {0x03,0,0x01,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0};

static void rq_zero(void *ptr, size_t len)
{
    volatile uint8_t *p = (volatile uint8_t *)ptr;
    while (len--) *p++ = 0;
}

static uint32_t rotr(uint32_t value, unsigned shift)
{
    return (value >> shift) | (value << (32u - shift));
}

static uint32_t be32(const uint8_t *p)
{
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
           ((uint32_t)p[2] << 8) | p[3];
}

static uint16_t le16(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t le32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void put16(uint8_t *p, uint16_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void put32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static int equal_bytes(const uint8_t *a, const uint8_t *b, size_t len)
{
    uint8_t difference = 0;
    for (size_t i = 0; i < len; ++i) difference |= (uint8_t)(a[i] ^ b[i]);
    return difference == 0;
}

static void sha_transform(struct rq_sha256 *ctx, const uint8_t block[64])
{
    static const uint32_t k[64] = {
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,
        0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
        0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,
        0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,
        0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
        0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,
        0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,
        0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
        0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    };
    uint32_t w[64];
    uint32_t a, b, c, d, e, f, g, h;
    for (unsigned i = 0; i < 16; ++i) w[i] = be32(block + 4u * i);
    for (unsigned i = 16; i < 64; ++i) {
        uint32_t s0 = rotr(w[i-15], 7) ^ rotr(w[i-15], 18) ^
                      (w[i-15] >> 3);
        uint32_t s1 = rotr(w[i-2], 17) ^ rotr(w[i-2], 19) ^
                      (w[i-2] >> 10);
        w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    a = ctx->h[0]; b = ctx->h[1]; c = ctx->h[2]; d = ctx->h[3];
    e = ctx->h[4]; f = ctx->h[5]; g = ctx->h[6]; h = ctx->h[7];
    for (unsigned i = 0; i < 64; ++i) {
        uint32_t s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
        uint32_t ch = (e & f) ^ (~e & g);
        uint32_t t1 = h + s1 + ch + k[i] + w[i];
        uint32_t s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t t2 = s0 + maj;
        h = g; g = f; f = e; e = d + t1;
        d = c; c = b; b = a; a = t1 + t2;
    }
    ctx->h[0] += a; ctx->h[1] += b; ctx->h[2] += c; ctx->h[3] += d;
    ctx->h[4] += e; ctx->h[5] += f; ctx->h[6] += g; ctx->h[7] += h;
    rq_zero(w, sizeof w);
}

static void sha_init(struct rq_sha256 *ctx)
{
    static const uint32_t initial[8] = {
        0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,
        0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19
    };
    memset(ctx, 0, sizeof *ctx);
    memcpy(ctx->h, initial, sizeof initial);
}

static void sha_update(struct rq_sha256 *ctx, const uint8_t *data, size_t len)
{
    ctx->bits += (uint64_t)len * 8u; /* caller bounds data to 524288 B */
    while (len) {
        size_t space = 64u - ctx->used;
        size_t take = len < space ? len : space;
        memcpy(ctx->block + ctx->used, data, take);
        ctx->used += take;
        data += take;
        len -= take;
        if (ctx->used == 64u) {
            sha_transform(ctx, ctx->block);
            ctx->used = 0;
        }
    }
}

static void sha_final(struct rq_sha256 *ctx, uint8_t out[32])
{
    uint64_t bits = ctx->bits;
    ctx->block[ctx->used++] = 0x80;
    if (ctx->used > 56u) {
        memset(ctx->block + ctx->used, 0, 64u - ctx->used);
        sha_transform(ctx, ctx->block);
        ctx->used = 0;
    }
    memset(ctx->block + ctx->used, 0, 56u - ctx->used);
    for (unsigned i = 0; i < 8; ++i)
        ctx->block[56u + i] = (uint8_t)(bits >> (56u - 8u * i));
    sha_transform(ctx, ctx->block);
    for (unsigned i = 0; i < 8; ++i) {
        out[4u*i] = (uint8_t)(ctx->h[i] >> 24);
        out[4u*i+1] = (uint8_t)(ctx->h[i] >> 16);
        out[4u*i+2] = (uint8_t)(ctx->h[i] >> 8);
        out[4u*i+3] = (uint8_t)ctx->h[i];
    }
    rq_zero(ctx, sizeof *ctx);
}

static void sha_buffer(const uint8_t *data, size_t len, uint8_t out[32])
{
    struct rq_sha256 ctx;
    sha_init(&ctx);
    sha_update(&ctx, data, len);
    sha_final(&ctx, out);
}

int rq_host_sha256_fixture(const uint8_t *data, size_t len, uint8_t out[32])
{
    static const uint8_t empty = 0;
    if (!out || (len != 0 && !data) || len > RQ_BASELINE_BYTES) return -1;
    sha_buffer(data ? data : &empty, len, out);
    return 0;
}

static void abort_core(struct rq_core *core)
{
    if (core->candidate) {
        rq_zero(core->candidate, RQ_BASELINE_BYTES);
        free(core->candidate);
        core->candidate = NULL;
    }
    rq_zero(&core->received, sizeof core->received);
    core->state = RQ_ABORTED;
}

struct rq_core *rq_create(const char *host_ack, const uint8_t *baseline,
                          size_t baseline_len,
                          const uint8_t pinned_sha256[32])
{
    uint8_t actual[32];
    struct rq_core *core;
    if (!host_ack || strcmp(host_ack, RQ_HOST_ACK) != 0 || !baseline ||
        !pinned_sha256 || baseline_len != RQ_BASELINE_BYTES) return NULL;
    core = (struct rq_core *)calloc(1, sizeof *core);
    if (!core) return NULL;
    core->baseline = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    if (!core->baseline) { free(core); return NULL; }
    memcpy(core->baseline, baseline, RQ_BASELINE_BYTES);
    sha_buffer(core->baseline, RQ_BASELINE_BYTES, actual);
    if (!equal_bytes(actual, pinned_sha256, sizeof actual)) {
        rq_zero(actual, sizeof actual);
        rq_destroy(core);
        return NULL;
    }
    memcpy(core->baseline_digest, actual, sizeof actual);
    rq_zero(actual, sizeof actual);
    core->state = RQ_WAIT_7;
    return core;
}

void rq_destroy(struct rq_core *core)
{
    if (!core) return;
    abort_core(core);
    if (core->baseline) {
        rq_zero(core->baseline, RQ_BASELINE_BYTES);
        free(core->baseline);
    }
    rq_zero(core, sizeof *core);
    free(core);
}

static int exact(const uint8_t *frame, size_t len,
                 const uint8_t *expected, size_t expected_len)
{
    return len == expected_len && equal_bytes(frame, expected, len);
}

static enum rq_result issue_grant(struct rq_core *core, uint8_t *reply,
                                  size_t *reply_len)
{
    size_t remaining;
    if (core->offset >= RQ_WRITE_BYTES || core->grants >= RQ_MAX_GRANTS)
        return RQ_ERROR;
    remaining = RQ_WRITE_BYTES - core->offset;
    core->expected_chunk = remaining < RQ_CHUNK_BYTES ? remaining : RQ_CHUNK_BYTES;
    put16(reply, 2);
    put16(reply + 2, 1);
    put32(reply + 4, 12);
    put32(reply + 8, 3);
    put32(reply + 12, (uint32_t)core->offset);
    put32(reply + 16, (uint32_t)core->expected_chunk);
    ++core->grants;
    *reply_len = 20;
    return RQ_REPLY;
}

static int candidate_invariants(struct rq_core *core)
{
    uint8_t actual[32], received[32], prefix[32];
    int valid;
    if (core->offset != RQ_WRITE_BYTES || core->grants != RQ_MAX_GRANTS ||
        !core->candidate) return 0;
    sha_buffer(core->baseline, RQ_BASELINE_BYTES, actual);
    sha_final(&core->received, received);
    sha_buffer(core->candidate, RQ_WRITE_BYTES, prefix);
    valid = equal_bytes(actual, core->baseline_digest, 32) &&
            equal_bytes(received, prefix, 32) &&
            equal_bytes(core->candidate + RQ_WRITE_BYTES,
                        core->baseline + RQ_WRITE_BYTES,
                        RQ_BASELINE_BYTES - RQ_WRITE_BYTES);
    rq_zero(actual, sizeof actual);
    rq_zero(received, sizeof received);
    rq_zero(prefix, sizeof prefix);
    return valid;
}

enum rq_result rq_accept(struct rq_core *core, const uint8_t *frame,
                         size_t frame_len, uint8_t *reply, size_t reply_cap,
                         size_t *reply_len)
{
    uint32_t payload_len;
    if (!core) return RQ_ERROR;
    if (!reply_len) { abort_core(core); return RQ_ERROR; }
    *reply_len = 0;
    if (core->state == RQ_ABORTED) return RQ_ERROR;
    if (core->state == RQ_COMPLETE) {
        abort_core(core);
        *reply_len = 0;
        return RQ_ERROR;
    }
    if (!frame || !reply || reply_cap < RQ_MAX_RESPONSE_BYTES ||
        frame_len < 8 || frame_len > 8u + 12u + RQ_CHUNK_BYTES)
        goto failure;
    payload_len = le32(frame + 4);
    if (payload_len > 12u + RQ_CHUNK_BYTES || frame_len != 8u + payload_len)
        goto failure;

    if (core->state == RQ_WAIT_7) {
        uint8_t check[32];
        if (!exact(frame, frame_len, request_7, sizeof request_7)) goto failure;
        core->candidate = (uint8_t *)malloc(RQ_BASELINE_BYTES);
        if (!core->candidate) goto failure;
        memcpy(core->candidate, core->baseline, RQ_BASELINE_BYTES);
        sha_buffer(core->candidate, RQ_BASELINE_BYTES, check);
        if (!equal_bytes(check, core->baseline_digest, sizeof check)) {
            rq_zero(check, sizeof check);
            goto failure;
        }
        rq_zero(check, sizeof check);
        core->state = RQ_WAIT_3;
        memcpy(reply, status_7, sizeof status_7);
        *reply_len = sizeof status_7;
        return RQ_REPLY;
    }
    if (core->state == RQ_WAIT_3) {
        if (!exact(frame, frame_len, request_3, sizeof request_3)) goto failure;
        core->state = RQ_WAIT_6;
        return RQ_NO_REPLY;
    }
    if (core->state == RQ_WAIT_6) {
        enum rq_result result;
        if (!core->candidate ||
            !exact(frame, frame_len, request_6, sizeof request_6)) goto failure;
        sha_init(&core->received);
        core->state = RQ_WAIT_DATA;
        result = issue_grant(core, reply, reply_len);
        if (result == RQ_ERROR) goto failure;
        return result;
    }
    if (core->state == RQ_WAIT_DATA) {
        uint32_t chunk;
        enum rq_result result;
        if (le16(frame) != 2 || le16(frame + 2) != 1 ||
            payload_len != 12u + core->expected_chunk ||
            frame_len != 20u + core->expected_chunk ||
            le32(frame + 8) != 0 || le32(frame + 12) != 3)
            goto failure;
        chunk = le32(frame + 16);
        /* Exact advertised chunk is a stricter SaaiOS policy than factory. */
        if (chunk != core->expected_chunk ||
            chunk > RQ_WRITE_BYTES - core->offset) goto failure;
        sha_update(&core->received, frame + 20, chunk);
        memcpy(core->candidate + core->offset, frame + 20, chunk);
        core->offset += chunk;
        if (core->offset == RQ_WRITE_BYTES) {
            if (!candidate_invariants(core)) goto failure;
            core->state = RQ_COMPLETE;
            memcpy(reply, status_complete, sizeof status_complete);
            *reply_len = sizeof status_complete;
            return RQ_REPLY;
        }
        result = issue_grant(core, reply, reply_len);
        if (result == RQ_ERROR) goto failure;
        return result;
    }
failure:
    abort_core(core);
    *reply_len = 0;
    return RQ_ERROR;
}

enum rq_state rq_get_state(const struct rq_core *core)
{
    return core ? core->state : RQ_ABORTED;
}

unsigned rq_grants_issued(const struct rq_core *core)
{
    return core ? core->grants : 0;
}

size_t rq_bytes_received(const struct rq_core *core)
{
    return core ? core->offset : 0;
}

int rq_candidate_prepared(const struct rq_core *core)
{
    return core && core->candidate != NULL;
}

int rq_copy_completed_candidate(const struct rq_core *core, uint8_t *out,
                                size_t out_len)
{
    if (!core || core->state != RQ_COMPLETE || !core->candidate || !out ||
        out_len != RQ_BASELINE_BYTES) return -1;
    memcpy(out, core->candidate, RQ_BASELINE_BYTES);
    return 0;
}
