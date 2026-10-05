/* Dual-handle RFS NV quarantine store. See rfs-nv-store.h for the contract. */
#define _GNU_SOURCE
#include "rfs-nv-store.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

enum { IO_SLICE = 4096, SIDECAR_HEX = 64 };

static const char *const seed_name[NV_KIND_COUNT] = {
    "seed-normal.bin", "seed-protected.bin"
};
static const char *const staging_name[NV_KIND_COUNT] = {
    "staging-normal.bin", "staging-protected.bin"
};
static const char *const normal_current = "normal-current.bin";
static const char *const normal_prev = "normal-prev.bin";
static const char *const protected_capture = "protected-capture.bin";
static const uint32_t kind_handle[NV_KIND_COUNT] = {
    NV_HANDLE_NORMAL, NV_HANDLE_PROTECTED
};

static void wipe(void *p, size_t n)
{
    volatile uint8_t *v = p;
    while (n--) *v++ = 0;
}

static uint16_t le16(const uint8_t *p) { return (uint16_t)(p[0] | p[1] << 8); }

static uint32_t le32(const uint8_t *p)
{
    return (uint32_t)p[0] | (uint32_t)p[1] << 8 | (uint32_t)p[2] << 16 |
           (uint32_t)p[3] << 24;
}

static void put32(uint8_t *p, uint32_t v)
{
    p[0] = (uint8_t)v;
    p[1] = (uint8_t)(v >> 8);
    p[2] = (uint8_t)(v >> 16);
    p[3] = (uint8_t)(v >> 24);
}

/* ---- SHA-256 (FIPS 180-4) ---- */

