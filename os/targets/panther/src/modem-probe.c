#define _GNU_SOURCE

#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <net/if.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

/*
 * Read-only Pixel 7 (panther) cellular map. Does not boot the CP, does not
 * start cbd/rild, and does not mount or write efs / efs_backup / modem.
 */

static FILE *out;

static void emit(const char *format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(out, format, args);
    va_end(args);
}

static void emit_file(const char *label, const char *path) {
    FILE *file = fopen(path, "r");
    emit("%s %s: ", label, path);
    if (!file) {
        emit("missing (%s)\n", strerror(errno));
        return;
    }
    char line[256] = {0};
    if (fgets(line, sizeof(line), file) == NULL) {
        emit("empty\n");
        fclose(file);
        return;
    }
    line[strcspn(line, "\r\n")] = '\0';
    emit("%s\n", line[0] ? line : "empty");
    fclose(file);
}

static int copy_trimmed(const char *path, char *value, size_t value_size) {
    FILE *file = fopen(path, "r");
    if (!file) {
        return -1;
    }
    if (fgets(value, (int)value_size, file) == NULL) {
        fclose(file);
        return -1;
    }
    fclose(file);
    value[strcspn(value, "\r\n")] = '\0';
    return value[0] ? 0 : -1;
}

static void walk_dir(const char *path, const char *prefix) {
    DIR *directory = opendir(path);
    if (!directory) {
        emit("dir %s: missing (%s)\n", path, strerror(errno));
        return;
    }
    emit("dir %s:\n", path);
    struct dirent *entry;
    int count = 0;
    while ((entry = readdir(directory)) != NULL) {
        if (entry->d_name[0] == '.') {
            continue;
        }
        if (prefix && strncmp(entry->d_name, prefix, strlen(prefix)) != 0) {
            continue;
        }
        emit("  %s\n", entry->d_name);
        count++;
    }
    if (count == 0) {
        emit("  (none)\n");
    }
    closedir(directory);
}

static void emit_modules(void) {
    static const char *const names[] = {
        "cpif", "cpif_page", "shm_ipc", "boot_device_spi", "cp_thermal_zone",
        "exynos_dit", "google_modemctl", "gnssif", "bbd", "bcm47765",
    };
    FILE *file = fopen("/proc/modules", "r");
    emit("modules:\n");
    if (!file) {
        emit("  /proc/modules missing (%s)\n", strerror(errno));
        return;
    }
    char line[512];
    while (fgets(line, sizeof(line), file)) {
        char name[64] = {0};
        if (sscanf(line, "%63s", name) != 1) {
            continue;
        }
        for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i) {
            if (strcmp(name, names[i]) == 0) {
                emit("  %s", line);
                break;
            }
        }
    }
    fclose(file);
}

static void emit_rmnet(void) {
    DIR *directory = opendir("/sys/class/net");
    emit("net:\n");
    if (!directory) {
        emit("  /sys/class/net missing (%s)\n", strerror(errno));
        return;
    }
    int sock = socket(AF_INET, SOCK_DGRAM | SOCK_CLOEXEC, 0);
    struct dirent *entry;
    int found = 0;
    while ((entry = readdir(directory)) != NULL) {
        const char *name = entry->d_name;
        if (strncmp(name, "rmnet", 5) != 0 &&
            strncmp(name, "umts", 4) != 0 &&
            strncmp(name, "vnet", 4) != 0) {
            continue;
        }
        found++;
        char oper[64];
        char stats_rx[128];
        char stats_tx[128];
        snprintf(oper, sizeof(oper), "/sys/class/net/%s/operstate", name);
        snprintf(stats_rx, sizeof(stats_rx),
                 "/sys/class/net/%s/statistics/rx_bytes", name);
        snprintf(stats_tx, sizeof(stats_tx),
                 "/sys/class/net/%s/statistics/tx_bytes", name);
        char state[32] = "?";
        char rx[32] = "0";
        char tx[32] = "0";
        (void)copy_trimmed(oper, state, sizeof(state));
        (void)copy_trimmed(stats_rx, rx, sizeof(rx));
        (void)copy_trimmed(stats_tx, tx, sizeof(tx));
        int flags = 0;
        if (sock >= 0) {
            struct ifreq request = {0};
            snprintf(request.ifr_name, sizeof(request.ifr_name), "%s", name);
            if (ioctl(sock, SIOCGIFFLAGS, &request) == 0) {
                flags = request.ifr_flags;
            }
        }
        emit("  %s oper=%s flags=0x%x rx=%s tx=%s\n",
             name, state, flags, rx, tx);
    }
    if (found == 0) {
        emit("  (no rmnet/umts/vnet interfaces)\n");
    }
    closedir(directory);
    if (sock >= 0) {
        close(sock);
    }
}

static bool try_modem_state(const char *path, char *value, size_t value_size) {
    if (copy_trimmed(path, value, value_size) == 0) {
        emit_file("sysfs", path);
        return true;
    }
    return false;
}

