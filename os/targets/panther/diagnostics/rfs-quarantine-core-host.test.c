#include "rfs-quarantine-core-host.h"

/* Test actions live inside assertions, so optimization must not elide them. */
#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static uint8_t baseline[RQ_BASELINE_BYTES];
/* Independently computed SHA-256 of 524288 bytes of 0x5a. */
static const uint8_t pinned[32] = {
    0x0d,0x57,0xce,0x7e,0x6f,0x29,0x9b,0x77,
    0xf1,0xaa,0x75,0xb8,0xb0,0x19,0x8a,0xaa,
    0xa9,0x10,0xb6,0xfd,0x2e,0xa0,0x9b,0xfc,
    0x4a,0x5f,0xcc,0x4d,0x20,0x23,0xf5,0xd2
};
static const uint8_t cmd7[] =
    {0x07,0,0,0, 0x04,0,0,0, 0x03,0,0,0};
static const uint8_t cmd3[] =
    {0x03,0,0,0, 0x0c,0,0,0, 0,0,0,0, 0x03,0,0,0, 0,0,0,0};
static const uint8_t cmd6[] =
    {0x06,0,0x01,0, 0x10,0,0,0, 0x03,0,0,0, 0,0,0,0,
     0x06,0xe4,0x02,0, 0x02,0,0,0};
static const uint8_t status7[] =
    {0x03,0,0,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0};
static const uint8_t final_status[] =
    {0x03,0,0x01,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0};

static unsigned hex_nibble(char c)
{
    if (c >= '0' && c <= '9') return (unsigned)(c - '0');
    if (c >= 'a' && c <= 'f') return (unsigned)(c - 'a' + 10);
    assert(0 && "invalid fixture hex digit");
    return 0;
}

static void expect_sha(const uint8_t *data, size_t len, const char *hex)
{
    uint8_t actual[32];
    assert(strlen(hex) == 64);
    assert(rq_host_sha256_fixture(data, len, actual) == 0);
    for (size_t i = 0; i < sizeof actual; ++i)
        assert(actual[i] ==
               (hex_nibble(hex[2*i]) << 4 | hex_nibble(hex[2*i+1])));
}

