/* RO scan: find ARM32 LE pointers to key MAIN strings; print nearby code words. */
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/stat.h>

#define MAIN_FILE_OFF 0x16c10u
#define MAIN_VA       0x40010000u

struct hit {
    const char *name;
    uint32_t file_off; /* absolute in modem.bin */
};

static uint32_t file_to_va(uint32_t file_off) {
    return MAIN_VA + (file_off - MAIN_FILE_OFF);
}

static void dump_near(const uint8_t *main, uint32_t main_sz, uint32_t ptr_off, const char *tag) {
    /* ptr_off is offset within MAIN */
    uint32_t start = ptr_off > 64 ? ptr_off - 64 : 0;
    uint32_t end = ptr_off + 64;
    if (end > main_sz) end = main_sz;
    printf("  xref@MAIN+0x%x VA=0x%08x (%s)\n", ptr_off, MAIN_VA + ptr_off, tag);
    for (uint32_t o = start & ~3u; o + 4 <= end; o += 4) {
        uint32_t w;
        memcpy(&w, main + o, 4);
        char mark = (o == (ptr_off & ~3u)) ? '*' : ' ';
        printf("  %c MAIN+0x%08x: %08x\n", mark, o, w);
    }
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "/data/saaios/bin/saaios-probe-b-modem.bin";
    int fd = open(path, O_RDONLY);
    if (fd < 0) { perror("open"); return 1; }
    struct stat st;
    if (fstat(fd, &st)) { perror("fstat"); return 1; }
    size_t len = (size_t)st.st_size;
    uint8_t *map = mmap(NULL, len, PROT_READ, MAP_PRIVATE, fd, 0);
    if (map == MAP_FAILED) { perror("mmap"); return 1; }

    struct hit hits[] = {
        {"tx_app_state_s0", 0x4c2fb67},
        {"tx_app_state_s1", 0x4c2fb0f},
        {"sitSetPin1_s0", 0x4c3398b},
        {"DetermineSimStatus", 0x4cfe8d3},
        {"GetMultiPin1Status", 0x4d054c3},
        {"pin_disabled_fcp", 0x4d15513},
        {"pin1_enabled_notif", 0x4cf9feb},
        {"pin_skip_fail_state", 0x4d1391b},
        {"pin_action_not_req", 0x4bb3db3},
        {"ps_do_adm", 0x4d05be3},
    };
    const uint8_t *main = map + MAIN_FILE_OFF;
    uint32_t main_sz = 0x05917accu;
    if (MAIN_FILE_OFF + main_sz > len) main_sz = (uint32_t)(len - MAIN_FILE_OFF);

    printf("MAIN file_off=0x%x size=0x%x VA_base=0x%08x\n", MAIN_FILE_OFF, main_sz, MAIN_VA);

    for (unsigned i = 0; i < sizeof hits / sizeof hits[0]; i++) {
        if (hits[i].file_off < MAIN_FILE_OFF || hits[i].file_off >= MAIN_FILE_OFF + main_sz) {
            printf("SKIP %s off out of MAIN\n", hits[i].name);
            continue;
        }
        uint32_t va = file_to_va(hits[i].file_off);
        uint8_t needle[4] = {
            (uint8_t)va, (uint8_t)(va >> 8), (uint8_t)(va >> 16), (uint8_t)(va >> 24)
        };
        printf("\n== %s file=0x%x VA=0x%08x ==\n", hits[i].name, hits[i].file_off, va);
        unsigned found = 0;
        for (uint32_t o = 0; o + 4 <= main_sz; o++) {
            if (main[o] == needle[0] && main[o + 1] == needle[1] &&
                main[o + 2] == needle[2] && main[o + 3] == needle[3]) {
                dump_near(main, main_sz, o, hits[i].name);
                if (++found >= 6) break;
            }
        }
        printf("  ptr_hits=%u\n", found);
    }

    /* Also search for immediate stores of app_state=2 near DetermineSimStatus string refs —
       scan for literal pools already done. Look for ASCII enum table near tx string. */
    uint32_t tx_off = 0x4c2fb67 - MAIN_FILE_OFF;
    printf("\n== bytes around tx_app_state string (no secrets) ==\n");
    /* print only format string length confirm */
    printf("  strlen_approx check first chars: %c%c%c%c\n",
        map[0x4c2fb67], map[0x4c2fb68], map[0x4c2fb69], map[0x4c2fb6a]);

    (void)tx_off;
    munmap(map, len);
    close(fd);
    return 0;
}
