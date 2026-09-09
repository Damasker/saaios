#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#include "display-watchdog.h"

#define ENABLE_PATH "/run/saaios-display.enable"
#define READY_PATH "/run/saai-displayd.ready"
#define HEARTBEAT_PATH "/run/saai-displayd.heartbeat"
#define REASON_PATH "/run/saai-display-fallback.reason"
#define LOG_PATH "/run/saai-display-supervisor.log"
#define RUNTIME_DIR "/run/saai-wayland"
#define READY_TIMEOUT_MS 8000U
#define HEARTBEAT_TIMEOUT_MS 2000U

static uint64_t monotonic_ms(void) {
    struct timespec now = {0};
    if (clock_gettime(CLOCK_MONOTONIC, &now) < 0) {
        return 0;
    }
    return (uint64_t)now.tv_sec * 1000U + (uint64_t)now.tv_nsec / 1000000U;
}

static void redirect_log(void) {
    int output = open(LOG_PATH, O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC, 0644);
    if (output < 0) {
        return;
    }
    (void)dup2(output, STDOUT_FILENO);
    (void)dup2(output, STDERR_FILENO);
    if (output > STDERR_FILENO) {
        close(output);
    }
}

static void child_setup(void) {
    (void)prctl(PR_SET_PDEATHSIG, SIGKILL);
    redirect_log();
}

static pid_t start_fallback(void) {
    pid_t child = fork();
    if (child == 0) {
        child_setup();
        execl("/saaios/drm-splash", "drm-splash", NULL);
        _exit(127);
    }
    return child;
}

static pid_t start_compositor(void) {
    pid_t child = fork();
    if (child == 0) {
        child_setup();
        (void)setenv("XDG_RUNTIME_DIR", RUNTIME_DIR, 1);
        execl("/saaios/saai-displayd", "saai-displayd",
              "--backend", "panther",
              "--socket", "wayland-saaios",
              "--report", "/run/saai-displayd-report.json",
              "--ready", READY_PATH,
              "--heartbeat", HEARTBEAT_PATH,
              NULL);
        _exit(127);
    }
    return child;
}

static pid_t start_client(void) {
    pid_t child = fork();
    if (child == 0) {
        child_setup();
        (void)setenv("XDG_RUNTIME_DIR", RUNTIME_DIR, 1);
        (void)setenv("WAYLAND_DISPLAY", "wayland-saaios", 1);
        execl("/saaios/saai-demo-surface", "saai-demo-surface",
              "--interactive", NULL);
        _exit(127);
    }
    return child;
}

static bool child_alive(pid_t child) {
    if (child <= 0) {
        return false;
    }
    int status = 0;
    pid_t result = waitpid(child, &status, WNOHANG);
    return result == 0;
}

static void stop_child(pid_t child) {
    if (child <= 0) {
        return;
    }
    (void)kill(child, SIGKILL);
    while (waitpid(child, NULL, 0) < 0 && errno == EINTR) {}
}

static bool read_heartbeat(uint64_t *value) {
    int input = open(HEARTBEAT_PATH, O_RDONLY | O_CLOEXEC);
    if (input < 0) {
        return false;
    }
    char text[32] = {0};
    ssize_t count = read(input, text, sizeof(text) - 1);
    close(input);
    if (count <= 0) {
        return false;
    }
    char *end = NULL;
    unsigned long long parsed = strtoull(text, &end, 10);
    if (end == text) {
        return false;
    }
    *value = (uint64_t)parsed;
    return true;
}

static void record_reason(const char *reason) {
    int output = open(REASON_PATH,
                      O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (output < 0) {
        return;
    }
    (void)write(output, reason, strlen(reason));
    close(output);
}

static void run_fallback_forever(const char *reason) {
    record_reason(reason);
    pid_t fallback = start_fallback();
    for (;;) {
        if (!child_alive(fallback)) {
            usleep(250000);
            fallback = start_fallback();
        }
        usleep(100000);
    }
}

int main(void) {
    redirect_log();
    (void)unlink(REASON_PATH);
    pid_t fallback = start_fallback();
    while (access(ENABLE_PATH, F_OK) < 0) {
        if (!child_alive(fallback)) {
            usleep(250000);
            fallback = start_fallback();
        }
        usleep(100000);
    }
    (void)unlink(ENABLE_PATH);
    stop_child(fallback);

    (void)mkdir(RUNTIME_DIR, 0700);
    (void)chmod(RUNTIME_DIR, 0700);
    (void)unlink(RUNTIME_DIR "/wayland-saaios");
    (void)unlink(READY_PATH);
    (void)unlink(HEARTBEAT_PATH);

    pid_t compositor = start_compositor();
    if (compositor <= 0) {
        run_fallback_forever("CompositorSpawnFailed");
    }
    struct display_watchdog watchdog = display_watchdog_new(
        monotonic_ms(), READY_TIMEOUT_MS, HEARTBEAT_TIMEOUT_MS);
    pid_t client = -1;
    for (;;) {
        uint64_t current = 0;
        bool heartbeat_known = read_heartbeat(&current);
        enum display_action action = display_watchdog_observe(
            &watchdog,
            monotonic_ms(),
            access(READY_PATH, F_OK) == 0,
            child_alive(compositor),
            client > 0 ? (child_alive(client) ? 1 : 0) : -1,
            heartbeat_known,
            current);
        switch (action) {
            case DISPLAY_START_CLIENT:
                client = start_client();
                if (client <= 0) {
                    stop_child(compositor);
                    run_fallback_forever("ClientSpawnFailed");
                }
                break;
            case DISPLAY_FALLBACK_NO_READINESS:
                stop_child(compositor);
                run_fallback_forever("NoReadiness");
                break;
            case DISPLAY_FALLBACK_COMPOSITOR_EXITED:
                stop_child(client);
                run_fallback_forever("CompositorExited");
                break;
            case DISPLAY_FALLBACK_CLIENT_EXITED:
                stop_child(compositor);
                run_fallback_forever("ClientExited");
                break;
            case DISPLAY_FALLBACK_STALE_HEARTBEAT:
                stop_child(client);
                stop_child(compositor);
                run_fallback_forever("StaleHeartbeat");
                break;
            case DISPLAY_WAIT:
                break;
        }
        usleep(50000);
    }
}
