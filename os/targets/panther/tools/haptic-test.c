#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <unistd.h>

int main(void) {
    int fd = open("/dev/input/haptic", O_RDWR | O_CLOEXEC);
    if (fd < 0) {
        fprintf(stderr, "haptic-test: open failed: %s\n", strerror(errno));
        return 1;
    }

    int16_t custom_data[2] = {0, 2};
    struct ff_effect effect = {0};
    effect.type = FF_PERIODIC;
    effect.id = -1;
    effect.direction = 0x0000;
    effect.replay.length = 30;
    effect.u.periodic.waveform = FF_CUSTOM;
    effect.u.periodic.custom_data = custom_data;
    effect.u.periodic.custom_len = 2;
    if (ioctl(fd, EVIOCSFF, &effect) < 0) {
        fprintf(stderr, "haptic-test: upload failed: %s\n", strerror(errno));
        close(fd);
        return 2;
    }

    struct input_event gain = {0};
    gain.type = EV_FF;
    gain.code = FF_GAIN;
    gain.value = 65;
    if (write(fd, &gain, sizeof(gain)) != (ssize_t)sizeof(gain)) {
        fprintf(stderr, "haptic-test: gain failed: %s\n", strerror(errno));
        close(fd);
        return 3;
    }

    struct input_event event = {0};
    event.type = EV_FF;
    event.code = (uint16_t)effect.id;
    event.value = 1;
    if (write(fd, &event, sizeof(event)) != (ssize_t)sizeof(event)) {
        fprintf(stderr, "haptic-test: playback failed: %s\n", strerror(errno));
        (void)ioctl(fd, EVIOCRMFF, effect.id);
        close(fd);
        return 4;
    }

    usleep(700000);
    close(fd);
    puts("haptic-test: pulse complete");
    return 0;
}
