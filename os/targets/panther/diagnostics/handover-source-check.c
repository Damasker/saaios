/* Read-only representation check. No ioctl, output block, or identity logging. */
#include "../src/sit-handover.h"
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <sys/stat.h>
#include <sys/resource.h>
#include <sys/prctl.h>
#include <unistd.h>

static void wipe(void *p, size_t n)
{
    volatile unsigned char *v = p;
    while (n--) *v++ = 0;
}

static int check_identity(const char *path)
{
    unsigned char bytes[17] = {0};
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    size_t n = fread(bytes, 1, sizeof(bytes), f);
    int ok = !ferror(f) && n == 16 && bytes[15] == 0;
    fclose(f);
    for (unsigned i = 0; i < 15; ++i)
        if (bytes[i] < '0' || bytes[i] > '9') ok = 0;
    wipe(bytes, sizeof(bytes));
    return ok ? 0 : -1;
}

int main(int argc, char **argv)
{
    int candidate = argc == 3 && !strcmp(argv[1], "candidate-no-json");
    if (!candidate && (argc != 2 || strcmp(argv[1], "check-sources"))) return 64;
    const struct rlimit core_limit = {0, 0};
    if (setrlimit(RLIMIT_CORE, &core_limit) || prctl(PR_SET_DUMPABLE, 0)) return 1;
    alarm(15);
    FILE *f = fopen("/proc/bootconfig", "rb");
    if (!f) return 1;
    char *buf = calloc(1, 65537);
    if (!buf) { fclose(f); return 1; }
    size_t n = fread(buf, 1, 65536, f);
    int bad = ferror(f) || n == 65536 || memchr(buf, 0, n) != NULL;
    fclose(f);
    const char key[] = "androidboot.cdt_hwid";
    unsigned matches = 0;
    uint32_t fields[10] = {0};
    if (!bad) {
        char *line = buf;
        while (*line) {
            char *end = strchr(line, '\n');
            if (!end) end = line + strlen(line);
            size_t len = (size_t)(end-line);
            if (len >= sizeof(key)-1 && !memcmp(line, key, sizeof(key)-1)) {
                char *v = line + sizeof(key)-1;
                if (v < end && (*v == ' ' || *v == '\t' || *v == '=')) {
                    ++matches;
                    while (v < end && (*v == ' ' || *v == '\t')) ++v;
                    if (v == end || *v++ != '=') bad = 1;
                    while (v < end && (*v == ' ' || *v == '\t')) ++v;
                    char *last = end;
                    while (last > v && (last[-1] == ' ' || last[-1] == '\t')) --last;
                    if (last-v != 36 || v[0] != '"' || last[-1] != '"' ||
                        saaios_parse_cdt(v+1, 34, fields)) bad = 1;
                }
            }
            line = *end ? end+1 : end;
        }
    }
    bad |= matches != 1;
    puts(bad ? "CDT: unsupported or missing representation" : "CDT: strict format accepted");
    wipe(buf, 65537); free(buf);
    const char *ids[] = {
        "/sys/firmware/devicetree/base/chosen/config/imei1",
        "/sys/firmware/devicetree/base/chosen/config/imei2"
    };
    for (unsigned i = 0; i < 2; ++i) {
        int result = check_identity(ids[i]);
        printf("Identity source %u: %s\n", i+1, result ? "unsupported or unavailable" : "15 digits plus NUL accepted");
        bad |= result != 0;
    }
    if (candidate && !bad) {
        /* Candidate only: no ioctl or output artifact even on success. */
        uint8_t ids_bytes[2][16] = {{0}}, sig[64] = {0}, rf[4] = {0};
        uint8_t output[SAAIOS_HANDOVER_SIZE] = {0};
        struct saaios_handover_inputs input = {0};
        const char *paths[] = {ids[0], ids[1], argv[2],
            "/sys/firmware/devicetree/base/chosen/plat/rfid"};
        uint8_t *targets[] = {ids_bytes[0], ids_bytes[1], sig, rf};
        const size_t lengths[] = {16,16,64,4};
        struct stat st;
        errno = 0;
        if (!stat("/sys/firmware/devicetree/base/chosen/config/modem_flag", &st) ||
            errno != ENOENT) bad = 1;
        for (unsigned i = 0; i < 4 && !bad; ++i) {
            f = fopen(paths[i], "rb");
            if (!f) { bad = 1; break; }
            size_t got = fread(targets[i], 1, lengths[i], f);
            int extra = fgetc(f);
            bad |= got != lengths[i] || extra != EOF || ferror(f);
            fclose(f);
        }
        uint32_t rfid = ((uint32_t)rf[0]<<24) | ((uint32_t)rf[1]<<16) |
                        ((uint32_t)rf[2]<<8) | rf[3];
        if (!bad) bad |= saaios_handover_no_json_words(fields, rfid, input.words) != 0;
        input.identity[0] = ids_bytes[0]; input.identity[1] = ids_bytes[1];
        input.identity_size[0] = input.identity_size[1] = 16;
        input.signature = sig; input.signature_size = 64;
        if (!bad) bad |= saaios_build_handover(output, sizeof(output), &input) != 0;
        puts(bad ? "Candidate: refused" : "Candidate: 161 bytes assembled in memory and discarded");
        wipe(output, sizeof(output)); wipe(&input, sizeof(input));
        wipe(ids_bytes, sizeof(ids_bytes)); wipe(sig, sizeof(sig)); wipe(rf, sizeof(rf));
    }
    wipe(fields, sizeof(fields));
    puts("No block sent or saved; source values withheld. Candidate is not deployment approval.");
    return bad ? 1 : 0;
}
