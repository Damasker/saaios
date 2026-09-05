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
    const char prefix[] = "saaios-probe: ";
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

int main(void) {
    const struct timespec delay = {.tv_sec = 30, .tv_nsec = 0};

    write_message("native PID 1 reached; returning to bootloader in 30 seconds");
    nanosleep(&delay, NULL);
    write_message("probe complete; rebooting to bootloader");
    sync();

    if (syscall(SYS_reboot,
                LINUX_REBOOT_MAGIC1,
                LINUX_REBOOT_MAGIC2,
                LINUX_REBOOT_CMD_RESTART2,
                "bootloader") == -1) {
        char error_message[128];
        snprintf(error_message,
                 sizeof(error_message),
                 "reboot syscall failed: %s",
                 strerror(errno));
        write_message(error_message);
    }

    for (;;) {
        pause();
    }
}
