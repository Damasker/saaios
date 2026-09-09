#ifndef SAAIOS_DISPLAY_WATCHDOG_H
#define SAAIOS_DISPLAY_WATCHDOG_H

#include <stdbool.h>
#include <stdint.h>

enum display_action {
    DISPLAY_WAIT = 0,
    DISPLAY_START_CLIENT,
    DISPLAY_FALLBACK_NO_READINESS,
    DISPLAY_FALLBACK_COMPOSITOR_EXITED,
    DISPLAY_FALLBACK_CLIENT_EXITED,
    DISPLAY_FALLBACK_STALE_HEARTBEAT,
};

struct display_watchdog {
    uint64_t started_ms;
    uint64_t ready_timeout_ms;
    uint64_t heartbeat_timeout_ms;
    bool client_started;
    bool heartbeat_known;
    uint64_t heartbeat;
    uint64_t heartbeat_changed_ms;
};

static inline struct display_watchdog display_watchdog_new(
        uint64_t started_ms,
        uint64_t ready_timeout_ms,
        uint64_t heartbeat_timeout_ms) {
    return (struct display_watchdog) {
        .started_ms = started_ms,
        .ready_timeout_ms = ready_timeout_ms,
        .heartbeat_timeout_ms = heartbeat_timeout_ms,
        .client_started = false,
        .heartbeat_known = false,
        .heartbeat = 0,
        .heartbeat_changed_ms = started_ms,
    };
}

/* client_alive: -1 before launch, 0 exited, 1 alive. */
static inline enum display_action display_watchdog_observe(
        struct display_watchdog *watchdog,
        uint64_t now_ms,
        bool ready,
        bool compositor_alive,
        int client_alive,
        bool heartbeat_known,
        uint64_t heartbeat) {
    if (!compositor_alive) {
        return DISPLAY_FALLBACK_COMPOSITOR_EXITED;
    }
    if (!watchdog->client_started) {
        if (ready) {
            watchdog->client_started = true;
            watchdog->heartbeat_known = heartbeat_known;
            watchdog->heartbeat = heartbeat;
            watchdog->heartbeat_changed_ms = now_ms;
            return DISPLAY_START_CLIENT;
        }
        if (now_ms - watchdog->started_ms >= watchdog->ready_timeout_ms) {
            return DISPLAY_FALLBACK_NO_READINESS;
        }
        return DISPLAY_WAIT;
    }
    if (client_alive == 0) {
        return DISPLAY_FALLBACK_CLIENT_EXITED;
    }
    if (heartbeat_known &&
        (!watchdog->heartbeat_known || heartbeat != watchdog->heartbeat)) {
        watchdog->heartbeat_known = true;
        watchdog->heartbeat = heartbeat;
        watchdog->heartbeat_changed_ms = now_ms;
    } else if (now_ms - watchdog->heartbeat_changed_ms >=
               watchdog->heartbeat_timeout_ms) {
        return DISPLAY_FALLBACK_STALE_HEARTBEAT;
    }
    return DISPLAY_WAIT;
}

#endif
