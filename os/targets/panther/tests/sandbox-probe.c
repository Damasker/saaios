#define _GNU_SOURCE

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/capability.h>
#include <linux/reboot.h>
#include <sched.h>
#include <signal.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <sys/un.h>
#include <unistd.h>

static FILE *report;
static int failures;

static void check(bool passed, const char *format, ...) {
    fprintf(report, "%s ", passed ? "PASS" : "FAIL");
    va_list args;
    va_start(args, format);
    vfprintf(report, format, args);
    va_end(args);
    fputc('\n', report);
    if (!passed) {
        ++failures;
    }
}

static bool absent(const char *path) {
    struct stat value;
    errno = 0;
    return lstat(path, &value) < 0 && errno == ENOENT;
}

static bool unix_connect_fails(const char *path) {
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        return false;
    }
    struct sockaddr_un address = {.sun_family = AF_UNIX};
    strncpy(address.sun_path, path, sizeof(address.sun_path) - 1);
    int result = connect(fd, (struct sockaddr *)&address, sizeof(address));
    close(fd);
    return result < 0;
}

static bool internet_connect_fails(void) {
    int fd = socket(AF_INET, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        return true;
    }
    struct sockaddr_in address = {
        .sin_family = AF_INET,
        .sin_port = htons(38127),
    };
    inet_pton(AF_INET, "172.31.7.1", &address.sin_addr);
    int result = connect(fd, (struct sockaddr *)&address, sizeof(address));
    close(fd);
    return result < 0;
}

/* Picking the write target matters: /init looked like an obvious choice
 * but always fails with ETXTBSY (26, "Text file busy") because PID 1 is
 * actively executing from it -- true unsandboxed too, so it never actually
 * exercised the read-only root mount at all (confirmed with a throwaway
 * on-device spike, root-ro-spike.c, not committed). /plat_property_contexts
 * is a plain, always-present, never-executing regular file directly on the
 * rootfs -- writable unsandboxed (verified), EROFS once the sandbox seals
 * root read-only. */
static bool write_fails(const char *path) {
    errno = 0;
    int fd = open(path, O_WRONLY | O_CLOEXEC);
    if (fd >= 0) {
        close(fd);
        return false;
    }
    return errno == EROFS || errno == EACCES || errno == ENOENT;
}

int main(void) {
    const char *data = getenv("SAAIOS_DATA_DIR");
    if (!data) {
        return 2;
    }
    char path[512];
    snprintf(path, sizeof(path), "%s/sandbox-report.txt", data);
    report = fopen(path, "w");
    if (!report) {
        return 2;
    }

    check(absent("/run/saaios/appd.sock"), "appd socket hidden");
    check(absent("/run/saaios/entityd.sock"), "entityd socket hidden");
    check(!absent("/run/saaios/portal.sock"), "portal socket revealed");
    check(!absent("/run/wayland/wayland-1"), "Wayland socket revealed");
    check(absent("/data/saaios/system/saai-appd"), "system binaries hidden");
    check(absent("/data/saaios/var/entities/selection.json"), "raw entities hidden");
    check(absent("/saaios/saai-shell"), "initramfs tools hidden");
    check(absent("/proc/1/status"), "host process tree hidden");
    check(absent("/sys/class"), "sysfs hidden");
    check(absent("/dev/dri/card0"), "DRM device hidden");
    check(absent("/dev/input"), "input devices hidden");
    check(write_fails("/plat_property_contexts"), "root filesystem is read-only");
    check(unix_connect_fails("/run/saaios/appd.sock"), "appd connect denied");
    check(unix_connect_fails("/run/saaios/entityd.sock"), "entityd connect denied");
    check(internet_connect_fails(), "network unavailable without net.internet");

    struct __user_cap_header_struct header = {
        .version = _LINUX_CAPABILITY_VERSION_3,
        .pid = 0,
    };
    struct __user_cap_data_struct caps[2] = {{0}};
    check(syscall(SYS_capget, &header, caps) == 0 && caps[0].effective == 0 &&
              caps[0].permitted == 0 && caps[1].effective == 0 &&
              caps[1].permitted == 0,
          "effective and permitted capabilities cleared");

    errno = 0;
    check(kill(getppid(), 0) < 0 && errno == EPERM, "kill blocked by seccomp");
    errno = 0;
    check(mount("none", "/tmp", "tmpfs", 0, NULL) < 0 && errno == EPERM,
          "mount blocked by seccomp");
    errno = 0;
    check(syscall(SYS_reboot, LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2,
                  LINUX_REBOOT_CMD_RESTART, NULL) < 0 && errno == EPERM,
          "reboot blocked by seccomp");

    fprintf(report, "RESULT %s failures=%d\n", failures ? "FAIL" : "PASS", failures);
    fclose(report);
    return 0;
}
