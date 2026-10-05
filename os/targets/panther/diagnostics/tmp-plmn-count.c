/* Count PLMN / serving-cell markers in a read-only NV image.
 * Prints counts and offsets only. Never prints payload bytes. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { MAX_HITS = 48 };

struct hit {
    uint32_t off;
    unsigned plmn; /* 0..99 for 25500+plmn */
    unsigned kind; /* 1 ascii, 2 bcd */
};

static int is_digit(uint8_t c)
{
    return c >= '0' && c <= '9';
}

static void note(struct hit *hits, unsigned *n, uint32_t off, unsigned plmn,
                 unsigned kind)
{
    if (*n >= MAX_HITS) return;
    hits[*n].off = off;
    hits[*n].plmn = plmn;
    hits[*n].kind = kind;
    (*n)++;
}

static unsigned clusters(const struct hit *hits, unsigned n)
{
    unsigned i, c = 0;
    for (i = 0; i < n; i++) {
        unsigned j, near = 0;
        for (j = 0; j < n; j++) {
            uint32_t d = hits[i].off > hits[j].off ? hits[i].off - hits[j].off
                                                   : hits[j].off - hits[i].off;
            if (d <= 96 && hits[i].plmn != hits[j].plmn) near = 1;
        }
        if (near) c++;
    }
    return c;
}

static int scan_file(const char *path)
{
    FILE *f = fopen(path, "rb");
    unsigned char *buf;
    long sz, i;
    unsigned ascii[100];
    unsigned bcd[100];
    unsigned cid_a = 0, cid_b = 0;
    unsigned tok_plmn = 0, tok_acq = 0, tok_earf = 0;
    struct hit hits[MAX_HITS];
    unsigned nh = 0;
    const uint32_t cid0 = 85791486u;
    const uint32_t cid1 = 85793345u;

    if (!f) {
        printf("open_fail\n");
        return 1;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return 1;
    }
    sz = ftell(f);
    if (sz < 16 || sz > 1024 * 1024) {
        printf("size_reject %ld\n", sz);
        fclose(f);
        return 1;
    }
    if (fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return 1;
    }
    buf = malloc((size_t)sz);
    if (!buf || fread(buf, 1, (size_t)sz, f) != (size_t)sz) {
        free(buf);
        fclose(f);
        printf("read_fail\n");
        return 1;
    }
    fclose(f);
    memset(ascii, 0, sizeof ascii);
    memset(bcd, 0, sizeof bcd);

    for (i = 0; i + 5 <= sz; i++) {
        unsigned plmn;
        if (buf[i] == '2' && buf[i + 1] == '5' && buf[i + 2] == '5' &&
            is_digit(buf[i + 3]) && is_digit(buf[i + 4])) {
            plmn = (unsigned)(buf[i + 3] - '0') * 10u +
                   (unsigned)(buf[i + 4] - '0');
            ascii[plmn]++;
            note(hits, &nh, (uint32_t)i, plmn, 1);
        }
        if (buf[i] == 0x52 && buf[i + 1] == 0xF5) {
            uint8_t m = buf[i + 2];
            unsigned d1 = m & 0x0f;
            unsigned d2 = m >> 4;
            if (d1 <= 9 && d2 <= 9) {
                plmn = d1 * 10u + d2;
                bcd[plmn]++;
                note(hits, &nh, (uint32_t)i, plmn, 2);
            }
        }
        if (i + 4 <= sz) {
            uint32_t w = (uint32_t)buf[i] | ((uint32_t)buf[i + 1] << 8) |
                         ((uint32_t)buf[i + 2] << 16) | ((uint32_t)buf[i + 3] << 24);
            if (w == cid0) cid_a++;
            if (w == cid1) cid_b++;
        }
    }
    for (i = 0; i + 4 <= sz; i++) {
        if (memcmp(buf + i, "PLMN", 4) == 0) tok_plmn++;
        if (i + 5 <= sz && memcmp(buf + i, "ACQDB", 5) == 0) tok_acq++;
        if (i + 6 <= sz && memcmp(buf + i, "EARFCN", 6) == 0) tok_earf++;
    }
    printf("size=%ld\n", sz);
    printf("ascii");
    for (i = 0; i < 100; i++)
        if (ascii[i]) printf(" 255%02u=%u", (unsigned)i, ascii[i]);
    if (ascii[1] + ascii[2] + ascii[3] + ascii[6] + ascii[7] == 0 &&
        !ascii[1]) {
        /* still show the known operators even when zero */
    }
    printf("\n");
    printf("ascii_known 25501=%u 25503=%u 25506=%u\n", ascii[1], ascii[3],
           ascii[6]);
    printf("bcd");
    for (i = 0; i < 100; i++)
        if (bcd[i]) printf(" 255%02u=%u", (unsigned)i, bcd[i]);
    printf("\n");
    printf("bcd_known 25501=%u 25503=%u 25506=%u\n", bcd[1], bcd[3], bcd[6]);
    printf("cid 85791486=%u 85793345=%u\n", cid_a, cid_b);
    printf("token PLMN=%u ACQDB=%u EARFCN=%u\n", tok_plmn, tok_acq, tok_earf);
    printf("mixed_plmn_pairs=%u hit_slots=%u\n", clusters(hits, nh), nh);
    printf("other");
    {
        unsigned printed = 0;
        for (i = 0; i < (long)nh; i++) {
            if (hits[i].plmn == 1) continue;
            printf(" 255%02u@0x%x", hits[i].plmn, hits[i].off);
            printed++;
            if (printed >= 12) break;
        }
        if (!printed) printf(" none");
    }
    printf("\n");
    printf("shape");
    for (i = 0; i < (long)nh; i++) {
        uint32_t off;
        unsigned run, zbefore;
        int after;
        if (hits[i].kind != 1 || hits[i].plmn == 1) continue;
        off = hits[i].off;
        run = 0;
        while (off + run < (uint32_t)sz && is_digit(buf[off + run]) && run < 12)
            run++;
        after = (off + run < (uint32_t)sz) ? buf[off + run] : -1;
        zbefore = 0;
        while (zbefore < 16 && off >= zbefore + 1 && buf[off - 1 - zbefore] == 0)
            zbefore++;
        printf(" 255%02u digits=%u", hits[i].plmn, run);
        if (after == '#') printf(" after=hash");
        else if (after == 0) printf(" after=zero");
        else if (after >= '0' && after <= '9') printf(" after=digit");
        else printf(" after=other");
        printf(" zeros_before=%u", zbefore);
    }
    printf("\n");
    free(buf);
    return 0;
}

int main(int argc, char **argv)
{
    int a, rc = 0;
    if (argc < 2) return 2;
    for (a = 1; a < argc; a++) {
        printf("file_index=%d\n", a);
        if (scan_file(argv[a])) rc = 1;
    }
    return rc;
}
