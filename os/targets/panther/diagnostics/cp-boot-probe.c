/* Isolated diagnostic. Never dispatch the included production loader. */
#define SAAIOS_DIAGNOSTIC_WRAPPER 1
#if defined(PROBE_COMPLETE) && (!defined(PROBE_PREAMBLE) || !defined(PROBE_FULL_MAIN) || !defined(PROBE_FIRMWARE_ONLY))
#error "Complete probe requires all verified firmware stages and preamble"
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
    alarm(180);
    if (system("test \"$(sha256sum /tmp/saaios-probe-b-modem.bin | cut -d ' ' -f 1)\" = 449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b") != 0)
        die("B firmware hash mismatch; no hardware operations");
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
        result = sit_req_resp(SIT_FIN, SIT_FIN_ACK, SIT_ACK_DEADLINE_MS);
        if (result == 0) result = do_ioctl("COMPLETE", IOCTL_COMPLETE_NORMAL_BOOTUP, NULL);
#ifdef PROBE_QUERY_SIM
        if (result == 0) {
            log_line("SIM query with IPC/RFS held open; no RFS filesystem service");
            int query = system("/tmp/sit-sim-status query-sim-status");
            log_line("SIM query process status=%d (separate from boot result)", query);
        }
#endif
        for (int i = 0; i < 10; i++) { print_modem_state("post-complete"); sleep(1); }
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
