/* One-shot OEM IPC inject: write an operator-supplied frame to /dev/oem_ipcN.
 *
 * Does NOT invent catalog SIM_INIT_REQ (0x2f50) bytes. No default payload.
 * Frame must come from an external capture / dump (see OEM-IPC-CAPTURE.md).
 * Never starts cbd/rild. Logs lengths and errno only — no frame body dump.
 *
 * Build (static ARM64 for device):
 *   aarch64-linux-gnu-gcc -O2 -static -Wall -Wextra -Werror \
 *     oem-ipc-inject.c -o oem-ipc-inject
 * Host parser test:
 *   gcc -std=c11 -Wall -Wextra -Werror oem-ipc-inject.c -o /tmp/oem-ipc-inject
 *   /tmp/oem-ipc-inject self-test
 */
#define _GNU_SOURCE
#include <ctype.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#if defined(__linux__)
#include <sys/sysmacros.h>
#endif

enum { FRAME_MAX = 4096 };

static int hex_nibble(int c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/* Parse raw bytes, or hex text (whitespace / commas / optional 0x). */
static int load_frame(const char *path, uint8_t *out, size_t cap, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    uint8_t raw[FRAME_MAX];
    size_t n = fread(raw, 1, sizeof raw, f);
    int ferr = ferror(f);
    fclose(f);
    if (ferr || n == 0) return -2;
    if (n == sizeof raw) {
        /* Cap hit: refuse oversized (incomplete read risk). */
        return -3;
    }

    /* Prefer hex if every non-separator byte is a hex digit and we get pairs. */
    int all_hexish = 1;
    size_t hex_digits = 0;
    for (size_t i = 0; i < n; i++) {
        unsigned char c = raw[i];
        if (isspace(c) || c == ',' || c == ':' || c == '-') continue;
        if (c == '0' && i + 1 < n && (raw[i + 1] == 'x' || raw[i + 1] == 'X')) {
            i++;
            continue;
        }
        if (hex_nibble(c) < 0) {
            all_hexish = 0;
            break;
        }
        hex_digits++;
    }

    if (all_hexish && hex_digits >= 2 && (hex_digits % 2) == 0) {
        size_t o = 0;
        int hi = -1;
        for (size_t i = 0; i < n; i++) {
            unsigned char c = raw[i];
            if (isspace(c) || c == ',' || c == ':' || c == '-') continue;
            if (c == '0' && i + 1 < n && (raw[i + 1] == 'x' || raw[i + 1] == 'X')) {
                i++;
                continue;
            }
            int v = hex_nibble(c);
            if (v < 0) return -4;
            if (hi < 0) {
                hi = v;
            } else {
                if (o >= cap) return -3;
                out[o++] = (uint8_t)((hi << 4) | v);
                hi = -1;
            }
        }
        if (hi >= 0 || o == 0) return -4;
        *out_len = o;
        return 0;
    }

    /* Raw binary. */
    if (n > cap) return -3;
    memcpy(out, raw, n);
    *out_len = n;
    return 0;
}

static int all_zeros(const uint8_t *p, size_t n) {
    for (size_t i = 0; i < n; i++)
        if (p[i] != 0) return 0;
    return 1;
}

#if defined(__linux__)
static int ensure_oem_node(unsigned n, char *path, size_t path_sz) {
    if (n > 7) return -1;
    snprintf(path, path_sz, "/dev/oem_ipc%u", n);
    char sysa[128], sysb[160];
    snprintf(sysa, sizeof sysa, "/sys/class/cpif/oem_ipc%u/dev", n);
    snprintf(sysb, sizeof sysb, "/sys/devices/platform/cpif/oem_ipc%u/dev", n);
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sysa, "r");
    if (!f) f = fopen(sysb, "r");
    if (f) {
        if (fscanf(f, "%u:%u", &maj, &min) != 2) {
            fclose(f);
            return -1;
        }
        fclose(f);
    }
    struct stat st;
    if (stat(path, &st) == 0 && S_ISCHR(st.st_mode)) return 0;
    if (!maj) return -1;
    unlink(path);
    if (mknod(path, S_IFCHR | 0666, makedev(maj, min))) return -1;
    return 0;
}
#endif

