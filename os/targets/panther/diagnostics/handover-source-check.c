/* Read-only representation check. No ioctl, output block, or identity logging. */
#include "../src/sit-handover.h"
#include <stdio.h>
#include <stdlib.h>

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
    if (argc != 2 || strcmp(argv[1], "check-sources")) return 64;
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
    wipe(fields, sizeof(fields)); wipe(buf, 65537); free(buf);
    const char *ids[] = {
        "/sys/firmware/devicetree/base/chosen/config/imei1",
        "/sys/firmware/devicetree/base/chosen/config/imei2"
    };
    for (unsigned i = 0; i < 2; ++i) {
        int result = check_identity(ids[i]);
        printf("Identity source %u: %s\n", i+1, result ? "unsupported or unavailable" : "15 digits plus NUL accepted");
        bad |= result != 0;
    }
    puts("No handover block constructed or sent; source values withheld.");
    return bad ? 1 : 0;
}
