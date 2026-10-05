/* Find absolute refs to neighbor ptr near tx string; infer string VA base. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/stat.h>

int main(void) {
    const char *path = "/data/saaios/bin/saaios-probe-b-modem.bin";
    int fd = open(path, O_RDONLY);
    struct stat st; fstat(fd, &st);
    size_t len = (size_t)st.st_size;
    uint8_t *img = mmap(NULL, len, PROT_READ, MAP_PRIVATE, fd, 0);

    /* Neighbor word just before A[SIT_0_SIM] Tx SIM Status */
    uint32_t neigh = 0x410259f5u;
    uint32_t str_off = 0x4c2fb67u;
    printf("neighbor=0x%x str_file=0x%x\n", neigh, str_off);

    unsigned n_hits = 0;
    for (size_t o = 0; o + 4 <= len; o += 4) {
        uint32_t w; memcpy(&w, img + o, 4);
        if (w == neigh) {
            printf("REF neigh @file+0x%zx\n", o);
            if (++n_hits >= 12) break;
        }
    }
    printf("neigh_refs=%u\n", n_hits);

    /* Heuristic: if string VA = neigh + delta (delta from neigh word to string = 3 bytes? 
       neigh at file 0x4c2fb64, string at 0x4c2fb67, delta=+3 — unaligned, so neigh is NOT
       pointing at string; it's a separate field in a record. */

    /* Search PC-relative LDR style: for each 'ldr rX,[pc,#imm]' that lands on a word
       equal to possible string VAs — too heavy. Instead: find "sitSimRcmHandle.c" path
       and dump surrounding function names via strings -t nearby. */

    const char *keys[] = {
        "sitSimRcmHandle.c",
        "sitSimNsHandle.c",
        "Tx SIM Status(app_state",
        "changing PinStatus as PIN_DISABLED",
        "DetermineSimStatus",
        "PIN Action Not Required",
        "sitSetPin1Status",
        NULL
    };
    for (int i = 0; keys[i]; i++) {
        size_t klen = strlen(keys[i]);
        for (size_t o = 0; o + klen < len; o++) {
            if (memcmp(img + o, keys[i], klen) == 0) {
                printf("STR '%s' @0x%zx\n", keys[i], o);
                /* print preceding 16 bytes as potential descriptors */
                if (o >= 16) {
                    printf("  prev:");
                    for (int j = 16; j > 0; j--) printf(" %02x", img[o - j]);
                    puts("");
                }
                break;
            }
        }
    }

    /* Enumerate unique PinStatus / app_state related log format IDs:
       pattern  XX XX 00 00 yy yy yy 41 before 'A[' or '[' logs in SIT_0_SIM */
    unsigned records = 0;
    for (size_t o = 0x4c00000; o + 64 < 0x4d20000 && o + 64 < len; o++) {
        if (img[o]=='A' && img[o+1]=='[' && img[o+2]=='S' && img[o+3]=='I' &&
            img[o+4]=='T' && img[o+5]=='_' && img[o+6]=='0' && img[o+7]=='_' &&
            img[o+8]=='S' && img[o+9]=='I' && img[o+10]=='M') {
            /* look for app_state in this string */
            int has_app = 0, has_pin = 0;
            for (size_t k = 0; k < 120 && o + k < len; k++) {
                if (img[o+k]==0) break;
                if (k+9 < 120 && memcmp(img+o+k, "app_state", 9)==0) has_app = 1;
                if (k+10 < 120 && memcmp(img+o+k, "Pin1Status", 10)==0) has_pin = 1;
            }
            if (has_app || has_pin) {
                uint32_t a=0,b=0;
                if (o >= 8) { memcpy(&a, img+o-8, 4); memcpy(&b, img+o-4, 4); }
                printf("SIT0_SIM log @0x%zx prev=(0x%x,0x%x) app=%d pin=%d :: %.60s\n",
                    o, a, b, has_app, has_pin, (char*)img+o);
                if (++records >= 25) break;
            }
        }
    }

    munmap(img, len);
    close(fd);
    return 0;
}
