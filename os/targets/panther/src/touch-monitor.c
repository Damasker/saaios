#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <linux/input.h>
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#define SLOT_COUNT 10

struct touch_slot {
    int tracking_id;
    int x;
    int y;
    bool changed;
};

int main(void) {
    int fd = open("/dev/input/touchscreen", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        fprintf(stderr, "touch-monitor: open failed: %s\n", strerror(errno));
        return 1;
    }

    struct touch_slot slots[SLOT_COUNT];
    for (int index = 0; index < SLOT_COUNT; ++index) {
        slots[index].tracking_id = -1;
        slots[index].x = 0;
        slots[index].y = 0;
        slots[index].changed = false;
    }
    int current_slot = 0;
    fprintf(stderr, "touch-monitor: ready for %d simultaneous contacts\n",
            SLOT_COUNT);

    for (;;) {
        struct input_event event;
        ssize_t count = read(fd, &event, sizeof(event));
        if (count < 0) {
            if (errno == EINTR) {
                continue;
            }
            fprintf(stderr, "touch-monitor: read failed: %s\n",
                    strerror(errno));
            return 2;
        }
        if (count != (ssize_t)sizeof(event)) {
            continue;
        }
        if (event.type == EV_ABS) {
            if (event.code == ABS_MT_SLOT &&
                event.value >= 0 && event.value < SLOT_COUNT) {
                current_slot = event.value;
            } else if (event.code == ABS_MT_TRACKING_ID) {
                slots[current_slot].tracking_id = event.value;
                slots[current_slot].changed = true;
            } else if (event.code == ABS_MT_POSITION_X) {
                slots[current_slot].x = event.value;
                slots[current_slot].changed = true;
            } else if (event.code == ABS_MT_POSITION_Y) {
                slots[current_slot].y = event.value;
                slots[current_slot].changed = true;
            }
        } else if (event.type == EV_SYN && event.code == SYN_REPORT) {
            for (int index = 0; index < SLOT_COUNT; ++index) {
                if (!slots[index].changed) {
                    continue;
                }
                if (slots[index].tracking_id < 0) {
                    fprintf(stderr, "touch-monitor: slot=%d up\n", index);
                } else {
                    fprintf(stderr,
                            "touch-monitor: slot=%d id=%d x=%d y=%d\n",
                            index, slots[index].tracking_id,
                            slots[index].x, slots[index].y);
                }
                slots[index].changed = false;
            }
        }
    }
}