static const uint32_t sha_k[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

static uint32_t ror(uint32_t x, unsigned n) { return x >> n | x << (32 - n); }

static void sha_block(struct nv_sha256 *c, const uint8_t b[64])
{
    uint32_t w[64], s[8];
    for (int i = 0; i < 16; ++i)
        w[i] = (uint32_t)b[4 * i] << 24 | (uint32_t)b[4 * i + 1] << 16 |
               (uint32_t)b[4 * i + 2] << 8 | b[4 * i + 3];
    for (int i = 16; i < 64; ++i)
        w[i] = w[i - 16] + (ror(w[i - 15], 7) ^ ror(w[i - 15], 18) ^
                            w[i - 15] >> 3) +
               w[i - 7] + (ror(w[i - 2], 17) ^ ror(w[i - 2], 19) ^
                           w[i - 2] >> 10);
    memcpy(s, c->h, sizeof s);
    for (int i = 0; i < 64; ++i) {
        uint32_t t1 = s[7] + (ror(s[4], 6) ^ ror(s[4], 11) ^ ror(s[4], 25)) +
                      ((s[4] & s[5]) ^ (~s[4] & s[6])) + sha_k[i] + w[i];
        uint32_t t2 = (ror(s[0], 2) ^ ror(s[0], 13) ^ ror(s[0], 22)) +
                      ((s[0] & s[1]) ^ (s[0] & s[2]) ^ (s[1] & s[2]));
        memmove(s + 1, s, 7 * sizeof s[0]);
        s[4] += t1;
        s[0] = t1 + t2;
    }
    for (int i = 0; i < 8; ++i) c->h[i] += s[i];
    wipe(w, sizeof w);
    wipe(s, sizeof s);
}

void nv_sha256_init(struct nv_sha256 *c)
{
    static const uint32_t iv[8] = {
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
    };
    memcpy(c->h, iv, sizeof iv);
    c->bits = 0;
    c->used = 0;
}

void nv_sha256_update(struct nv_sha256 *c, const void *data, size_t len)
{
    const uint8_t *p = data;
    c->bits += (uint64_t)len * 8;
    while (len) {
        size_t take = 64 - c->used < len ? 64 - c->used : len;
        memcpy(c->block + c->used, p, take);
        c->used += take;
        p += take;
        len -= take;
        if (c->used == 64) {
            sha_block(c, c->block);
            c->used = 0;
        }
    }
}

void nv_sha256_final(struct nv_sha256 *c, uint8_t out[32])
{
    uint64_t bits = c->bits;
    uint8_t pad = 0x80, zero = 0, len[8];
    nv_sha256_update(c, &pad, 1);
    while (c->used != 56) nv_sha256_update(c, &zero, 1);
    for (int i = 0; i < 8; ++i) len[i] = (uint8_t)(bits >> (56 - 8 * i));
    nv_sha256_update(c, len, 8);
    for (int i = 0; i < 8; ++i) {
        out[4 * i] = (uint8_t)(c->h[i] >> 24);
        out[4 * i + 1] = (uint8_t)(c->h[i] >> 16);
        out[4 * i + 2] = (uint8_t)(c->h[i] >> 8);
        out[4 * i + 3] = (uint8_t)c->h[i];
    }
    wipe(c, sizeof *c);
}

/* ---- file helpers ---- */

static int read_at(int fd, void *out, size_t len, off_t off)
{
    uint8_t *p = out;
    while (len) {
        ssize_t n = pread(fd, p, len, off);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return -1;
        p += n;
        len -= (size_t)n;
        off += n;
    }
    return 0;
}

static int write_at(int fd, const void *in, size_t len, off_t off)
{
    const uint8_t *p = in;
    while (len) {
        ssize_t n = pwrite(fd, p, len, off);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return -1;
        p += n;
        len -= (size_t)n;
        off += n;
    }
    return 0;
}

static int sidecar_for(const char *image, char out[64])
{
    size_t n = strlen(image);
    if (n < 5 || n > 48 || strcmp(image + n - 4, ".bin")) return -1;
    snprintf(out, 64, "%.*s.sha256", (int)(n - 4), image);
    return 0;
}

static int digest_fd(int fd, uint8_t digest[32])
{
    uint8_t buf[IO_SLICE];
    struct nv_sha256 c;
    nv_sha256_init(&c);
    for (off_t off = 0; off < NV_IMAGE_BYTES; off += IO_SLICE) {
        if (read_at(fd, buf, IO_SLICE, off)) {
            wipe(buf, sizeof buf);
            wipe(&c, sizeof c);
            return -1;
        }
        nv_sha256_update(&c, buf, IO_SLICE);
    }
    nv_sha256_final(&c, digest);
    wipe(buf, sizeof buf);
    return 0;
}

static int write_sidecar(int store, const char *image, const uint8_t d[32])
{
    static const char hex[] = "0123456789abcdef";
    char name[64], text[SIDECAR_HEX];
    if (sidecar_for(image, name)) return -1;
    for (int i = 0; i < 32; ++i) {
        text[2 * i] = hex[d[i] >> 4];
        text[2 * i + 1] = hex[d[i] & 15];
    }
    (void)unlinkat(store, name, 0);
    int fd = openat(store, name,
                    O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
    int rc = fd < 0 || write_at(fd, text, sizeof text, 0) || fsync(fd) ? -1 : 0;
    if (fd >= 0 && close(fd)) rc = -1;
    wipe(text, sizeof text);
    return rc;
}

static int sidecar_matches(int store, const char *image, const uint8_t d[32])
{
    char name[64], text[SIDECAR_HEX + 1];
    if (sidecar_for(image, name)) return 0;
    int fd = openat(store, name, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return 0;
    struct stat st;
    int ok = !fstat(fd, &st) && S_ISREG(st.st_mode) &&
             st.st_size == SIDECAR_HEX && !read_at(fd, text, SIDECAR_HEX, 0);
    close(fd);
    for (int i = 0; ok && i < 32; ++i) {
        unsigned v = 0;
        for (int j = 0; j < 2; ++j) {
            char ch = text[2 * i + j];
            v <<= 4;
            if (ch >= '0' && ch <= '9') v |= (unsigned)(ch - '0');
            else if (ch >= 'a' && ch <= 'f') v |= (unsigned)(ch - 'a' + 10);
            else ok = 0;
        }
        if (v != d[i]) ok = 0;
    }
    wipe(text, sizeof text);
    return ok;
}

static int fsync_dir(int store) { return fsync(store); }

/* ---- image checks ---- */

const char *nv_check_name(enum nv_check check)
{
    static const char *const names[] = {
        "ok", "io", "size", "all_zero", "no_erig_magic", "sidecar_mismatch"
    };
    return (unsigned)check < sizeof names / sizeof names[0] ?
           names[check] : "unknown";
}

const char *nv_kind_name(enum nv_kind kind)
{
    return kind == NV_KIND_NORMAL ? "normal" :
           kind == NV_KIND_PROTECTED ? "protected" : "unknown";
}

enum nv_check nv_image_check_fd(int fd, enum nv_kind kind)
{
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISREG(st.st_mode)) return NV_CHECK_IO;
    if (st.st_size != NV_IMAGE_BYTES) return NV_CHECK_SIZE;
    uint8_t buf[IO_SLICE];
    int nonzero = 0, magic = 0;
    for (off_t off = 0; off < NV_IMAGE_BYTES; off += IO_SLICE) {
        if (read_at(fd, buf, IO_SLICE, off)) {
            wipe(buf, sizeof buf);
            return NV_CHECK_IO;
        }
        if (off == 0)
            for (int i = 0; i + 4 <= 64; ++i)
                if (!memcmp(buf + i, "ERIG", 4)) magic = 1;
        for (size_t i = 0; !nonzero && i < IO_SLICE; ++i)
            if (buf[i]) nonzero = 1;
    }
    wipe(buf, sizeof buf);
    if (!nonzero) return NV_CHECK_ZERO;
    if (kind == NV_KIND_NORMAL && !magic) return NV_CHECK_MAGIC;
    return NV_CHECK_OK;
}

enum nv_check nv_store_check(int store, const char *name, enum nv_kind kind)
{
    int fd = openat(store, name, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return NV_CHECK_IO;
    enum nv_check check = nv_image_check_fd(fd, kind);
    uint8_t d[32];
    if (check == NV_CHECK_OK)
        check = digest_fd(fd, d) ? NV_CHECK_IO :
                sidecar_matches(store, name, d) ? NV_CHECK_OK :
                NV_CHECK_SIDECAR;
    close(fd);
    wipe(d, sizeof d);
    return check;
}

int nv_store_seed(int store, enum nv_kind kind, int source_fd)
{
    if (kind < 0 || kind >= NV_KIND_COUNT ||
        nv_image_check_fd(source_fd, kind) != NV_CHECK_OK) return -1;
    const char *name = seed_name[kind];
    int fd = openat(store, name,
                    O_RDWR | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (fd < 0) return -1;
    uint8_t buf[IO_SLICE], d[32];
    int rc = 0;
    for (off_t off = 0; !rc && off < NV_IMAGE_BYTES; off += IO_SLICE)
        rc = read_at(source_fd, buf, IO_SLICE, off) ||
             write_at(fd, buf, IO_SLICE, off);
    wipe(buf, sizeof buf);
    if (!rc) rc = fsync(fd) || nv_image_check_fd(fd, kind) != NV_CHECK_OK ||
                  digest_fd(fd, d) || write_sidecar(store, name, d) ||
                  fchmod(fd, 0400) ? -1 : 0;
    wipe(d, sizeof d);
    if (close(fd)) rc = -1;
    if (rc) (void)unlinkat(store, name, 0);
    return rc || fsync_dir(store) ? -1 : 0;
}

int nv_store_boot_image(int store, enum nv_kind kind, const char **name)
{
    const char *normal[] = { normal_current, normal_prev,
                             seed_name[NV_KIND_NORMAL] };
    const char *protected_only[] = { seed_name[NV_KIND_PROTECTED] };
    const char *const *order = kind == NV_KIND_NORMAL ? normal : protected_only;
    size_t count = kind == NV_KIND_NORMAL ? 3 : 1;
    if (kind < 0 || kind >= NV_KIND_COUNT) return -1;
    for (size_t i = 0; i < count; ++i) {
        if (nv_store_check(store, order[i], kind) == NV_CHECK_OK) {
            if (name) *name = order[i];
            return 0;
        }
    }
    return -1;
}

/* ---- RFS server ---- */

void rfs_nv_server_init(struct rfs_nv_server *s, int store)
{
    memset(s, 0, sizeof *s);
    s->store = store;
    s->final_status[NV_KIND_PROTECTED] = 1;
    for (int k = 0; k < NV_KIND_COUNT; ++k) s->session[k].candidate = -1;
}

static void abort_session(struct rfs_nv_server *s, enum nv_kind kind)
{
    struct rfs_nv_session *x = &s->session[kind];
    if (x->candidate >= 0) close(x->candidate);
    char side[64];
    if (x->active || x->candidate >= 0) {
        (void)unlinkat(s->store, staging_name[kind], 0);
        if (!sidecar_for(staging_name[kind], side))
            (void)unlinkat(s->store, side, 0);
    }
    memset(x, 0, sizeof *x);
    x->candidate = -1;
}

void rfs_nv_server_abort(struct rfs_nv_server *s)
{
    for (int k = 0; k < NV_KIND_COUNT; ++k) abort_session(s, (enum nv_kind)k);
}

static int kind_of(uint32_t handle)
{
    return handle == NV_HANDLE_NORMAL ? NV_KIND_NORMAL :
           handle == NV_HANDLE_PROTECTED ? NV_KIND_PROTECTED : -1;
}

static size_t make_grant(uint8_t r[RFS_NV_REPLY_MAX],
                         const struct rfs_nv_session *x, uint32_t handle)
{
    put32(r, 2u | (uint32_t)x->seq << 16);
    put32(r + 4, 12);
    put32(r + 8, handle);
    put32(r + 12, x->received);
    put32(r + 16, x->expected);
    return 20;
}

static size_t make_status(uint8_t r[RFS_NV_REPLY_MAX], uint16_t seq,
                          uint32_t handle)
{
    put32(r, 3u | (uint32_t)seq << 16);
    put32(r + 4, 8);
    put32(r + 8, 0);
    put32(r + 12, handle);
    return 16;
}

static uint32_t next_len(const struct rfs_nv_session *x)
{
    uint32_t remain = x->total - x->received;
    return remain < NV_CHUNK_MAX ? remain : NV_CHUNK_MAX;
}

static enum rfs_nv_result reject(struct rfs_nv_server *s, enum nv_kind kind)
{
    abort_session(s, kind);
    s->rejected[kind]++;
    return RFS_NV_REJECTED;
}

static int begin(struct rfs_nv_server *s, enum nv_kind kind, uint16_t seq,
                 uint32_t total)
{
    abort_session(s, kind);
    struct rfs_nv_session *x = &s->session[kind];
    const char *base = NULL;
    if (nv_store_boot_image(s->store, kind, &base)) return -1;
    int src = openat(s->store, base, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    (void)unlinkat(s->store, staging_name[kind], 0);
    int fd = openat(s->store, staging_name[kind],
                    O_RDWR | O_CREAT | O_EXCL | O_CLOEXEC | O_NOFOLLOW, 0600);
    uint8_t buf[IO_SLICE];
    int rc = src < 0 || fd < 0;
    for (off_t off = 0; !rc && off < NV_IMAGE_BYTES; off += IO_SLICE)
        rc = read_at(src, buf, IO_SLICE, off) || write_at(fd, buf, IO_SLICE, off);
    wipe(buf, sizeof buf);
    if (src >= 0) close(src);
    if (!rc) rc = fsync(fd);
    x->candidate = fd;
    if (rc) {
        x->active = 1;
        return -1;
    }
    x->active = 1;
    x->seq = seq;
    x->total = total;
    x->received = 0;
    x->base = base;
    x->expected = next_len(x);
    return 0;
}

static int tail_matches_base(int store, const struct rfs_nv_session *x)
{
    int base = openat(store, x->base, O_RDONLY | O_CLOEXEC | O_NOFOLLOW);
    if (base < 0) return 0;
    uint8_t a[IO_SLICE / 2], b[IO_SLICE / 2];
    int ok = 1;
    for (off_t off = x->total; ok && off < NV_IMAGE_BYTES;) {
        size_t n = (size_t)(NV_IMAGE_BYTES - off);
        if (n > sizeof a) n = sizeof a;
        ok = !read_at(base, a, n, off) && !read_at(x->candidate, b, n, off) &&
             !memcmp(a, b, n);
        off += (off_t)n;
    }
    close(base);
    wipe(a, sizeof a);
    wipe(b, sizeof b);
    return ok;
}

static int rename_pair(int store, const char *from, const char *to)
{
    char fs[64], ts[64];
    if (sidecar_for(from, fs) || sidecar_for(to, ts)) return -1;
    return renameat(store, from, store, to) ||
           renameat(store, fs, store, ts) ? -1 : 0;
}

static int commit(struct rfs_nv_server *s, enum nv_kind kind)
{
    struct rfs_nv_session *x = &s->session[kind];
    uint8_t d[32];
    int rc = !tail_matches_base(s->store, x) ||
             nv_image_check_fd(x->candidate, kind) != NV_CHECK_OK ||
             digest_fd(x->candidate, d) ||
             write_sidecar(s->store, staging_name[kind], d);
    wipe(d, sizeof d);
    if (close(x->candidate)) rc = 1;
    x->candidate = -1;
    if (rc) return -1;
    if (kind == NV_KIND_NORMAL) {
        struct stat st;
        if (!fstatat(s->store, normal_current, &st, AT_SYMLINK_NOFOLLOW) &&
            rename_pair(s->store, normal_current, normal_prev)) return -1;
        if (rename_pair(s->store, staging_name[kind], normal_current))
            return -1;
    } else if (rename_pair(s->store, staging_name[kind], protected_capture)) {
        return -1;
    }
    return fsync_dir(s->store) ? -1 : 0;
}

static enum rfs_nv_result store_chunk(struct rfs_nv_server *s,
                                      enum nv_kind kind, const uint8_t *frame,
                                      size_t len, uint8_t *reply,
                                      size_t *reply_len)
{
    struct rfs_nv_session *x = &s->session[kind];
    uint32_t paylen = le32(frame + 4), chunk = le32(frame + 16);
    int last = x->active && x->received + chunk == x->total;
    int exact = paylen == 12u + chunk && len == 20u + chunk;
    /* The proven final protected-NV frame carries two zero bytes after the
     * granted payload; accept only that shape and only on the last chunk. */
    int padded = last && paylen == 14u + chunk && len == 22u + chunk &&
                 !frame[20 + chunk] && !frame[21 + chunk];
    if (!x->active || le16(frame + 2) != x->seq || le32(frame + 8) != 0 ||
        chunk == 0 || chunk != x->expected || !(exact || padded))
        return reject(s, kind);
    uint8_t check[NV_CHUNK_MAX];
    int bad = write_at(x->candidate, frame + 20, chunk, x->received) ||
              fsync(x->candidate) ||
              read_at(x->candidate, check, chunk, x->received) ||
              memcmp(check, frame + 20, chunk);
    wipe(check, sizeof check);
    if (bad) return reject(s, kind);
    x->received += chunk;
    x->grants++;
    if (x->received < x->total) {
        x->expected = next_len(x);
        *reply_len = make_grant(reply, x, kind_handle[kind]);
        return RFS_NV_REPLY;
    }
    uint16_t seq = x->seq;
    if (commit(s, kind)) return reject(s, kind);
    memset(x, 0, sizeof *x);
    x->candidate = -1;
    s->committed[kind]++;
    if (s->final_status[kind])
        *reply_len = make_status(reply, seq, kind_handle[kind]);
    return RFS_NV_COMMITTED;
}

enum rfs_nv_result rfs_nv_frame(struct rfs_nv_server *s, const uint8_t *frame,
                                size_t len, uint8_t reply[RFS_NV_REPLY_MAX],
                                size_t *reply_len)
{
    *reply_len = 0;
    if (!frame || len < 12 || le32(frame + 4) + 8u != len) return RFS_NV_IGNORED;
    uint16_t cmd = le16(frame), seq = le16(frame + 2);
    int kind;
    switch (cmd) {
    case 7:
        if (len != 12 || (kind = kind_of(le32(frame + 8))) < 0)
            return RFS_NV_IGNORED;
        *reply_len = make_status(reply, 0, kind_handle[kind]);
        return RFS_NV_REPLY;
    case 3:
        if (len != 20 || kind_of(le32(frame + 12)) < 0) return RFS_NV_IGNORED;
        return RFS_NV_NO_REPLY;
    case 6: {
        if (len != 24 || (kind = kind_of(le32(frame + 8))) < 0)
            return RFS_NV_IGNORED;
        uint32_t offset = le32(frame + 12), total = le32(frame + 16);
        if (offset != 0 || total == 0 || total > NV_IMAGE_BYTES ||
            begin(s, (enum nv_kind)kind, seq, total))
            return reject(s, (enum nv_kind)kind);
        *reply_len = make_grant(reply, &s->session[kind], kind_handle[kind]);
        return RFS_NV_REPLY;
    }
    case 2:
        if (len < 20 || (kind = kind_of(le32(frame + 12))) < 0)
            return RFS_NV_IGNORED;
        return store_chunk(s, (enum nv_kind)kind, frame, len, reply, reply_len);
    default:
        return RFS_NV_IGNORED;
    }
}