static void find_modem_state(char *value, size_t value_size) {
    static const char *const known[] = {
        "/sys/devices/platform/cpif/modem_state",
        "/sys/class/misc/modemctl/modem_state",
        "/sys/devices/platform/cpif/cpif/modem_state",
    };
    for (size_t i = 0; i < sizeof(known) / sizeof(known[0]); ++i) {
        if (try_modem_state(known[i], value, value_size)) {
            return;
        }
    }

    DIR *platform = opendir("/sys/devices/platform");
    if (!platform) {
        emit("modem_state: platform sysfs missing\n");
        snprintf(value, value_size, "UNKNOWN");
        return;
    }
    struct dirent *entry;
    while ((entry = readdir(platform)) != NULL) {
        if (entry->d_name[0] == '.') {
            continue;
        }
        char path[320];
        snprintf(path, sizeof(path),
                 "/sys/devices/platform/%s/modem_state", entry->d_name);
        if (try_modem_state(path, value, value_size)) {
            closedir(platform);
            return;
        }
    }
    closedir(platform);
    emit("modem_state: not found\n");
    snprintf(value, value_size, "NO_CPIF");
}

static void emit_partitions(void) {
    static const char *const names[] = {
        "efs", "efs_backup", "modem_userdata", "modem", "radio",
    };
    DIR *directory = opendir("/sys/class/block");
    emit("gpt names:\n");
    if (!directory) {
        emit("  /sys/class/block missing (%s)\n", strerror(errno));
        return;
    }
    struct dirent *entry;
    int found = 0;
    while ((entry = readdir(directory)) != NULL) {
        if (strncmp(entry->d_name, "sd", 2) != 0 &&
            strncmp(entry->d_name, "mmc", 3) != 0 &&
            strncmp(entry->d_name, "dm-", 3) != 0) {
            continue;
        }
        char uevent[160];
        snprintf(uevent, sizeof(uevent),
                 "/sys/class/block/%s/uevent", entry->d_name);
        FILE *file = fopen(uevent, "r");
        if (!file) {
            continue;
        }
        char line[256];
        char partname[64] = {0};
        while (fgets(line, sizeof(line), file)) {
            if (sscanf(line, "PARTNAME=%63s", partname) == 1) {
                break;
            }
        }
        fclose(file);
        if (partname[0] == '\0') {
            continue;
        }
        bool wanted = false;
        for (size_t i = 0; i < sizeof(names) / sizeof(names[0]); ++i) {
            size_t nlen = strlen(names[i]);
            if (strcmp(partname, names[i]) == 0 ||
                (strncmp(partname, names[i], nlen) == 0 &&
                 (partname[nlen] == '\0' || partname[nlen] == '_'))) {
                wanted = true;
                break;
            }
        }
        if (wanted) {
            emit("  %s %s\n", entry->d_name, partname);
            found++;
        }
    }
    if (found == 0) {
        emit("  (no efs/modem GPT names)\n");
    }
    closedir(directory);
}

static void write_state_file(const char *path, const char *state) {
    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (fd < 0) {
        return;
    }
    (void)write(fd, state, strlen(state));
    (void)write(fd, "\n", 1);
    close(fd);
}

int main(int argc, char **argv) {
    const char *report_path = NULL;
    for (int i = 1; i < argc; ++i) {
        if (strcmp(argv[i], "-o") == 0 && i + 1 < argc) {
            report_path = argv[++i];
        }
    }

    if (report_path) {
        out = fopen(report_path, "w");
        if (!out) {
            fprintf(stderr, "modem-probe: open %s failed: %s\n",
                    report_path, strerror(errno));
            return 1;
        }
    } else {
        out = stdout;
    }

    emit("SaaiOS panther modem-probe (read-only)\n");
    emit("goal: CP ONLINE and (rmnet rx/tx != 0 or IPv4 on rmnet*)\n");
    emit("forbidden: efs writes, IOCTL_POWER_OFF, cbd, rild, dd modem\n\n");

    char state[64] = "UNKNOWN";
    find_modem_state(state, sizeof(state));
    emit_modules();
    walk_dir("/sys/class/cpif", NULL);
    walk_dir("/sys/class/misc", "umts_");
    walk_dir("/sys/class/misc", "gnss");
    walk_dir("/dev", "umts_");
    walk_dir("/dev", "gnss");
    emit_file("node", "/dev/logbuffer_cpif");
    emit_file("node", "/dev/umts_boot0");
    emit_file("node", "/dev/umts_ipc0");
    emit_file("node", "/dev/umts_rfs0");
    emit_rmnet();
    emit_partitions();
    emit_file("ds_detect", "/sys/devices/platform/cpif/sim/ds_detect");
    emit("\nsummary_state=%s\n", state);

    write_state_file("/run/saaios-modem.state", state);
    if (out != stdout) {
        fclose(out);
        FILE *copy = fopen("/run/saaios-modem.txt", "w");
        if (copy) {
            FILE *src = fopen(report_path, "r");
            if (src) {
                char buf[4096];
                size_t n;
                while ((n = fread(buf, 1, sizeof(buf), src)) > 0) {
                    fwrite(buf, 1, n, copy);
                }
                fclose(src);
            }
            fclose(copy);
        }
    }

    return 0;
}
