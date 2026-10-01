/* Isolated diagnostic. Never dispatch the included production loader. */
#define SAAIOS_DIAGNOSTIC_WRAPPER 1
#if defined(PROBE_COMPLETE) && (!defined(PROBE_PREAMBLE) || !defined(PROBE_FULL_MAIN) || !defined(PROBE_FIRMWARE_ONLY))
#error "Complete probe requires all verified firmware stages and preamble"
#endif
#if defined(PROBE_OWNER_HANDOFF) && !defined(PROBE_COMPLETE)
#error "Owner handoff requires the complete guarded probe"
#endif
#if defined(PROBE_OWNER_HANDOFF) && defined(PROBE_QUERY_SIM)
#error "Owner handoff is incompatible with the competing PROBE_QUERY_SIM reader"
#endif
#define main unused_original_main
#include "cp-boot-probe-support.inc"
#undef main

#ifdef PROBE_HANDOVER
#ifndef PROBE_COMPLETE
#error "Handover comparison requires complete guarded probe"
#endif
#define SAAIOS_HANDOVER_INTERNAL 1
#include "handover-source-check.c"
#endif

#ifdef PROBE_PREAMBLE
#include "../src/sit-boot-preamble.h"
static int probe_exchange(void *ctx, const uint8_t *packet, size_t n, uint32_t ack) {
    (void)ctx;
    log_line("preamble packet len=%zu expected=%x", n, ack);
    if (sit_write_bytes(packet, n) < 0) return -1;
    return sit_req_resp(0, ack, SIT_ACK_DEADLINE_MS);
}
static int send_factory_preamble(const uint8_t *bin, size_t len) {
    return saaios_sit_boot_preamble(bin, len, probe_exchange, NULL);
}
#endif

#ifdef PROBE_OWNER_HANDOFF
#ifndef PROBE_OWNER_EXEC
#define PROBE_OWNER_EXEC "/data/saaios/bin/modem-channel-owner"
#endif
#ifndef PROBE_OWNER_LOG
#define PROBE_OWNER_LOG "/data/saaios/var/modem-channel-owner.log"
#endif
#define PROBE_OWNER_READY_TIMEOUT_MS 5000

/* fork/dup keeps the same open file descriptions. The parent never reads
 * IPC/RFS; the child becomes their only reader before FIN is sent. */
static void probe_owner_child(int ipc_fd, int rfs_fd, int ready_write) {
    int ipc_owned = fcntl(ipc_fd, F_DUPFD, 3);
    int rfs_owned = fcntl(rfs_fd, F_DUPFD, 3);
    int ready_owned = fcntl(ready_write, F_DUPFD, 3);
    if (ipc_owned < 0 || rfs_owned < 0 || ready_owned < 0) _exit(126);
    close(ipc_fd);
    close(rfs_fd);
    close(ready_write);

    int log_fd = open(PROBE_OWNER_LOG,
                      O_WRONLY | O_CREAT | O_APPEND | O_NOFOLLOW, 0600);
    struct stat log_stat;
    if (log_fd < 0 || fstat(log_fd, &log_stat) || !S_ISREG(log_stat.st_mode) ||
        log_stat.st_uid != geteuid() || log_stat.st_nlink != 1 ||
        (log_stat.st_mode & 0777) != 0600 ||
        dup2(log_fd, STDOUT_FILENO) < 0 || dup2(log_fd, STDERR_FILENO) < 0)
        _exit(126);
    if (log_fd > STDERR_FILENO) close(log_fd);
    int null_fd = open("/dev/null", O_RDONLY | O_NOFOLLOW);
    if (null_fd < 0 || dup2(null_fd, STDIN_FILENO) < 0) _exit(126);
    if (null_fd > STDERR_FILENO) close(null_fd);

    char ipc_arg[24], rfs_arg[24], ready_arg[24];
    if (snprintf(ipc_arg, sizeof(ipc_arg), "%d", ipc_owned) <= 0 ||
        snprintf(rfs_arg, sizeof(rfs_arg), "%d", rfs_owned) <= 0 ||
        snprintf(ready_arg, sizeof(ready_arg), "%d", ready_owned) <= 0)
        _exit(126);
    char *const owner_argv[] = {
        (char *)PROBE_OWNER_EXEC,
        "--ipc-fd", ipc_arg,
        "--rfs-fd", rfs_arg,
        "--ready-fd", ready_arg,
        NULL,
    };
    execv(PROBE_OWNER_EXEC, owner_argv);
    _exit(127);
}

