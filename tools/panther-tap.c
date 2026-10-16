#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

struct input_event {
    int64_t sec;
    int64_t usec;
    uint16_t type;
    uint16_t code;
    int32_t value;
};

static int emit(int fd, uint16_t type, uint16_t code, int32_t value) {
    struct input_event event;
    memset(&event, 0, sizeof event);
    event.type = type;
    event.code = code;
    event.value = value;
    if (write(fd, &event, sizeof event) != (ssize_t)sizeof event) {
        perror("write");
        return -1;
    }
    return 0;
}

static int tap(int x, int y) {
    int fd = open("/dev/input/touchscreen", O_WRONLY);
    if (fd < 0) {
        perror("touchscreen");
        return 1;
    }
    int rc = 0;
    rc |= emit(fd, 3, 0x39, 1);
    rc |= emit(fd, 3, 0x35, x);
    rc |= emit(fd, 3, 0x36, y);
    rc |= emit(fd, 0, 0, 0);
    usleep(60000);
    rc |= emit(fd, 3, 0x39, -1);
    rc |= emit(fd, 0, 0, 0);
    close(fd);
    return rc ? 1 : 0;
}

static int power(void) {
    int fd = open("/dev/input/power-button", O_WRONLY);
    if (fd < 0) {
        perror("power-button");
        return 1;
    }
    int rc = 0;
    rc |= emit(fd, 1, 116, 1);
    rc |= emit(fd, 0, 0, 0);
    usleep(80000);
    rc |= emit(fd, 1, 116, 0);
    rc |= emit(fd, 0, 0, 0);
    close(fd);
    return rc ? 1 : 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "power") == 0) {
        return power();
    }
    if (argc != 3) {
        fprintf(stderr, "usage: panther-tap X Y | panther-tap power\n");
        return 2;
    }
    return tap(atoi(argv[1]), atoi(argv[2]));
}
