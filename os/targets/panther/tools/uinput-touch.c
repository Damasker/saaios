#define _GNU_SOURCE

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>
#include <linux/uinput.h>

/* S09 Change 2 / ADR-028's method reused verbatim: a synthetic
 * protocol-B touchscreen over /dev/uinput, matching exactly what
 * saai-displayd's touch.rs actually reads (ABS_MT_POSITION_X/Y,
 * ABS_MT_TRACKING_ID, SYN_REPORT) -- not the simpler BTN_TOUCH/ABS_X/Y
 * single-touch protocol.
 *
 * Reads "X Y" lines from a FIFO (one line per discrete tap: down, a
 * short pause, up) until EOF, then destroys the device and exits. The
 * FIFO is opened O_RDWR by this process itself so it never sees EOF
 * just because a writer briefly disconnects between taps. */

static void emit(int fd, unsigned short type, unsigned short code, int value) {
    struct input_event ev;
    memset(&ev, 0, sizeof(ev));
    ev.type = type;
    ev.code = code;
    ev.value = value;
    if (write(fd, &ev, sizeof(ev)) != (ssize_t)sizeof(ev)) {
        perror("uinput-touch: write event");
        exit(1);
    }
}

static void tap(int fd, int x, int y) {
    emit(fd, EV_ABS, ABS_MT_TRACKING_ID, 1);
    emit(fd, EV_ABS, ABS_MT_POSITION_X, x);
    emit(fd, EV_ABS, ABS_MT_POSITION_Y, y);
    emit(fd, EV_SYN, SYN_REPORT, 0);
    struct timespec hold = {0, 80L * 1000 * 1000};
    nanosleep(&hold, NULL);
    emit(fd, EV_ABS, ABS_MT_TRACKING_ID, -1);
    emit(fd, EV_SYN, SYN_REPORT, 0);
    struct timespec gap = {0, 200L * 1000 * 1000};
    nanosleep(&gap, NULL);
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s /path/to/fifo\n", argv[0]);
        return 2;
    }

    int fd = open("/dev/uinput", O_WRONLY | O_NONBLOCK);
    if (fd < 0) {
        perror("uinput-touch: open /dev/uinput");
        return 1;
    }

    if (ioctl(fd, UI_SET_EVBIT, EV_SYN) < 0 ||
        ioctl(fd, UI_SET_EVBIT, EV_ABS) < 0 ||
        ioctl(fd, UI_SET_ABSBIT, ABS_MT_POSITION_X) < 0 ||
        ioctl(fd, UI_SET_ABSBIT, ABS_MT_POSITION_Y) < 0 ||
        ioctl(fd, UI_SET_ABSBIT, ABS_MT_TRACKING_ID) < 0) {
        perror("uinput-touch: UI_SET_*BIT");
        close(fd);
        return 1;
    }

    struct uinput_user_dev uidev;
    memset(&uidev, 0, sizeof(uidev));
    snprintf(uidev.name, UINPUT_MAX_NAME_SIZE, "saaios-synthetic-touch");
    uidev.id.bustype = BUS_VIRTUAL;
    uidev.absmin[ABS_MT_POSITION_X] = 0;
    uidev.absmax[ABS_MT_POSITION_X] = 1080;
    uidev.absmin[ABS_MT_POSITION_Y] = 0;
    uidev.absmax[ABS_MT_POSITION_Y] = 2400;
    uidev.absmin[ABS_MT_TRACKING_ID] = -1;
    uidev.absmax[ABS_MT_TRACKING_ID] = 65535;

    if (write(fd, &uidev, sizeof(uidev)) != (ssize_t)sizeof(uidev)) {
        perror("uinput-touch: write uinput_user_dev");
        close(fd);
        return 1;
    }
    if (ioctl(fd, UI_DEV_CREATE) < 0) {
        perror("uinput-touch: UI_DEV_CREATE");
        close(fd);
        return 1;
    }

    fprintf(stderr, "uinput-touch: device created, reading taps from %s\n", argv[1]);

    int fifo_fd = open(argv[1], O_RDWR);
    if (fifo_fd < 0) {
        perror("uinput-touch: open fifo");
        ioctl(fd, UI_DEV_DESTROY);
        close(fd);
        return 1;
    }
    FILE *fifo = fdopen(fifo_fd, "r");
    if (!fifo) {
        perror("uinput-touch: fdopen fifo");
        ioctl(fd, UI_DEV_DESTROY);
        close(fd);
        return 1;
    }

    char line[128];
    while (fgets(line, sizeof(line), fifo)) {
        int x, y;
        if (strncmp(line, "quit", 4) == 0) {
            break;
        }
        if (sscanf(line, "%d %d", &x, &y) == 2) {
            fprintf(stderr, "uinput-touch: tap %d,%d\n", x, y);
            tap(fd, x, y);
        }
    }

    fclose(fifo);
    ioctl(fd, UI_DEV_DESTROY);
    close(fd);
    fprintf(stderr, "uinput-touch: device destroyed, exiting\n");
    return 0;
}