static int run_self_test(void) {
    const char *dir = getenv("TMPDIR");
    if (!dir || !*dir) dir = getenv("TEMP");
    if (!dir || !*dir) dir = getenv("TMP");
    if (!dir || !*dir) dir = ".";
    char path[512];
    snprintf(path, sizeof path, "%s/oem-ipc-inject-selftest.%d", dir, (int)getpid());

    /* Hex with 0x / whitespace. */
    {
        FILE *f = fopen(path, "wb");
        if (!f) return 1;
        fputs("0x08 01\n10,02\n", f);
        fclose(f);
        uint8_t buf[FRAME_MAX];
        size_t len = 0;
        if (load_frame(path, buf, sizeof buf, &len) != 0) return 1;
        if (len != 4 || buf[0] != 0x08 || buf[1] != 0x01 || buf[2] != 0x10 || buf[3] != 0x02)
            return 1;
    }
    /* Continuous hex. */
    {
        FILE *f = fopen(path, "wb");
        if (!f) return 1;
        fputs("deadbeef", f);
        fclose(f);
        uint8_t buf[FRAME_MAX];
        size_t len = 0;
        if (load_frame(path, buf, sizeof buf, &len) != 0) return 1;
        if (len != 4 || buf[0] != 0xde || buf[1] != 0xad || buf[2] != 0xbe || buf[3] != 0xef)
            return 1;
    }
    /* Raw binary with non-hex byte. */
    {
        FILE *f = fopen(path, "wb");
        if (!f) return 1;
        unsigned char raw[] = {0x01, 0x02, 0xff, 'Z'};
        fwrite(raw, 1, sizeof raw, f);
        fclose(f);
        uint8_t buf[FRAME_MAX];
        size_t len = 0;
        if (load_frame(path, buf, sizeof buf, &len) != 0) return 1;
        if (len != 4 || buf[3] != 'Z') return 1;
    }
    /* Empty refuse. */
    {
        FILE *f = fopen(path, "wb");
        if (!f) return 1;
        fclose(f);
        uint8_t buf[FRAME_MAX];
        size_t len = 0;
        if (load_frame(path, buf, sizeof buf, &len) != -2) return 1;
    }
    /* All-zero refuse path (checked after load). */
    {
        FILE *f = fopen(path, "wb");
        if (!f) return 1;
        fputs("0000", f);
        fclose(f);
        uint8_t buf[FRAME_MAX];
        size_t len = 0;
        if (load_frame(path, buf, sizeof buf, &len) != 0) return 1;
        if (!all_zeros(buf, len)) return 1;
    }
    unlink(path);
    puts("PASS: hex/raw load, empty refuse, all-zero detect");
    return 0;
}

static void usage(void) {
    fprintf(stderr,
            "usage: oem-ipc-inject self-test\n"
            "       oem-ipc-inject inject <frame-file> [oem_ipcN]\n"
            "frame-file: raw bytes or hex text (no default / no invented 0x2f50)\n"
            "oem_ipcN: 0..7 (default 0). Refuses empty and all-zero frames.\n");
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "self-test"))
        return run_self_test();

    if (argc < 3 || strcmp(argv[1], "inject") != 0) {
        usage();
        return 64;
    }
    const char *frame_path = argv[2];
    unsigned ipc_n = 0;
    if (argc >= 4) {
        char *end = NULL;
        long v = strtol(argv[3], &end, 10);
        if (!end || *end || v < 0 || v > 7) {
            fprintf(stderr, "oem_ipcN must be 0..7\n");
            return 64;
        }
        ipc_n = (unsigned)v;
    }
    if (argc > 4) {
        usage();
        return 64;
    }

    uint8_t frame[FRAME_MAX];
    size_t len = 0;
    int lr = load_frame(frame_path, frame, sizeof frame, &len);
    if (lr == -1) {
        printf("open_frame_fail path_len=%zu errno=%d\n", strlen(frame_path), errno);
        return 1;
    }
    if (lr == -2) {
        puts("refuse empty frame (no invent / no default payload)");
        return 2;
    }
    if (lr == -3) {
        printf("refuse oversized frame cap=%d\n", FRAME_MAX);
        return 2;
    }
    if (lr != 0) {
        puts("refuse frame parse (hex/raw)");
        return 2;
    }
    if (all_zeros(frame, len)) {
        printf("refuse all-zero frame len=%zu (no invent body)\n", len);
        return 2;
    }

#if !defined(__linux__)
    (void)ipc_n;
    printf("parse_ok write_len=%zu (inject requires Linux oem_ipc)\n", len);
    return 0;
#else
    char node[64];
    if (ensure_oem_node(ipc_n, node, sizeof node)) {
        printf("oem_ipc%u node fail errno=%d\n", ipc_n, errno);
        return 1;
    }

    FILE *ms = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (ms) {
        if (fscanf(ms, "%31s", state) != 1) state[0] = 0;
        fclose(ms);
    }
    if (strcmp(state, "ONLINE") != 0) {
        printf("requires ONLINE modem_state=%s\n", state[0] ? state : "?");
        return 1;
    }

    int fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) {
        printf("open_fail node=oem_ipc%u errno=%d\n", ipc_n, errno);
        return 1;
    }
    struct stat st;
    if (fstat(fd, &st) || !S_ISCHR(st.st_mode)) {
        close(fd);
        puts("not chr");
        return 1;
    }
    printf("inject oem_ipc%u maj=%u min=%u write_len=%zu (body not logged)\n",
           ipc_n, major(st.st_rdev), minor(st.st_rdev), len);
    ssize_t w = write(fd, frame, len);
    if (w != (ssize_t)len) {
        printf("write_ret=%zd errno=%d\n", w, errno);
        close(fd);
        return 3;
    }
    puts("write_ok");
    close(fd);
    return 0;
#endif
}
