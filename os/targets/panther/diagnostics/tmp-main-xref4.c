/* Scan for ARM imm 0x347 (log id near Tx SIM Status) and DetermineSimStatus context. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/stat.h>

/* movw/movt encoding helpers — print candidate sites with movw #0x347 */
static int is_movw_imm(uint32_t insn, uint32_t *imm, uint32_t *rd) {
    /* A1: 1110 0011 0000 iiii rrrr iiii iiii iiii  encoding MOVW */
    if ((insn & 0x0ff00000u) != 0x03000000u) return 0;
    if (((insn >> 20) & 0xf) != 0x0) return 0; /* actually check full */
    /* ARM MOVW: cond 0011 0000 imm4 Rd imm12 — opcode 0x30 for movw when bits */
    /* Better: (insn & 0x0ff00000) == 0x03000000 for MOVW */
    uint32_t imm12 = insn & 0xfff;
    uint32_t imm4 = (insn >> 16) & 0xf;
    *imm = (imm4 << 12) | imm12;
    *rd = (insn >> 12) & 0xf;
    return 1;
}

static int is_movw(uint32_t insn, uint32_t *imm, uint32_t *rd) {
    /* cond|00110000|imm4|Rd|imm12 */
    if ((insn & 0x0ff00000u) != 0x03000000u) return 0;
    *imm = ((insn & 0xf0000u) >> 4) | (insn & 0xfffu);
    *rd = (insn >> 12) & 0xf;
    return 1;
}

int main(void) {
    const char *path = "/data/saaios/bin/saaios-probe-b-modem.bin";
    int fd = open(path, O_RDONLY);
    struct stat st; fstat(fd, &st);
    size_t len = (size_t)st.st_size;
    uint8_t *img = mmap(NULL, len, PROT_READ, MAP_PRIVATE, fd, 0);

    /* 1) Find log-id words 0x00000347 immediately before Tx format (confirm) */
    printf("word@0x4c2fb60 = 0x%08x\n", *(uint32_t*)(img+0x4c2fb60));
    printf("word@0x4c2fb64 = 0x%08x\n", *(uint32_t*)(img+0x4c2fb64));

    /* 2) MOVW #0x347 sites in MAIN (0x16c10 .. end) */
    size_t main0 = 0x16c10, main1 = main0 + 0x5917acc;
    if (main1 > len) main1 = len;
    unsigned movw_hits = 0;
    printf("\n== MOVW #0x347 in MAIN ==\n");
    for (size_t o = main0; o + 4 <= main1; o += 4) {
        uint32_t insn; memcpy(&insn, img+o, 4);
        uint32_t imm=0, rd=0;
        if (is_movw(insn, &imm, &rd) && imm == 0x347) {
            printf("MOVW r%u,#0x347 @file+0x%zx insn=%08x\n", rd, o, insn);
            /* dump next 8 insns */
            for (int k = 0; k < 8; k++) {
                uint32_t w; memcpy(&w, img+o+4+k*4, 4);
                printf("  +%d: %08x\n", (k+1)*4, w);
            }
            if (++movw_hits >= 15) break;
        }
    }
    printf("movw_hits=%u\n", movw_hits);

    /* 3) literal pool 0x00000347 */
    unsigned lit = 0;
    printf("\n== literal .word 0x347 near code (sample) ==\n");
    for (size_t o = main0; o + 4 <= main1; o += 4) {
        uint32_t w; memcpy(&w, img+o, 4);
        if (w != 0x347) continue;
        /* require nearby ARM insn pattern (cond bits not 0xf und) */
        int near_code = 0;
        for (int d = -16; d <= 16; d += 4) {
            if (d == 0) continue;
            size_t p = o + (size_t)d;
            if (p < main0 || p + 4 > main1) continue;
            uint32_t insn; memcpy(&insn, img+p, 4);
            uint32_t cond = insn >> 28;
            if (cond <= 0xe && (insn & 0x0e000000) != 0) { near_code = 1; break; }
        }
        if (!near_code) continue;
        printf("lit 0x347 @file+0x%zx\n", o);
        if (++lit >= 20) break;
    }

    /* 4) Surround DetermineSimStatus: look for function entry by scanning
       back for STMFD/PUSH signature from string - 0x4cfe8d3 */
    size_t ds = 0x4cfe8d3;
    printf("\n== context words before DetermineSimStatus string ==\n");
    for (size_t o = ds - 32; o < ds; o += 4) {
        uint32_t w; memcpy(&w, img+o, 4);
        printf("  0x%zx: %08x\n", o, w);
    }

    /* 5) Search unique fmt "Pin Status Disabled in APP FCP" id word */
    size_t pdis = 0x4d15513;
    printf("\n== pin_disabled_fcp record header ==\n");
    for (size_t o = pdis - 16; o < pdis + 4; o += 4) {
        uint32_t w; memcpy(&w, img+o, 4);
        printf("  0x%zx: %08x\n", o, w);
    }

    /* 6) Cross: search MOVW of pin_disabled log id */
    uint32_t pin_id = 0;
    memcpy(&pin_id, img + (pdis - 8), 4);
    /* try pdis-8 and nearby aligned */
    for (int back = 4; back <= 16; back += 4) {
        uint32_t cand; memcpy(&cand, img + pdis - back, 4);
        if (cand > 0 && cand < 0x10000) {
            printf("candidate log_id=0x%x at -%d\n", cand, back);
            unsigned h = 0;
            for (size_t o = main0; o + 4 <= main1; o += 4) {
                uint32_t insn; memcpy(&insn, img+o, 4);
                uint32_t imm=0, rd=0;
                if (is_movw(insn, &imm, &rd) && imm == cand) {
                    printf("  MOVW r%u,#0x%x @0x%zx\n", rd, cand, o);
                    if (++h >= 8) break;
                }
            }
        }
    }

    munmap(img, len);
    close(fd);
    return 0;
}
