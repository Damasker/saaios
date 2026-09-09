#include "../src/display-watchdog.h"

#include <assert.h>

int main(void) {
    struct display_watchdog no_ready = display_watchdog_new(100, 500, 200);
    assert(display_watchdog_observe(
               &no_ready, 599, false, true, -1, true, 1) == DISPLAY_WAIT);
    assert(display_watchdog_observe(
               &no_ready, 600, false, true, -1, true, 2) ==
           DISPLAY_FALLBACK_NO_READINESS);

    struct display_watchdog running = display_watchdog_new(0, 500, 200);
    assert(display_watchdog_observe(
               &running, 10, true, true, -1, true, 1) ==
           DISPLAY_START_CLIENT);
    assert(display_watchdog_observe(
               &running, 50, true, true, 1, true, 2) == DISPLAY_WAIT);
    assert(display_watchdog_observe(
               &running, 250, true, true, 1, true, 2) ==
           DISPLAY_FALLBACK_STALE_HEARTBEAT);

    struct display_watchdog compositor_exit =
        display_watchdog_new(0, 500, 200);
    assert(display_watchdog_observe(
               &compositor_exit, 1, false, false, -1, false, 0) ==
           DISPLAY_FALLBACK_COMPOSITOR_EXITED);

    struct display_watchdog client_exit = display_watchdog_new(0, 500, 200);
    assert(display_watchdog_observe(
               &client_exit, 10, true, true, -1, true, 1) ==
           DISPLAY_START_CLIENT);
    assert(display_watchdog_observe(
               &client_exit, 20, true, true, 0, true, 1) ==
           DISPLAY_FALLBACK_CLIENT_EXITED);
    return 0;
}
