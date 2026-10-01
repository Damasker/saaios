/*
 * Bounded reproduction of the observed CP2A protected-NV RFS handshake.
 * This is a diagnostic, not a filesystem service. It never opens NV or EFS.
 *
 * Factory rfsd SHA-256: 58d7f885e7533a328268f0de47ef9eb9995cdfa6b317d755b57973d4f5dfb71b.
 * The command-6 error reply follows the factory RFS_IO_REQUEST operation-2
 * path (status 6). All incoming packets must match the recorded sequence.
 */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef _WIN32
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <sys/file.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>
#endif

enum { MAX_EVENTS = 3, TOTAL_TIMEOUT_MS = 60000, READ_CAP = 64,
       MONITOR_MS = 120000, MONITOR_SAMPLE_MS = 5000 };

#ifndef _WIN32
/* Google/Samsung modem_prj.h: _IOR(IOCTL_MAGIC='o', 0x59, int). */
#define IOCTL_GET_OPENED_STATUS _IOR('o', 0x59, int)
#endif

/* CP messages observed on the RFS device, including the complete header. */
static const uint8_t request_unprotect[12] = {
    0x07,0,0,0, 0x04,0,0,0, 0x03,0,0,0
};
static const uint8_t request_op_status[20] = {
    0x03,0,0,0, 0x0c,0,0,0, 0,0,0,0, 0x03,0,0,0, 0,0,0,0
};
static const uint8_t request_io_write[24] = {
    0x06,0,0x01,0, 0x10,0,0,0, 0x03,0,0,0,
    0,0,0,0, 0x06,0xe4,0x02,0, 0x02,0,0,0
};
/* CP-visible factory status messages; no local file operation is reproduced. */
static const uint8_t reply_unprotect[16] = {
    0x03,0,0,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0
};
static const uint8_t reply_io_error[16] = {
    0x03,0,0x01,0, 0x08,0,0,0, 0x06,0,0,0, 0x03,0,0,0
};

struct exchange {
    const uint8_t *request;
    size_t request_len;
    const uint8_t *reply;
    size_t reply_len;
};
static const struct exchange expected[MAX_EVENTS] = {
    {request_unprotect, sizeof request_unprotect, reply_unprotect, sizeof reply_unprotect},
    {request_op_status, sizeof request_op_status, NULL, 0},
    {request_io_write, sizeof request_io_write, reply_io_error, sizeof reply_io_error},
};

static int classify(unsigned step, const uint8_t *packet, size_t packet_len,
                    const uint8_t **reply, size_t *reply_len) {
    *reply = NULL;
    *reply_len = 0;
    if (step >= MAX_EVENTS || packet_len != expected[step].request_len ||
        memcmp(packet, expected[step].request, packet_len) != 0)
        return -1;
    *reply = expected[step].reply;
    *reply_len = expected[step].reply_len;
    return 0;
}

/* Host-only test: no device or lockfile is opened in this mode. */
static int fixture(void) {
    uint8_t changed[READ_CAP];
    const uint8_t *reply;
    size_t reply_len;
    for (unsigned step = 0; step < MAX_EVENTS; ++step) {
        const struct exchange *e = &expected[step];
        if (classify(step, e->request, e->request_len, &reply, &reply_len) ||
            reply_len != e->reply_len ||
            (reply_len && memcmp(reply, e->reply, reply_len)))
            return 1;
        if (classify(step, e->request, e->request_len - 1, &reply, &reply_len) == 0)
            return 2;
        memcpy(changed, e->request, e->request_len);
        changed[e->request_len] = 0;
        if (classify(step, changed, e->request_len + 1, &reply, &reply_len) == 0)
            return 3;
        for (size_t i = 0; i < e->request_len; ++i) {
            changed[i] ^= 1;
            if (classify(step, changed, e->request_len, &reply, &reply_len) == 0)
                return 4;
            changed[i] ^= 1;
        }
        if (classify((step + 1) % MAX_EVENTS, e->request, e->request_len,
                     &reply, &reply_len) == 0)
            return 5;
    }
    if (classify(MAX_EVENTS, request_unprotect, sizeof request_unprotect,
                 &reply, &reply_len) == 0)
        return 6;
    puts("rfs-error-probe fixture PASS: three exact requests, replies and rejection cases");
    return 0;
}

#ifndef _WIN32
static int64_t monotonic_ms(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts)) return -1;
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

enum cp_state { CP_INVALID, CP_OFFLINE, CP_BOOTING, CP_ONLINE };

static enum cp_state get_cp_state(void) {
    char state[32] = {0};
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    if (!f) return CP_INVALID;
    int n = fscanf(f, "%31s", state);
    fclose(f);
    if (n != 1) return CP_INVALID;
    if (strcmp(state, "ONLINE") == 0) return CP_ONLINE;
    if (strcmp(state, "BOOTING") == 0) return CP_BOOTING;
    if (strcmp(state, "OFFLINE") == 0) return CP_OFFLINE;
    return CP_INVALID;
}

