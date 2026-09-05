#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <linux/reboot.h>
#include <stdio.h>
#include <string.h>
#include <sys/reboot.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

static void write_message(const char *message) {
    const char prefix[] = "saaios-reboot-probe: ";
    const char newline[] = "\n";
    const char *paths[] = {"/dev/kmsg", "/dev/ttySAC0", "/dev/console"};

    for (unsigned int i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i) {
        int fd = open(paths[i], O_WRONLY | O_CLOEXEC);
        if (fd < 0) {
            continue;
        }
        (void)write(fd, prefix, sizeof(prefix) - 1);
        (void)write(fd, message, strlen(message));
        (void)write(fd, newline, sizeof(newline) - 1);
        close(fd);
    }
}

static void load_module(const char *path) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return;
    }
    (void)syscall(SYS_finit_module, fd, "", 0);
    close(fd);
}

int main(void) {
    const struct timespec delay = {.tv_sec = 10, .tv_nsec = 0};

    write_message("native PID 1 reached");
    load_module("/lib/modules/logbuffer.ko");
    load_module("/lib/modules/google-bms.ko");
    load_module("/lib/modules/exynos-pd_el3.ko");
    load_module("/lib/modules/pixel-reboot.ko");
    write_message("Pixel restart driver loaded; rebooting in 10 seconds");
    nanosleep(&delay, NULL);
    sync();

    (void)syscall(SYS_reboot,
                  LINUX_REBOOT_MAGIC1,
                  LINUX_REBOOT_MAGIC2,
                  LINUX_REBOOT_CMD_RESTART2,
                  "bootloader");
    (void)syscall(SYS_reboot,
                  LINUX_REBOOT_MAGIC1,
                  LINUX_REBOOT_MAGIC2,
                  LINUX_REBOOT_CMD_RESTART,
                  NULL);

    write_message("reboot syscalls returned unexpectedly");
    for (;;) {
        pause();
    }
}
