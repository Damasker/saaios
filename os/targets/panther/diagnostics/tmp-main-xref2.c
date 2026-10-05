/* Try multiple VA bases for MAIN string xrefs (ARM32 LE pointers). */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/stat.h>

#define MAIN_FILE_OFF 0x16c10u

struct hit { const char *name; uint32_t file_off; };

static unsigned scan_va(const uint8_t *img, size_t len, uint32_t va, const char *label) {
    uint8_t n[4] = {(uint8_t)va,(uint8_t)(va>>8),(uint8_t)(va>>16),(uint8_t)(va>>24)};
    unsigned found = 0;
    for (size_t o = 0; o + 4 <= len; o += 4) {
        if (img[o]==n[0]&&img[o+1]==n[1]&&img[o+2]==n[2]&&img[o+3]==n[3]) {
            printf("  HIT %s va=0x%08x @file+0x%zx\n", label, va, o);
            /* show ±8 words */
            size_t s = o > 32 ? o - 32 : 0;
            size_t e = o + 36 < len ? o + 36 : len;
            for (size_t p = s & ~3ull; p + 4 <= e; p += 4) {
                uint32_t w; memcpy(&w, img + p, 4);
                printf("   %c %08zx: %08x\n", p == o ? '*' : ' ', p, w);
            }
            if (++found >= 4) break;
        }
    }
    return found;
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "/data/saaios/bin/saaios-probe-b-modem.bin";
    int fd = open(path, O_RDONLY);
    if (fd < 0) { perror("open"); return 1; }
    struct stat st; fstat(fd, &st);
    size_t len = (size_t)st.st_size;
    uint8_t *img = mmap(NULL, len, PROT_READ, MAP_PRIVATE, fd, 0);
    if (img == MAP_FAILED) { perror("mmap"); return 1; }

    struct hit hits[] = {
        {"tx_app_state_s0", 0x4c2fb67},
        {"DetermineSimStatus", 0x4cfe8d3},
        {"pin_disabled_fcp", 0x4d15513},
        {"sitSetPin1_s0", 0x4c3398b},
        {"pin_action_not_req", 0x4bb3db3},
    };

    uint32_t bases[] = {
        0x40010000u, /* TOC m_off */
        0x00000000u, /* identity file=VA */
        0x41000000u,
        0x42000000u,
        0x44000000u,
        0x45000000u,
        0x50000000u,
        0x60000000u,
        0x80000000u,
        0xC0000000u,
    };

    for (unsigned hi = 0; hi < sizeof hits / sizeof hits[0]; hi++) {
        uint32_t fo = hits[hi].file_off;
        printf("\n==== %s file_off=0x%x ====\n", hits[hi].name, fo);
        /* Confirm string */
        if (fo + 8 < len)
            printf("  text: %.40s\n", (char *)img + fo);

        unsigned total = 0;
        for (unsigned bi = 0; bi < sizeof bases / sizeof bases[0]; bi++) {
            uint32_t va1 = bases[bi] + fo; /* file_off as offset from base 0 */
            uint32_t va2 = bases[bi] + (fo - MAIN_FILE_OFF); /* MAIN-relative */
            char lab[64];
            snprintf(lab, sizeof lab, "base=0x%x+file", bases[bi]);
            total += scan_va(img, len, va1, lab);
            snprintf(lab, sizeof lab, "base=0x%x+mainrel", bases[bi]);
            total += scan_va(img, len, va2, lab);
        }
        /* also try fo itself and fo|1 thumb */
        total += scan_va(img, len, fo, "va=file_off");
        total += scan_va(img, len, fo | 1u, "va=file_off|1");
        printf("  total_hits=%u\n", total);
    }

    munmap(img, len);
    close(fd);
    return 0;
}