static void test_sha_vectors(void)
{
    uint8_t a[64], digest[32];
    memset(a, 'a', sizeof a);
    expect_sha(NULL, 0,
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    expect_sha((const uint8_t *)"abc", 3,
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    expect_sha(a, 55,
      "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318");
    expect_sha(a, 56,
      "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a");
    expect_sha(a, 63,
      "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34");
    expect_sha(a, 64,
      "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");
    assert(rq_host_sha256_fixture(NULL, 1, digest) == -1);
    assert(rq_host_sha256_fixture(a, RQ_BASELINE_BYTES + 1u, digest) == -1);
    assert(rq_host_sha256_fixture(a, 1, NULL) == -1);
    puts("SHA-256 vectors (0/abc/55/56/63/64): PASS");
}

static uint16_t get16(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t get32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void put16(uint8_t *p, uint16_t n)
{
    p[0] = (uint8_t)n;
    p[1] = (uint8_t)(n >> 8);
}

static void put32(uint8_t *p, uint32_t n)
{
    p[0] = (uint8_t)n;
    p[1] = (uint8_t)(n >> 8);
    p[2] = (uint8_t)(n >> 16);
    p[3] = (uint8_t)(n >> 24);
}

static struct rq_core *new_core(void)
{
    struct rq_core *core;
    memset(baseline, 0x5a, sizeof baseline);
    core = rq_create(RQ_HOST_ACK, baseline, sizeof baseline, pinned);
    assert(core);
    return core;
}

static void through_grant(struct rq_core *core, uint8_t grant[20])
{
    size_t len = 99;
    assert(rq_get_state(core) == RQ_WAIT_7);
    assert(!rq_candidate_prepared(core));
    assert(rq_accept(core, cmd7, sizeof cmd7, grant, 20, &len) == RQ_REPLY);
    assert(len == sizeof status7 && memcmp(grant, status7, len) == 0);
    assert(rq_candidate_prepared(core));
    assert(rq_copy_completed_candidate(core, baseline, sizeof baseline) == -1);
    assert(rq_accept(core, cmd3, sizeof cmd3, grant, 20, &len) == RQ_NO_REPLY);
    assert(len == 0);
    assert(rq_accept(core, cmd6, sizeof cmd6, grant, 20, &len) == RQ_REPLY);
    assert(len == 20 && get16(grant) == 2 && get16(grant + 2) == 1);
}

static size_t data_frame(uint8_t out[2032], const uint8_t grant[20],
                         uint8_t fill)
{
    uint32_t chunk = get32(grant + 16);
    assert(chunk <= RQ_CHUNK_BYTES);
    memset(out, fill, 20u + chunk);
    put16(out, 2);
    put16(out + 2, get16(grant + 2));
    put32(out + 4, 12u + chunk);
    put32(out + 8, 0);
    put32(out + 12, 3);
    put32(out + 16, chunk);
    return 20u + chunk;
}

static void complete(struct rq_core *core, uint8_t fill)
{
    uint8_t reply[20], frame[2032], *snapshot;
    size_t reply_len, frame_len;
    through_grant(core, reply);
    for (unsigned i = 0; i < RQ_MAX_GRANTS; ++i) {
        uint32_t offset = get32(reply + 12);
        uint32_t chunk = get32(reply + 16);
        assert(get16(reply) == 2 && get16(reply + 2) == 1);
        assert(get32(reply + 4) == 12 && get32(reply + 8) == 3);
        assert(offset == i * RQ_CHUNK_BYTES);
        assert(chunk == (i == RQ_MAX_GRANTS - 1 ? 318u : RQ_CHUNK_BYTES));
        frame_len = data_frame(frame, reply, fill);
        assert(rq_accept(core, frame, frame_len, reply, sizeof reply,
                         &reply_len) == RQ_REPLY);
        if (i == RQ_MAX_GRANTS - 1) {
            assert(reply_len == sizeof final_status);
            assert(memcmp(reply, final_status, reply_len) == 0);
        } else {
            assert(reply_len == 20);
            assert(rq_copy_completed_candidate(core, baseline,
                                               sizeof baseline) == -1);
        }
    }
    assert(rq_get_state(core) == RQ_COMPLETE);
    assert(rq_grants_issued(core) == RQ_MAX_GRANTS);
    assert(rq_bytes_received(core) == RQ_WRITE_BYTES);
    snapshot = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    assert(snapshot);
    assert(rq_copy_completed_candidate(core, snapshot, RQ_BASELINE_BYTES) == 0);
    for (size_t i = 0; i < RQ_WRITE_BYTES; ++i) assert(snapshot[i] == fill);
    for (size_t i = RQ_WRITE_BYTES; i < RQ_BASELINE_BYTES; ++i)
        assert(snapshot[i] == 0x5a);
    for (size_t i = 0; i < RQ_BASELINE_BYTES; ++i)
        assert(baseline[i] == 0x5a);
    snapshot[0] ^= 0xff;
    assert(rq_copy_completed_candidate(core, snapshot, RQ_BASELINE_BYTES) == 0);
    assert(snapshot[0] == fill); /* Detached output, not an alias. */
    free(snapshot);
}

static void test_pin_and_consent(void)
{
    uint8_t changed[32];
    struct rq_core *core;
    memset(baseline, 0x5a, sizeof baseline);
    assert(!rq_create(NULL, baseline, sizeof baseline, pinned));
    assert(!rq_create("wrong", baseline, sizeof baseline, pinned));
    assert(!rq_create(RQ_HOST_ACK, baseline, sizeof baseline - 1, pinned));
    memcpy(changed, pinned, sizeof changed);
    changed[0] ^= 1;
    assert(!rq_create(RQ_HOST_ACK, baseline, sizeof baseline, changed));
    core = rq_create(RQ_HOST_ACK, baseline, sizeof baseline, pinned);
    assert(core);
    baseline[0] ^= 1; /* Caller mutation does not alter private copy. */
    rq_destroy(core);
    puts("pin/consent: PASS");
}

static void test_complete(void)
{
    struct rq_core *core = new_core();
    complete(core, 0x11);
    rq_destroy(core);
    puts("95 chunks/final ACK: PASS");
}

static void test_order_and_frames(void)
{
    uint8_t reply[20], bad[24];
    size_t n;
    struct rq_core *core = new_core();
    assert(rq_accept(core, cmd3, sizeof cmd3, reply, 20, &n) == RQ_ERROR);
    assert(n == 0 && rq_get_state(core) == RQ_ABORTED);
    rq_destroy(core);

    core = new_core();
    memcpy(bad, cmd7, sizeof cmd7);
    bad[8] = 4;
    assert(rq_accept(core, bad, sizeof cmd7, reply, 20, &n) == RQ_ERROR);
    assert(!rq_candidate_prepared(core));
    rq_destroy(core);

    core = new_core();
    assert(rq_accept(core, cmd7, sizeof cmd7, reply, 20, &n) == RQ_REPLY);
    assert(rq_candidate_prepared(core));
    assert(rq_accept(core, cmd6, sizeof cmd6, reply, 20, &n) == RQ_ERROR);
    assert(!rq_candidate_prepared(core));
    rq_destroy(core);

    core = new_core();
    assert(rq_accept(core, cmd7, sizeof cmd7, reply, 20, &n) == RQ_REPLY);
    assert(rq_accept(core, cmd3, sizeof cmd3, reply, 20, &n) == RQ_NO_REPLY);
    memcpy(bad, cmd6, sizeof cmd6);
    bad[20] = 3;
    assert(rq_accept(core, bad, sizeof cmd6, reply, 20, &n) == RQ_ERROR);
    rq_destroy(core);
    puts("order/exact frames: PASS");
}

static void test_bad_data(void)
{
    uint8_t reply[20], frame[2033]; /* One extra byte for overflow fixture. */
    size_t n, frame_len;
    for (unsigned variant = 0; variant < 7; ++variant) {
        struct rq_core *core = new_core();
        through_grant(core, reply);
        frame_len = data_frame(frame, reply, 0x22);
        switch (variant) {
        case 0: put16(frame + 2, 2); break;
        case 1: put32(frame + 8, 1); break;
        case 2: put32(frame + 12, 4); break;
        case 3: put32(frame + 16, RQ_CHUNK_BYTES - 1); break;
        case 4: put32(frame + 4, 12u + RQ_CHUNK_BYTES - 1); break;
        case 5: --frame_len; break;
        case 6: frame[frame_len++] = 0; break;
        }
        assert(rq_accept(core, frame, frame_len, reply, 20, &n) == RQ_ERROR);
        assert(n == 0 && rq_get_state(core) == RQ_ABORTED);
        assert(!rq_candidate_prepared(core));
        rq_destroy(core);
    }
    puts("malformed data: PASS");
}

static void test_output_contract(void)
{
    uint8_t reply[20];
    size_t n = 99;
    struct rq_core *core = new_core();
    assert(rq_accept(core, cmd7, sizeof cmd7, reply, 19, &n) == RQ_ERROR);
    assert(n == 0 && rq_get_state(core) == RQ_ABORTED);
    assert(!rq_candidate_prepared(core));
    rq_destroy(core);

    core = new_core();
    assert(rq_accept(core, cmd7, sizeof cmd7, NULL, 20, &n) == RQ_ERROR);
    assert(n == 0 && rq_get_state(core) == RQ_ABORTED);
    rq_destroy(core);

    core = new_core();
    assert(rq_accept(core, cmd7, sizeof cmd7, reply, 20, NULL) == RQ_ERROR);
    assert(rq_get_state(core) == RQ_ABORTED);
    rq_destroy(core);
    puts("output contract: PASS");
}

static void test_failed_final(void)
{
    uint8_t reply[20], frame[2032];
    size_t n, frame_len;
    struct rq_core *core = new_core();
    through_grant(core, reply);
    for (unsigned i = 0; i < RQ_MAX_GRANTS - 1; ++i) {
        frame_len = data_frame(frame, reply, 0x33);
        assert(rq_accept(core, frame, frame_len, reply, 20, &n) == RQ_REPLY);
        assert(n == 20);
    }
    frame_len = data_frame(frame, reply, 0x33);
    put32(frame + 8, 1); /* CP-reported failure on the 95th chunk. */
    assert(rq_accept(core, frame, frame_len, reply, 20, &n) == RQ_ERROR);
    assert(n == 0 && rq_get_state(core) == RQ_ABORTED);
    assert(!rq_candidate_prepared(core));
    assert(rq_copy_completed_candidate(core, baseline, sizeof baseline) == -1);
    rq_destroy(core);
    puts("failed final/no ACK: PASS");
}

static void test_post_complete(void)
{
    uint8_t reply[20];
    size_t n;
    struct rq_core *core = new_core();
    complete(core, 0x44);
    assert(rq_accept(core, cmd7, sizeof cmd7, reply, 20, &n) == RQ_ERROR);
    assert(n == 0 && rq_get_state(core) == RQ_ABORTED);
    assert(!rq_candidate_prepared(core));
    rq_destroy(core);
    puts("post-completion frame: PASS");
}

static void test_fresh_candidate(void)
{
    struct rq_core *first = new_core();
    struct rq_core *second = new_core();
    uint8_t *a = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    uint8_t *b = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    assert(a && b);
    complete(first, 0x11);
    complete(second, 0x22);
    assert(rq_copy_completed_candidate(first, a, RQ_BASELINE_BYTES) == 0);
    assert(rq_copy_completed_candidate(second, b, RQ_BASELINE_BYTES) == 0);
    assert(a[0] != b[0] && a[RQ_WRITE_BYTES] == b[RQ_WRITE_BYTES]);
    free(a); free(b);
    rq_destroy(first); rq_destroy(second);
    puts("fresh candidate: PASS");
}

int main(void)
{
    test_sha_vectors();
    test_pin_and_consent();
    test_complete();
    test_order_and_frames();
    test_bad_data();
    test_output_contract();
    test_failed_final();
    test_post_complete();
    test_fresh_candidate();
    puts("host-only C fixtures: PASS");
    return 0;
}