static int open_verified_node(const char *path, const char *sysdev) {
    unsigned maj, min;
    FILE *f = fopen(sysdev, "r");
    if (!f) return -1;
    int n = fscanf(f, "%u:%u", &maj, &min);
    fclose(f);
    if (n != 2) return -1;
    int fd = open(path, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return -1;
    struct stat st;
    if (fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        close(fd);
        return -1;
    }
    return fd;
}

static int open_verified_rfs(void) {
    return open_verified_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
}

static int wait_for(int fd, short events, int64_t deadline) {
    for (;;) {
        int64_t now = monotonic_ms();
        if (now < 0) return -1;
        int64_t remaining = deadline - now;
        if (remaining <= 0) return 0;
        struct pollfd pfd = {.fd = fd, .events = events};
        int rc = poll(&pfd, 1, remaining > INT32_MAX ? INT32_MAX : (int)remaining);
        if (rc < 0 && errno == EINTR) continue;
        if (rc <= 0 || (pfd.revents & (POLLERR | POLLHUP | POLLNVAL))) return rc < 0 ? -1 : 0;
        if (pfd.revents & events) return 1;
        return -1;
    }
}

static int wait_cp_online(int64_t deadline) {
    for (;;) {
        enum cp_state state = get_cp_state();
        if (state == CP_ONLINE) return 1;
        if (state == CP_INVALID) return 0;
        int64_t now = monotonic_ms();
        if (now < 0 || now >= deadline) return 0;
        struct timespec delay = {.tv_sec = 0, .tv_nsec = 100000000};
        nanosleep(&delay, NULL);
    }
}

/* Only POLLIN readiness is observed; unknown packets are not consumed. */
static int observe_channel(int fd) {
    struct pollfd pfd = {.fd = fd, .events = POLLIN};
    int rc = poll(&pfd, 1, 0);
    if (rc < 0 || (pfd.revents & (POLLERR | POLLHUP | POLLNVAL))) return -1;
    return rc > 0 && (pfd.revents & POLLIN) ? 1 : 0;
}

static int monitor_channels(int ipc, int rfs) {
    int64_t start = monotonic_ms();
    if (start < 0) return 20;
    int64_t end = start + MONITOR_MS;
    for (;;) {
        int64_t now = monotonic_ms();
        if (now < 0 || get_cp_state() != CP_ONLINE) {
            fputs("Monitor stopped: CP left ONLINE or clock failed\n", stderr);
            return 21;
        }
        int ipc_pending = observe_channel(ipc);
        int rfs_pending = observe_channel(rfs);
        if (ipc_pending < 0 || rfs_pending < 0) {
            fputs("Monitor stopped: IPC or RFS poll failed\n", stderr);
            return 22;
        }
        printf("monitor_ms=%lld ipc_pending=%d rfs_pending=%d\n",
               (long long)(now - start), ipc_pending, rfs_pending);
        fflush(stdout);
        if (now >= end) return 0;
        int64_t remaining = end - now;
        int64_t delay_ms = remaining < MONITOR_SAMPLE_MS ? remaining : MONITOR_SAMPLE_MS;
        struct timespec delay = {.tv_sec = delay_ms / 1000,
                                 .tv_nsec = (delay_ms % 1000) * 1000000};
        while (nanosleep(&delay, &delay) && errno == EINTR) { }
    }
}

static int hold_monitor_online(void) {
    int lock = open("/run/saaios-sit-status.lock",
                    O_CREAT | O_RDWR | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        fputs("Monitor refused: common SIT lock busy or unavailable\n", stderr);
        if (lock >= 0) close(lock);
        return 30;
    }
    if (get_cp_state() != CP_ONLINE) {
        fputs("Monitor refused: CP not ONLINE\n", stderr);
        close(lock);
        return 31;
    }
    int ipc = open_verified_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_verified_rfs();
    if (ipc < 0 || rfs < 0) {
        fputs("Monitor refused: IPC0/RFS0 device identity failed\n", stderr);
        if (ipc >= 0) close(ipc);
        if (rfs >= 0) close(rfs);
        close(lock);
        return 32;
    }
    int rfs_opened = 0;
    if (ioctl(rfs, IOCTL_GET_OPENED_STATUS, &rfs_opened) != 0 || rfs_opened != 1) {
        fputs("Monitor refused: another RFS owner exists or open count is unknown\n", stderr);
        close(rfs);
        close(ipc);
        close(lock);
        return 33;
    }
    /* ready-network-once is IPC-only and may now take the common SIT lock. */
    flock(lock, LOCK_UN);
    close(lock);
    puts("MONITOR_READY IPC0+RFS0 held; common SIT lock released; IPC-only queries may run");
    fflush(stdout);
    int rc = monitor_channels(ipc, rfs);
    fputs("WARNING: closing a last channel descriptor may purge queued CP requests.\n", stderr);
    close(rfs);
    close(ipc);
    return rc;
}

static int run_exact(int monitor_after) {
    /* Share the existing diagnostic lock with SIT tools that also open RFS. */
    int lock = open("/run/saaios-sit-status.lock",
                    O_CREAT | O_RDWR | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        fprintf(stderr, "RFS diagnostic lock busy or unavailable\n");
        if (lock >= 0) close(lock);
        return 10;
    }
    if (get_cp_state() == CP_INVALID) {
        fprintf(stderr, "CP state unavailable or invalid; no RFS access\n");
        close(lock);
        return 11;
    }
    int fd = open_verified_rfs();
    if (fd < 0) {
        fprintf(stderr, "RFS device identity could not be verified\n");
        close(lock);
        return 12;
    }
    int ipc = -1;
    if (monitor_after) {
        ipc = open_verified_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
        if (ipc < 0) {
            fputs("IPC0 device identity could not be verified\n", stderr);
            close(fd);
            close(lock);
            return 23;
        }
    }
    int64_t start = monotonic_ms();
    if (start < 0) {
        if (ipc >= 0) close(ipc);
        close(fd); close(lock); return 13;
    }
    int64_t deadline = start + TOTAL_TIMEOUT_MS;
    int rc = 0;
    for (unsigned step = 0; step < MAX_EVENTS; ++step) {
        if (get_cp_state() == CP_INVALID || wait_for(fd, POLLIN, deadline) != 1) {
            fprintf(stderr, "RFS step %u unavailable or timed out\n", step + 1);
            rc = 14;
            break;
        }
        uint8_t packet[READ_CAP];
        ssize_t n = read(fd, packet, sizeof packet);
        if (n <= 0) {
            fprintf(stderr, "RFS step %u read failed\n", step + 1);
            rc = 15;
            break;
        }
        const uint8_t *reply;
        size_t reply_len;
        if (classify(step, packet, (size_t)n, &reply, &reply_len)) {
            /* No packet bytes are logged and no response is written. */
            fprintf(stderr, "RFS step %u unexpected packet, %zd bytes; stopped\n",
                    step + 1, n);
            rc = 16;
            break;
        }
        printf("RFS step %u accepted (%zd bytes)\n", step + 1, n);
        if (reply_len) {
            /* This CPIF driver's poll method never advertises POLLOUT. */
            /* Its write method rejects RFS writes before CP reaches ONLINE. */
            if (!wait_cp_online(deadline) ||
                write(fd, reply, reply_len) != (ssize_t)reply_len) {
                fprintf(stderr, "RFS step %u response failed; stopped\n", step + 1);
                rc = 17;
                break;
            }
            printf("RFS step %u sent factory status (%zu bytes)\n", step + 1, reply_len);
        }
    }
    if (rc == 0) {
        if (!wait_cp_online(deadline)) {
            rc = 18;
            fprintf(stderr, "RFS sequence completed, CP did not reach ONLINE\n");
        }
    }
    if (rc == 0 && monitor_after) {
        /* Let a dedicated SIT client use its shared lock during the window. */
        flock(lock, LOCK_UN);
        close(lock);
        lock = -1;
        puts("RFS exact sequence completed; IPC0 and RFS0 held for 120 seconds");
        fflush(stdout);
        rc = monitor_channels(ipc, fd);
    }
    if (ipc >= 0) close(ipc);
    if (monitor_after)
        fputs("WARNING: closing a last channel descriptor may purge queued CP requests.\n", stderr);
    close(fd);
    if (lock >= 0) close(lock);
    if (rc == 0) puts("RFS exact three-step diagnostic completed");
    return rc;
}
#endif

int main(int argc, char **argv) {
    setvbuf(stdout, NULL, _IOLBF, 0);
    if (argc == 2 && strcmp(argv[1], "--fixture") == 0) return fixture();
    if (argc == 2 && strcmp(argv[1], "--run-exact-rfs-20260929") == 0) {
#ifndef _WIN32
        return run_exact(0);
#else
        fputs("Device run requires Linux\n", stderr);
        return 2;
#endif
    }
    if (argc == 2 && strcmp(argv[1], "--run-exact-and-monitor-120s") == 0) {
#ifndef _WIN32
        return run_exact(1);
#else
        fputs("Device run requires Linux\n", stderr);
        return 2;
#endif
    }
    if (argc == 2 && strcmp(argv[1], "--hold-monitor-online-120s") == 0) {
#ifndef _WIN32
        return hold_monitor_online();
#else
        fputs("Device run requires Linux\n", stderr);
        return 2;
#endif
    }
    fprintf(stderr, "Usage: %s --fixture | --run-exact-rfs-20260929 | --run-exact-and-monitor-120s | --hold-monitor-online-120s\n", argv[0]);
    return 2;
}