static int64_t probe_owner_now_ms(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return -1;
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static int probe_owner_alive(pid_t pid) {
    int status;
    pid_t found;
    do { found = waitpid(pid, &status, WNOHANG); } while (found < 0 && errno == EINTR);
    return found == 0;
}

static void probe_owner_abort(pid_t pid) {
    if (pid <= 0) return;
    if (probe_owner_alive(pid)) kill(pid, SIGKILL);
    int status;
    while (waitpid(pid, &status, 0) < 0 && errno == EINTR) { }
}

static int probe_owner_wait_ready(int ready_read, pid_t pid) {
    static const char expected[] = "READY\n";
    char got[sizeof(expected) - 1];
    size_t used = 0;
    int64_t start = probe_owner_now_ms();
    if (start < 0) return -1;
    int64_t deadline = start + PROBE_OWNER_READY_TIMEOUT_MS;
    while (used < sizeof(got)) {
        int64_t now = probe_owner_now_ms();
        if (now < 0 || now >= deadline) return -1;
        struct pollfd pfd = { .fd = ready_read, .events = POLLIN };
        int rc = poll(&pfd, 1, (int)(deadline - now));
        if (rc < 0 && errno == EINTR) continue;
        if (rc <= 0 || (pfd.revents & (POLLERR | POLLNVAL))) return -1;
        ssize_t n = read(ready_read, got + used, sizeof(got) - used);
        if (n > 0) { used += (size_t)n; continue; }
        if (n < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        return -1; /* EOF before READY or an unexpected read error. */
    }
    if (memcmp(got, expected, sizeof(got)) != 0) return -1;
    /* Owner keeps its write end open until exit. HUP here means the handoff
     * process already died, even if its READY bytes were buffered. */
    struct pollfd pfd = { .fd = ready_read, .events = POLLIN };
    if (poll(&pfd, 1, 0) < 0 ||
        (pfd.revents & (POLLIN | POLLHUP | POLLERR | POLLNVAL)))
        return -1;
    return probe_owner_alive(pid) ? 0 : -1;
}

static int probe_owner_start(int ipc_fd, int rfs_fd, pid_t *owner_pid) {
    int ready_pipe[2];
    if (pipe2(ready_pipe, O_CLOEXEC | O_NONBLOCK) < 0) return -1;
    pid_t pid = fork();
    if (pid < 0) {
        close(ready_pipe[0]);
        close(ready_pipe[1]);
        return -1;
    }
    if (pid == 0) {
        close(ready_pipe[0]);
        probe_owner_child(ipc_fd, rfs_fd, ready_pipe[1]);
    }
    close(ready_pipe[1]);
    int ready = probe_owner_wait_ready(ready_pipe[0], pid);
    close(ready_pipe[0]);
    if (ready != 0) {
        probe_owner_abort(pid);
        return -1;
    }
    *owner_pid = pid;
    return 0;
}
#endif

int main(int argc, char **argv) {
#ifdef PROBE_COMPLETE
#ifdef PROBE_HANDOVER
    if (argc != 3 || strcmp(argv[1], "boot-b-with-verified-nv-handover") != 0) return 64;
    uint8_t handover[SAAIOS_HANDOVER_SIZE] = {0};
    char *source_args[] = {argv[0], "candidate-no-json", argv[2], NULL};
    if (check_handover_sources(3, source_args, handover)) die("handover sources refused");
#else
    if (argc != 2 || strcmp(argv[1], "boot-b-with-verified-nv") != 0) {
        fprintf(stderr, "Requires boot-b-with-verified-nv; never run automatically\n");
        return 64;
    }
#endif
    if (system("sh /tmp/saaios-verify-nv-copies.sh") != 0)
        die("NV verification failed; no hardware operations");
#else
    if (argc != 2 || strcmp(argv[1], "probe-b-ram-only") != 0) {
        fprintf(stderr, "Requires explicit probe-b-ram-only argument\n");
        return 64;
    }
#endif
#ifdef PROBE_OWNER_HANDOFF
    struct stat owner_exec_stat;
    if (lstat(PROBE_OWNER_EXEC, &owner_exec_stat) != 0 ||
        !S_ISREG(owner_exec_stat.st_mode) || access(PROBE_OWNER_EXEC, X_OK) != 0)
        die("runtime owner executable unavailable; no hardware operations");
#endif
    alarm(420);
    /* Our soft gate only: stock B or READY-patch COPY. OEM IOCTL_REQ_SECURITY
     * is defined in support.inc but never issued on the complete probe path. */
    if (system("h=$(sha256sum /tmp/saaios-probe-b-modem.bin | cut -d ' ' -f 1); "
               "test \"$h\" = 449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b "
               "-o \"$h\" = 193e4f48f46b2eed2023fbc1726cdb6e96f2f240ec1cc2b849d2a779dbc5cfd4") != 0)
        die("B firmware hash mismatch; no hardware operations");
    log_line("firmware sha accepted (stock or READY-patch COPY)");
    if (efs_is_mounted()) die("original EFS mounted: refusing");
    char state[64];
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (strcmp(state, "OFFLINE") != 0) die("requires OFFLINE, got %s", state);
    if (dmesg_has_bad_cfg() != 0) die("BAD CFG or unavailable kernel log");
    size_t len = 0;
    uint8_t *bin = read_file("/tmp/saaios-probe-b-modem.bin", &len);
    if (!bin || len < sizeof(struct toc_entry)) die("missing verified B image");
    const struct toc_entry *toc = (const struct toc_entry *)bin;
    if (!toc[0].idx || toc[0].idx > 16 ||
        len < toc[0].idx * sizeof(*toc)) die("invalid TOC");
    const struct toc_entry *boot = find_toc(toc, toc[0].idx, "BOOT");
    const struct toc_entry *stage = find_toc(toc, toc[0].idx, "MAIN");
    if (!boot || !stage || boot->idx != 1 || stage->idx != 2 ||
        boot->size != 0x16800 || !boot->b_off || !stage->b_off ||
        stage->size <= 0x02400000u ||
        (uint64_t)boot->b_off + boot->size > len ||
        (uint64_t)stage->b_off + stage->size > len) die("unexpected TOC layout");
    log_line("B probe BOOT=%x MAIN=%x", boot->size, stage->size);
#ifdef PROBE_FIRMWARE_ONLY
    const char *names[] = { "VSS", "APM" };
    const struct toc_entry *extra[2];
    for (unsigned i = 0; i < 2; i++) {
        extra[i] = find_toc(toc, toc[0].idx, names[i]);
        if (!extra[i] || extra[i]->idx != i + 3 || !extra[i]->size ||
            !extra[i]->b_off || (uint64_t)extra[i]->b_off + extra[i]->size > len)
            die("invalid %s before POWER_ON", names[i]);
    }
#endif
    boot_fd = open(BOOT0_PATH, O_RDWR | O_CLOEXEC);
#ifdef PROBE_COMPLETE
    const char *nv_names[] = { "NV_NORM", "NV_PROT" };
    const char *nv_paths[] = { NV_NORM_PATH, NV_PROT_PATH };
    uint8_t *nv_data[2];
    for (unsigned i = 0; i < 2; i++) {
        const struct toc_entry *e = find_toc(toc, toc[0].idx, nv_names[i]);
        size_t n = 0;
        nv_data[i] = read_file(nv_paths[i], &n);
        if (!e || e->idx != i+5 || e->b_off != 0 || e->size != 0x80000 ||
            !nv_data[i] || n != e->size) die("invalid verified NV copy");
    }
#endif
    if (boot_fd < 0) die("boot node: %s", strerror(errno));
    if (do_ioctl("POWER_ON", IOCTL_POWER_ON, NULL) < 0) die("POWER_ON failed");
    sleep(3);
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (strcmp(state, "OFFLINE") != 0 || dmesg_has_bad_cfg() != 0) die("not safe after POWER_ON");
    if (load_image("BOOT", bin + boot->b_off, boot->size, 0, 0) < 0) die("BOOT load failed");
    struct boot_mode mode = { .idx = CP_BOOT_MODE_NORMAL };
    if (do_ioctl("START", IOCTL_START_CP_BOOTLOADER, &mode) < 0) die("START failed");
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (strcmp(state, "BOOTING") != 0 || dmesg_has_bad_cfg() != 0) die("not safe after START");
#ifdef PROBE_HANDOVER
    int handover_rc = do_ioctl("HANDOVER_RAM_ONLY", 0x6f57, handover);
    wipe(handover, sizeof(handover));
    if (handover_rc < 0) die("handover failed; no firmware stages");
#endif
#ifdef PROBE_PREAMBLE
    log_line("FACTORY PREAMBLE: READY, TOC START/BIN/DONE");
    if (send_factory_preamble(bin, len) < 0) die("preamble failed; no MAIN");
#endif
    int result = sit_send_stage(stage->idx, "MAIN", bin + stage->b_off, stage->size, stage->crc);
#ifdef PROBE_FIRMWARE_ONLY
    for (unsigned i = 0; result == 0 && i < 2; i++) {
        log_line("Firmware stage %s; factory CRC disabled", names[i]);
        result = sit_send_stage(extra[i]->idx, names[i], bin + extra[i]->b_off,
                                extra[i]->size, extra[i]->crc);
    }
    log_line("FIRMWARE STAGES END");
#endif
#ifdef PROBE_COMPLETE
    for (unsigned i = 0; result == 0 && i < 2; i++) {
        result = sit_send_stage(i+5, nv_names[i], nv_data[i], 0x80000, 0);
    }
    if (result == 0) {
        int ipc_fd = open("/dev/umts_ipc0", O_RDWR | O_NONBLOCK | O_CLOEXEC);
        int rfs_fd = open("/dev/umts_rfs0", O_RDWR | O_NONBLOCK | O_CLOEXEC);
        if (ipc_fd < 0 || rfs_fd < 0) die("runtime endpoints unavailable; no FIN");
#ifdef PROBE_OWNER_HANDOFF
        pid_t owner_pid = -1;
        if (probe_owner_start(ipc_fd, rfs_fd, &owner_pid) < 0)
            die("runtime owner did not confirm READY; no FIN sent");
        log_line("runtime owner READY before FIN pid=%ld", (long)owner_pid);
        if (!probe_owner_alive(owner_pid)) {
            log_line("runtime owner exited before FIN; no FIN sent");
            result = -1;
        }
        if (result == 0)
            result = sit_req_resp(SIT_FIN, SIT_FIN_ACK, SIT_ACK_DEADLINE_MS);
#else
        result = sit_req_resp(SIT_FIN, SIT_FIN_ACK, SIT_ACK_DEADLINE_MS);
#endif
#ifdef PROBE_OWNER_HANDOFF
        if (result == 0 && !probe_owner_alive(owner_pid)) {
            log_line("runtime owner exited after FIN; refusing COMPLETE");
            result = -1;
        }
#endif
        if (result == 0) result = do_ioctl("COMPLETE", IOCTL_COMPLETE_NORMAL_BOOTUP, NULL);
#ifdef PROBE_OWNER_HANDOFF
        if (result != 0) {
            log_line("FIN/COMPLETE failed; stopping diagnostic owner");
            probe_owner_abort(owner_pid);
            owner_pid = -1;
        }
#endif
#ifdef PROBE_QUERY_SIM
        if (result == 0) {
            log_line("SIM query with IPC/RFS held open; no RFS filesystem service");
            int query = system("/tmp/sit-sim-status query-sim-status");
            log_line("SIM query process status=%d (separate from boot result)", query);
        }
#endif
        for (int i = 0; i < 10; i++) { print_modem_state("post-complete"); sleep(1); }
#ifdef PROBE_OWNER_HANDOFF
        if (result == 0 && !probe_owner_alive(owner_pid)) {
            log_line("runtime owner exited during post-complete observation");
            result = -1;
        }
#endif
        close(ipc_fd); close(rfs_fd);
    }
    for (unsigned i = 0; i < 2; i++) free(nv_data[i]);
#endif
    log_line("PROBE END result=%d (2=bound reached, -1=transfer failed)", result);
    print_modem_state("probe-end");
    free(bin);
    close(boot_fd);
#ifdef PROBE_FULL_MAIN
    return result == 0 ? 0 : 1;
#else
    return result == 2 ? 0 : 1;
#endif
}
