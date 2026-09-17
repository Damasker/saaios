#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/*
 * Static CP loader for Pixel 7 (panther) s5300 / s5100sit.
 *
 * Ioctl numbers from LineageOS android-16 radio/samsung/s5300/modem_prj.h
 * (same family as live /lib/modules/cpif.ko strings: IOCTL_POWER_ON,
 * IOCTL_START_CP_BOOTLOADER, IOCTL_LOAD_CP_IMAGE via "load_cp_image is null",
 * IOCTL_REQ_SECURITY, IOCTL_COMPLETE_NORMAL_BOOTUP, IOCTL_GET_CPIF_VERSION).
 * Nr values match the older pantah drivers/soc/google/cpif header.
 *
 * Never issues IOCTL_POWER_OFF. Never opens original efs. NV comes from
 * /data/saaios/var/efs-copy only. Does not start vendor cbd or rild.
 */

#define IOCTL_MAGIC 'o'

#define IOCTL_POWER_ON _IO(IOCTL_MAGIC, 0x19)
#define IOCTL_POWER_OFF _IO(IOCTL_MAGIC, 0x20)

enum cp_boot_mode {
    CP_BOOT_MODE_NORMAL = 0,
    CP_BOOT_MODE_DUMP = 1,
    CP_BOOT_RE_INIT = 2,
};

struct boot_mode {
    enum cp_boot_mode idx;
};

#define IOCTL_POWER_RESET _IOW(IOCTL_MAGIC, 0x21, struct boot_mode)
#define IOCTL_START_CP_BOOTLOADER _IOW(IOCTL_MAGIC, 0x22, struct boot_mode)
#define IOCTL_COMPLETE_NORMAL_BOOTUP _IO(IOCTL_MAGIC, 0x23)
#define IOCTL_GET_CP_STATUS _IO(IOCTL_MAGIC, 0x27)

struct cp_image {
    unsigned long long binary;
    uint32_t size;
    uint32_t m_offset;
    uint32_t b_offset;
    uint32_t mode;
    uint32_t len;
} __attribute__((packed));

#define IOCTL_LOAD_CP_IMAGE _IOW(IOCTL_MAGIC, 0x40, struct cp_image)

struct modem_sec_req {
    uint32_t mode;
    uint32_t param2;
    uint32_t param3;
    uint32_t param4;
} __attribute__((packed));

#define IOCTL_REQ_SECURITY _IOW(IOCTL_MAGIC, 0x53, struct modem_sec_req)

#define CPIF_VERSION_SIZE 20
struct cpif_version {
    char string[CPIF_VERSION_SIZE];
} __attribute__((packed));

#define IOCTL_GET_CPIF_VERSION _IOR(IOCTL_MAGIC, 0x56, struct cpif_version)

struct toc_entry {
    char name[12];
    uint32_t b_off;
    uint32_t m_off;
    uint32_t size;
    uint32_t crc;
    uint32_t idx;
} __attribute__((packed));

#define BOOT0_PATH "/dev/umts_boot0"
#define MODEM_STATE_PATH "/sys/devices/platform/cpif/modem_state"
#define EFS_COPY_DIR "/data/saaios/var/efs-copy"
#define NV_NORM_PATH EFS_COPY_DIR "/nv_normal.bin"
#define NV_PROT_PATH EFS_COPY_DIR "/nv_protected.bin"
#define MODEM_MNT "/mnt/modem_img"
#define MODEM_DEV "/dev/block/sda19"

/*
 * SIT UDL commands from factory vendor/bin/cbd strings + aarch64 disassembly
 * (panther AP3A dump, ELF Android 35, not executed). Bases are OR'd with
 * ((toc.idx << 4) & 0xFFF0). Stage 1 START is therefore 0xA110 / 0xC110,
 * matching Pixel Tensor cbd logs (std_udl_stage_start).
 */
#define SIT_START 0x0000A100u
#define SIT_START_ACK 0x0000C100u
#define SIT_BIN 0x0000A10Bu
#define SIT_BIN_ACK 0x0000C10Bu
#define SIT_CRC 0x0000A301u
#define SIT_CRC_ACK 0x0000C300u
#define SIT_DONE 0x0000A10Du
#define SIT_DONE_ACK 0x0000C10Du
#define SIT_READY 0x0000A00Bu
#define SIT_READY_ACK 0x0000C00Bu
#define SIT_FIN 0x0000A400u
#define SIT_FIN_ACK 0x0000C400u
#define SIT_IDX_SPECIAL 0xfu
/*
 * cbd f2f8 default block is 0xC000 (SHMEM-sized). Alt 0x7D00 is only stored
 * after a prior write() failure (bss flag at f6a4) — not a PCIE default.
 * s5100sit / PCIE still one write() of header+payload (1fec0).
 *
 * Live boot0 has no iod,max_tx_size, so bootdump_write does not split a
 * 0xC00C write. format=IPC_BOOT forces EXYNOS SINGLE. ipc_write SIT and
 * exynos_build_fr_config use SZ_2K; live DT pktproc_ul_max_packet_size=0x800.
 * A 0xC00C SINGLE therefore becomes one illegal-sized PCIE/legacy frame
 * (CRASH_EXIT after first MAIN BIN). Keep the cbd 12-byte header, but size
 * the payload so EXYNOS(12)+SIT hdr(12)+chunk == SZ_2K.
 */
#define EXYNOS_HEADER_SIZE 12u
#define SIT_HDR_SIZE 12u
#define SIT_CHUNK 0xC000u
/* cbd e2a0: one poll(POLLIN, 2000). BIN ACK is a 4-byte read after that. */
#define SIT_POLL_MS 2000
#define SIT_ACK_DEADLINE_MS 30000
#define SIT_START_DEADLINE_MS 30000
/*
 * Boot traffic bypasses pktproc UL and writes the legacy NORM_RAW byte ring.
 * Panther's ring is exactly 0x1FD000 = 1018 * 0x800 bytes.  The repeatable
 * failure is the frame that makes cumulative wire bytes 3054 * 0x800, which
 * is exactly three ring lengths.  Give CP time to publish its tail before
 * every exact head wrap; cbd's 0xC000 frames cross this ring much less often.
 */
#define LEGACY_RAW_TXQ_SIZE 0x1FD000u
#define SIT_WIRE_FRAME_SIZE (EXYNOS_HEADER_SIZE + SIT_HDR_SIZE + SIT_CHUNK)
#define LEGACY_FRAMES_PER_WRAP (LEGACY_RAW_TXQ_SIZE / SIT_WIRE_FRAME_SIZE)
#define LEGACY_WRAP_PACE_NS 100000000L

/*
 * Factory cbd 0x1fec0 BIN frame (LE, packed). Caller stores chunk at +2,
 * then 1fec0 copies payload to +12, rewrites +2 to chunk+8, write(chunk+12).
 * len counts the two u32s + payload; it does not include the first 4 bytes.
 * CRC is a later 8-byte {0xA301|bits, toc.crc} write, not inside this header.
 *
 * Live umts_boot0 iod,attrs=0x200 (ATTR_NO_CHECK_MAXQ only). ATTR_NO_LINK_HEADER
 * is 0x100 and is not set, so bootdump_write prepends a 12-byte EXYNOS header
 * (PROTOCOL_SIT). Userspace must not add that wrap — cbd write()s this frame only.
 * EXYNOS len is header+payload (12+write). BOOT is always SINGLE.
 */
struct sit_udl_hdr {
    uint16_t cmd;
    uint16_t len;
    uint32_t total;
    uint32_t offset;
} __attribute__((packed));

static int boot_fd = -1;

static void die(const char *format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(stderr, format, args);
    va_end(args);
    fputc('\n', stderr);
    exit(1);
}

static void log_line(const char *format, ...) {
    va_list args;
    va_start(args, format);
    vprintf(format, args);
    va_end(args);
    fputc('\n', stdout);
    fflush(stdout);
}

static void read_trimmed(const char *path, char *value, size_t value_size) {
    value[0] = '\0';
    FILE *file = fopen(path, "r");
    if (!file) {
        snprintf(value, value_size, "missing(%s)", strerror(errno));
        return;
    }
    if (fgets(value, (int)value_size, file) == NULL) {
        snprintf(value, value_size, "empty");
        fclose(file);
        return;
    }
    fclose(file);
    value[strcspn(value, "\r\n")] = '\0';
    if (value[0] == '\0') {
        snprintf(value, value_size, "empty");
    }
}

static void print_modem_state(const char *when) {
    char state[64];
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    log_line("modem_state %s: %s", when, state);
}

static void print_legacy_status(const char *when, uint32_t chunks,
                                uint32_t off) {
    FILE *file = fopen("/sys/devices/platform/cpif/legacy/status", "r");
    if (!file) {
        log_line("legacy status %s chunk=%u off=0x%x: %s",
                 when, chunks, off, strerror(errno));
        return;
    }
    log_line("legacy status %s chunk=%u off=0x%x", when, chunks, off);
    char line[256];
    while (fgets(line, sizeof(line), file)) {
        line[strcspn(line, "\r\n")] = '\0';
        if (strstr(line, "NORM_RAW") || strstr(line, "TX busy:"))
            log_line("  %s", line);
    }
    fclose(file);
}

static void pace_legacy_wrap(uint32_t chunks, uint32_t off) {
    if (((chunks + 1) % LEGACY_FRAMES_PER_WRAP) != 0)
        return;

    print_legacy_status("pre-wrap", chunks, off);
    log_line("UDL legacy wrap guard: next frame=%u reaches %u * 0x%x; sleep 100ms",
             chunks + 1, (chunks + 1) / LEGACY_FRAMES_PER_WRAP,
             LEGACY_RAW_TXQ_SIZE);
    struct timespec delay = { .tv_sec = 0, .tv_nsec = LEGACY_WRAP_PACE_NS };
    while (nanosleep(&delay, &delay) < 0 && errno == EINTR) {
    }
}

static bool efs_is_mounted(void) {
    FILE *file = fopen("/proc/mounts", "r");
    if (!file) {
        die("open /proc/mounts: %s", strerror(errno));
    }
    char line[512];
    bool mounted = false;
    while (fgets(line, sizeof(line), file)) {
        if (strstr(line, "sda5") || strstr(line, "sda6") ||
            strstr(line, "/mnt/vendor/efs") ||
            strstr(line, "/mnt/efs-ro") ||
            strstr(line, " efs ") ||
            strstr(line, "efs_backup")) {
            mounted = true;
            break;
        }
    }
    fclose(file);
    return mounted;
}

static int do_ioctl(const char *name, unsigned long request, void *arg) {
    if (request == IOCTL_POWER_OFF) {
        die("refusing IOCTL_POWER_OFF");
    }
    int rc = ioctl(boot_fd, request, arg);
    int saved = errno;
    if (rc < 0) {
        log_line("%s: FAIL rc=%d errno=%d (%s) req=0x%lx",
                 name, rc, saved, strerror(saved), request);
    } else {
        log_line("%s: OK rc=%d", name, rc);
    }
    return rc;
}

static int load_image(const char *name, const uint8_t *data, uint32_t size,
                      uint32_t m_off, uint32_t b_off) {
    if (!data || size == 0) {
        log_line("LOAD %s: skip (empty)", name);
        return 0;
    }
    struct cp_image img;
    memset(&img, 0, sizeof(img));
    img.binary = (unsigned long long)(uintptr_t)data;
    img.size = size;
    img.m_offset = m_off;
    img.b_offset = b_off;
    img.mode = 0;
    img.len = size;
    log_line("LOAD %s: size=0x%x m_off=0x%x b_off=0x%x",
             name, size, m_off, b_off);
    return do_ioctl("IOCTL_LOAD_CP_IMAGE", IOCTL_LOAD_CP_IMAGE, &img);
}

static const struct toc_entry *find_toc(const struct toc_entry *toc, int count,
                                        const char *name) {
    for (int i = 0; i < count; ++i) {
        if (strncmp(toc[i].name, name, sizeof(toc[i].name)) == 0) {
            return &toc[i];
        }
    }
    return NULL;
}

static uint8_t *read_file(const char *path, size_t *out_size) {
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        log_line("open %s: %s", path, strerror(errno));
        return NULL;
    }
    struct stat st;
    if (fstat(fd, &st) < 0 || st.st_size <= 0) {
        log_line("stat %s failed", path);
        close(fd);
        return NULL;
    }
    uint8_t *buf = malloc((size_t)st.st_size);
    if (!buf) {
        close(fd);
        die("oom reading %s", path);
    }
    size_t done = 0;
    while (done < (size_t)st.st_size) {
        ssize_t n = read(fd, buf + done, (size_t)st.st_size - done);
        if (n <= 0) {
            free(buf);
            close(fd);
            log_line("read %s failed", path);
            return NULL;
        }
        done += (size_t)n;
    }
    close(fd);
    *out_size = (size_t)st.st_size;
    return buf;
}

static int ensure_modem_node(void) {
    if (access(MODEM_DEV, F_OK) == 0) {
        return 0;
    }
    if (mkdir("/dev/block", 0755) < 0 && errno != EEXIST) {
        log_line("mkdir /dev/block: %s", strerror(errno));
    }
    FILE *uevent = fopen("/sys/block/sda/sda19/uevent", "r");
    if (!uevent) {
        log_line("sda19 uevent missing");
        return -1;
    }
    int major = -1;
    int minor = -1;
    char line[128];
    while (fgets(line, sizeof(line), uevent)) {
        if (sscanf(line, "MAJOR=%d", &major) == 1) {
            continue;
        }
        sscanf(line, "MINOR=%d", &minor);
    }
    fclose(uevent);
    if (major < 0 || minor < 0) {
        log_line("sda19 major/minor missing");
        return -1;
    }
    if (mknod(MODEM_DEV, S_IFBLK | 0644, makedev((unsigned)major, (unsigned)minor)) < 0 &&
        errno != EEXIST) {
        log_line("mknod %s: %s", MODEM_DEV, strerror(errno));
        return -1;
    }
    return 0;
}

static int mount_modem_img(void) {
    if (mkdir(MODEM_MNT, 0755) < 0 && errno != EEXIST) {
        log_line("mkdir %s: %s", MODEM_MNT, strerror(errno));
        return -1;
    }
    if (ensure_modem_node() < 0) {
        return -1;
    }
    if (mount(MODEM_DEV, MODEM_MNT, "ext4", MS_RDONLY, "noload") < 0) {
        log_line("mount modem_a noload: %s — trying ro", strerror(errno));
        if (mount(MODEM_DEV, MODEM_MNT, "ext4", MS_RDONLY, NULL) < 0) {
            log_line("mount modem_a ro: %s", strerror(errno));
            return -1;
        }
    }
    return 0;
}

static void umount_modem_img(void) {
    if (umount(MODEM_MNT) < 0) {
        log_line("umount %s: %s", MODEM_MNT, strerror(errno));
    } else {
        log_line("umount %s: OK", MODEM_MNT);
    }
}

static int find_modem_bin(char *path, size_t path_size) {
    FILE *pipe = popen("busybox find " MODEM_MNT " -name modem.bin -type f", "r");
    if (!pipe) {
        return -1;
    }
    if (!fgets(path, (int)path_size, pipe)) {
        pclose(pipe);
        return -1;
    }
    pclose(pipe);
    path[strcspn(path, "\r\n")] = '\0';
    return path[0] ? 0 : -1;
}

static uint32_t sit_cmd(uint32_t base, uint32_t idx) {
    return base | ((idx << 4) & 0xFFF0u);
}

static int sit_poll_in(int timeout_ms) {
    struct pollfd pfd = { .fd = boot_fd, .events = POLLIN };
    return poll(&pfd, 1, timeout_ms);
}

static int64_t sit_now_ms(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts) < 0) {
        return 0;
    }
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static void sit_log_hex(const char *tag, const uint8_t *data, size_t len) {
    char hex[3 * 32 + 1];
    size_t used = 0;
    size_t n = len > 32 ? 32 : len;
    for (size_t i = 0; i < n && used + 3 < sizeof(hex); ++i) {
        used += (size_t)snprintf(hex + used, sizeof(hex) - used, "%02x ", data[i]);
    }
    if (used > 0 && hex[used - 1] == ' ') {
        hex[used - 1] = '\0';
    } else {
        hex[used] = '\0';
    }
    log_line("%s n=%zu %s", tag, len, hex);
}

/*
 * cbd eb70 never drains. Factory ACK is one 4-byte read. If a previous ACK
 * skb left extra bytes (or a second 0xC12B) on umts_boot0, our next wait
 * would treat that leftover as the next chunk's ACK and write without the
 * CP having ACKed — then RXQ goes quiet. Drain only *before* a BIN write.
 */
static int sit_drain_rx(const char *why) {
    uint8_t buf[64];
    int total = 0;
    /*
     * bootdump_read does not honor O_NONBLOCK: an empty read blocks in
     * the kernel (~100ms "NO data in RXQ" retries) and never returns to
     * a deadline loop. Only read when poll(0) says POLLIN.
     */
    for (;;) {
        if (sit_poll_in(0) <= 0) {
            break;
        }
        ssize_t n = read(boot_fd, buf, sizeof(buf));
        if (n <= 0) {
            break;
        }
        sit_log_hex(why, buf, (size_t)n);
        total += (int)n;
    }
    if (total > 0) {
        log_line("SIT drain %s: %d leftover bytes consumed (not used as ACK)",
                 why, total);
    }
    return total;
}

/*
 * cbd e2a0+eb70: poll(POLLIN, 2000) then one read(4). We keep the 4-byte
 * ACK but (1) retry until a wall deadline so EAGAIN/n==0 is not a 4s abort
 * (bootdump_read logs "NO data in RXQ" and returns 0 on empty), (2) read up
 * to 64 and consume extras in *this* wait so the next wait cannot steal a
 * leftover 0xC12B.
 */
static int sit_wait_u32(uint32_t *out, int deadline_ms,
                        uint8_t *last, size_t *last_n) {
    uint8_t buf[64];
    size_t done = 0;
    int64_t start = sit_now_ms();
    if (last && last_n) {
        *last_n = 0;
    }
    while (sit_now_ms() - start < deadline_ms) {
        int remain = (int)(deadline_ms - (sit_now_ms() - start));
        if (remain <= 0) {
            break;
        }
        if (remain > SIT_POLL_MS) {
            remain = SIT_POLL_MS;
        }
        /* Never read unless POLLIN: bootdump_read blocks on empty RXQ. */
        if (sit_poll_in(remain) <= 0) {
            continue;
        }
        ssize_t n = read(boot_fd, buf + done, sizeof(buf) - done);
        if (n > 0) {
            done += (size_t)n;
            if (last && last_n) {
                size_t keep = done < 64 ? done : 64;
                memcpy(last, buf, keep);
                *last_n = keep;
            }
            if (done >= 4) {
                memcpy(out, buf, 4);
                if (done > 4) {
                    sit_log_hex("SIT extra RX after 4-byte ACK",
                                buf + 4, done - 4);
                }
                return 0;
            }
            continue;
        }
        if (n == 0 || errno == EAGAIN || errno == EWOULDBLOCK ||
            errno == EINTR) {
            done = 0;
            continue;
        }
        log_line("SIT read: n=%zd errno=%d (%s)", n, errno, strerror(errno));
        return -1;
    }
    return -1;
}

static int sit_write_bytes(const void *buf, size_t len) {
    const uint8_t *p = buf;
    int waits = 0;
    /*
     * cbd 0x1fec0 uses one write() of the whole buffer and fails if the
     * return is not exact. A second write() would be a new bootdump_write
     * and, with link_header, a second EXYNOS wrap — that splits the SIT frame.
     */
    for (;;) {
        ssize_t n = write(boot_fd, p, len);
        if (n == (ssize_t)len) {
            return 0;
        }
        if (n < 0 && (errno == EAGAIN || errno == EWOULDBLOCK) && waits < 8) {
            struct pollfd pfd = { .fd = boot_fd, .events = POLLOUT };
            if (poll(&pfd, 1, SIT_POLL_MS) <= 0) {
                waits++;
                continue;
            }
            waits++;
            continue;
        }
        log_line("SIT write want=%zu got=%zd errno=%d (%s)",
                 len, n, errno, strerror(errno));
        return -1;
    }
}

static int sit_req_resp_ex(uint32_t req, uint32_t exp, int deadline_ms,
                           bool verbose) {
    if (req != 0) {
        if (verbose) {
            log_line("SIT UDL req 0x%08x (expect 0x%08x)", req, exp);
        }
        if (sit_write_bytes(&req, sizeof(req)) < 0) {
            log_line("SIT UDL write 0x%08x: %s", req, strerror(errno));
            return -1;
        }
    } else if (verbose) {
        log_line("SIT UDL wait 0x%08x", exp);
    }
    uint32_t ack = 0;
    uint8_t last[64];
    size_t last_n = 0;
    if (sit_wait_u32(&ack, deadline_ms, last, &last_n) == 0) {
        if (verbose || ack != exp) {
            log_line("SIT UDL ack 0x%08x (expect 0x%08x) read=%zu",
                     ack, exp, last_n);
            if (last_n > 4) {
                sit_log_hex("SIT UDL ack bytes", last, last_n);
            }
        }
        return ack == exp ? 0 : -1;
    }
    log_line("SIT UDL timeout waiting 0x%08x (last read %zu bytes)",
             exp, last_n);
    if (last_n > 0) {
        sit_log_hex("SIT UDL last RX", last, last_n);
    }
    return -1;
}

static int sit_req_resp(uint32_t req, uint32_t exp, int deadline_ms) {
    return sit_req_resp_ex(req, exp, deadline_ms, true);
}

static int sit_send_stage(uint32_t idx, const char *name, const uint8_t *data,
                          uint32_t size, uint32_t crc) {
    uint32_t start = sit_cmd(SIT_START, idx);
    uint32_t start_ack = sit_cmd(SIT_START_ACK, idx);
    if (idx == SIT_IDX_SPECIAL) {
        start = SIT_FIN;
        start_ack = SIT_FIN_ACK;
    }
    log_line("UDL %s idx=%u size=0x%x crc=0x%x start=0x%x",
             name, idx, size, crc, start);
    if (sit_req_resp(start, start_ack, SIT_START_DEADLINE_MS) < 0) {
        log_line("UDL %s START fail", name);
        return -1;
    }
    if (!data || size == 0) {
        log_line("UDL %s: no payload after START", name);
        return 0;
    }

    uint32_t off = 0;
    uint32_t last_good = 0;
    uint32_t chunks = 0;
    uint8_t *frame = malloc(sizeof(struct sit_udl_hdr) + SIT_CHUNK);
    if (!frame) {
        die("oom UDL frame");
    }
    while (off < size) {
        uint32_t chunk = size - off;
        if (chunk > SIT_CHUNK) {
            chunk = SIT_CHUNK;
        }
        if (off != 0 && (off % SIT_CHUNK) != 0) {
            log_line("UDL %s offset 0x%x not a %u-byte multiple (drift)",
                     name, off, SIT_CHUNK);
        }
        /* cbd f55c–f568 then 1fec0: store chunk at +2, rewrite to chunk+8. */
        struct sit_udl_hdr *hdr = (struct sit_udl_hdr *)frame;
        memset(frame, 0, sizeof(*hdr) + chunk);
        hdr->cmd = (uint16_t)sit_cmd(SIT_BIN, idx);
        hdr->len = (uint16_t)chunk;
        hdr->total = size;
        hdr->offset = off;
        memcpy(frame + sizeof(*hdr), data + off, chunk);
        hdr->len = (uint16_t)(chunk + 8);
        size_t wrote = sizeof(*hdr) + chunk;
        if (off == 0) {
            log_line("UDL %s first BIN hdr %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x %02x cmd=0x%x len=0x%x total=0x%x off=0x%x write=0x%zx (no EXYNOS in userspace)",
                     name,
                     frame[0], frame[1], frame[2], frame[3],
                     frame[4], frame[5], frame[6], frame[7],
                     frame[8], frame[9], frame[10], frame[11],
                     hdr->cmd, hdr->len, hdr->total, hdr->offset, wrote);
        }
        if (off == 0 || (chunks & 0x1ffu) == 0) {
            log_line("UDL %s chunk off=0x%x payload=0x%x write=0x%zx acked=%u last_good=0x%x",
                     name, off, chunk, wrote, chunks, last_good);
        }
        if (sit_write_bytes(frame, wrote) < 0) {
            log_line("UDL %s chunk write: %s", name, strerror(errno));
            free(frame);
            return -1;
        }
        if (sit_req_resp_ex(0, sit_cmd(SIT_BIN_ACK, idx), SIT_ACK_DEADLINE_MS,
                            off == 0) < 0) {
            log_line("UDL %s BIN ACK fail at 0x%x chunk=%u last_good=0x%x (cbd waits 0x%08x after every BIN write)",
                     name, off, chunks, last_good, sit_cmd(SIT_BIN_ACK, idx));
            free(frame);
            return -1;
        }
        last_good = off;
        chunks++;
        if ((chunks % LEGACY_FRAMES_PER_WRAP) == 0)
            print_legacy_status("post-wrap-ACK", chunks, off);
        off += chunk;
    }
    free(frame);

    uint32_t crc_pkt[2] = { sit_cmd(SIT_CRC, idx), crc };
    log_line("UDL %s crc cmd=0x%x crc=0x%x after %u BIN chunks",
             name, crc_pkt[0], crc, chunks);
    if (sit_write_bytes(crc_pkt, sizeof(crc_pkt)) < 0) {
        log_line("UDL %s CRC write: %s", name, strerror(errno));
        return -1;
    }
    if (sit_req_resp(0, sit_cmd(SIT_CRC_ACK, idx), SIT_ACK_DEADLINE_MS) < 0) {
        log_line("UDL %s CRC ACK fail", name);
        return -1;
    }
    if (sit_req_resp(sit_cmd(SIT_DONE, idx), sit_cmd(SIT_DONE_ACK, idx),
                     SIT_ACK_DEADLINE_MS) < 0) {
        log_line("UDL %s DONE fail", name);
        return -1;
    }
    log_line("UDL %s complete", name);
    return 0;
}

static void hold_forever(void) {
    pid_t child = fork();
    if (child < 0) {
        log_line("holder fork failed: %s — parent will pause", strerror(errno));
        for (;;) {
            pause();
        }
    }
    if (child == 0) {
        (void)setsid();
        signal(SIGHUP, SIG_IGN);
        signal(SIGTERM, SIG_IGN);
        int pid_fd = open("/run/saaios-cp-boot.holder",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
        if (pid_fd >= 0) {
            char buf[32];
            int n = snprintf(buf, sizeof(buf), "%d\n", getpid());
            (void)write(pid_fd, buf, (size_t)n);
            close(pid_fd);
        }
        log_line("holder pid %d keeping ipc0/rfs0/boot0 open", getpid());
        for (;;) {
            pause();
        }
    }
    log_line("holder forked pid %d", child);
}

static void print_status(void) {
    print_modem_state("now");
    if (boot_fd < 0) {
        boot_fd = open(BOOT0_PATH, O_RDWR | O_NONBLOCK | O_CLOEXEC);
        if (boot_fd < 0) {
            log_line("open %s: %s", BOOT0_PATH, strerror(errno));
            return;
        }
    }
    struct cpif_version version;
    memset(&version, 0, sizeof(version));
    int rc = do_ioctl("IOCTL_GET_CPIF_VERSION", IOCTL_GET_CPIF_VERSION, &version);
    if (rc >= 0) {
        log_line("cpif_version: %s", version.string);
    }
    rc = do_ioctl("IOCTL_GET_CP_STATUS", IOCTL_GET_CP_STATUS, NULL);
    log_line("GET_CP_STATUS raw=%d", rc);
}

static int cmd_load(void) {
    if (efs_is_mounted()) {
        die("refusing: original efs is mounted");
    }
    if (access(NV_NORM_PATH, R_OK) != 0 || access(NV_PROT_PATH, R_OK) != 0) {
        die("NV userdata copy missing under %s", EFS_COPY_DIR);
    }

    print_modem_state("before");
    {
        char state[64];
        read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
        if (strcmp(state, "BOOTING") == 0 || strcmp(state, "ONLINE") == 0 ||
            strcmp(state, "CRASH_EXIT") == 0) {
            die("refusing load while modem_state=%s (AP reboot only)", state);
        }
    }

    boot_fd = open(BOOT0_PATH, O_RDWR | O_CLOEXEC);
    if (boot_fd < 0) {
        die("open %s: %s", BOOT0_PATH, strerror(errno));
    }
    log_line("open boot0=%d O_RDWR (stock s5100sit opens no ipc0/rfs0 here)",
             boot_fd);

    struct cpif_version version;
    memset(&version, 0, sizeof(version));
    if (do_ioctl("IOCTL_GET_CPIF_VERSION", IOCTL_GET_CPIF_VERSION, &version) < 0) {
        die("GET_CPIF_VERSION failed — ioctl numbers may not match this kernel");
    }
    log_line("cpif_version: %s", version.string);

    if (mount_modem_img() < 0) {
        die("cannot mount modem_a read-only");
    }
    char modem_bin[256];
    if (find_modem_bin(modem_bin, sizeof(modem_bin)) < 0) {
        umount_modem_img();
        die("modem.bin not found on modem_a");
    }
    log_line("modem.bin: %s", modem_bin);

    size_t bin_size = 0;
    uint8_t *bin = read_file(modem_bin, &bin_size);
    umount_modem_img();
    if (!bin || bin_size < sizeof(struct toc_entry) * 2) {
        die("modem.bin read failed");
    }
    log_line("modem.bin size=%zu", bin_size);

    const struct toc_entry *toc = (const struct toc_entry *)bin;
    int toc_count = (int)toc[0].idx;
    if (toc_count <= 0 || toc_count > 16) {
        toc_count = 8;
    }
    log_line("TOC count=%d", toc_count);
    for (int i = 0; i < toc_count; ++i) {
        char name[13];
        memcpy(name, toc[i].name, 12);
        name[12] = '\0';
        log_line("TOC[%d] %s b_off=0x%x m_off=0x%x size=0x%x crc=0x%x idx=%u",
                 i, name, toc[i].b_off, toc[i].m_off, toc[i].size,
                 toc[i].crc, toc[i].idx);
    }

    size_t nv_norm_size = 0;
    size_t nv_prot_size = 0;
    uint8_t *nv_norm = read_file(NV_NORM_PATH, &nv_norm_size);
    uint8_t *nv_prot = read_file(NV_PROT_PATH, &nv_prot_size);
    if (!nv_norm || !nv_prot) {
        die("NV userdata copy unreadable");
    }
    log_line("NV copy nv_normal.bin=%zu nv_protected.bin=%zu",
             nv_norm_size, nv_prot_size);

    if (do_ioctl("IOCTL_POWER_ON", IOCTL_POWER_ON, NULL) < 0) {
        die("POWER_ON failed");
    }

    /*
     * Factory start_shannon5100_boot does not call std_security_req and the
     * PCIE link leaves ld->security_req null. POWER_RESET is also absent:
     * stock goes POWER_ON -> LOAD BOOT -> START. Calling POWER_RESET after
     * POWER_ON performs a second GPIO power cycle and tears down PCIe state.
     *
     * LOAD_CP_IMAGE on PCIE copies into a small staging buffer and sets
     * boot_img_size = img.size *before* the range check. A failed MAIN/NV
     * load therefore clobbers the BOOT size used by set_cp_rom_boot_img.
     * Only BOOT may use this ioctl. MAIN/VSS/APM/NV need the SIT UDL path.
     */
    log_line("REQ_SECURITY skipped (PCIE security_req is null)");

    const struct toc_entry *boot = find_toc(toc, toc_count, "BOOT");
    if (!boot || boot->b_off == 0 || boot->size == 0 ||
        (uint64_t)boot->b_off + boot->size > bin_size) {
        die("BOOT TOC unusable");
    }
    /*
     * std_boot_load_cp_bootloader seeks the image fd to toc.b_off, but its
     * packed cp_image has m_offset=0, b_offset=0, mode=0, len=size.
     */
    if (load_image("BOOT", bin + boot->b_off, boot->size, 0, 0) < 0) {
        die("LOAD BOOT failed");
    }

    struct boot_mode mode = { .idx = CP_BOOT_MODE_NORMAL };
    log_line("boot_stage checkpoint before START (next kernel sample should begin 0x0)");
    print_modem_state("before-start");
    mode.idx = CP_BOOT_MODE_NORMAL;
    if (do_ioctl("IOCTL_START_CP_BOOTLOADER",
                 IOCTL_START_CP_BOOTLOADER, &mode) < 0) {
        die("START_CP_BOOTLOADER failed");
    }
    log_line("boot_stage checkpoint after START (kernel requires DONE_MASK=0x3fff)");
    print_modem_state("after-start");
    log_line("boot_stage checkpoint before first MAIN BIN");

    static const char *const udl_from_bin[] = { "MAIN", "VSS", "APM", "INFO" };
    for (size_t i = 0; i < sizeof(udl_from_bin) / sizeof(udl_from_bin[0]); ++i) {
        const struct toc_entry *entry = find_toc(toc, toc_count, udl_from_bin[i]);
        if (!entry || entry->b_off == 0 || entry->size == 0 ||
            (uint64_t)entry->b_off + entry->size > bin_size) {
            log_line("UDL %s: skip (missing on modem.bin)", udl_from_bin[i]);
            continue;
        }
        if (sit_send_stage(entry->idx, udl_from_bin[i],
                           bin + entry->b_off, entry->size, entry->crc) < 0) {
            log_line("UDL %s failed — abort (no further stages, no COMPLETE)",
                     udl_from_bin[i]);
            print_modem_state("bin-fail");
            return 1;
        }
    }

    const struct toc_entry *nvn = find_toc(toc, toc_count, "NV_NORM");
    const struct toc_entry *nvp = find_toc(toc, toc_count, "NV_PROT");
    if (nvn && nv_norm && nv_norm_size > 0) {
        if (sit_send_stage(nvn->idx, "NV_NORM", nv_norm,
                           (uint32_t)nv_norm_size, nvn->crc) < 0) {
            log_line("UDL NV_NORM failed — continuing");
        }
    }
    if (nvp && nv_prot && nv_prot_size > 0) {
        if (sit_send_stage(nvp->idx, "NV_PROT", nv_prot,
                           (uint32_t)nv_prot_size, nvp->crc) < 0) {
            log_line("UDL NV_PROT failed — continuing");
        }
    }

    log_line("UDL finish handshake READY 0x%x then FIN 0x%x",
             SIT_READY, SIT_FIN);
    if (sit_req_resp(SIT_READY, SIT_READY_ACK, SIT_ACK_DEADLINE_MS) < 0) {
        log_line("UDL READY fail");
    }
    if (sit_req_resp(SIT_FIN, SIT_FIN_ACK, SIT_ACK_DEADLINE_MS) < 0) {
        log_line("UDL FIN fail");
    }

    (void)do_ioctl("IOCTL_COMPLETE_NORMAL_BOOTUP",
                   IOCTL_COMPLETE_NORMAL_BOOTUP, NULL);
    print_modem_state("after");

    int status = ioctl(boot_fd, IOCTL_GET_CP_STATUS, NULL);
    log_line("GET_CP_STATUS after=%d", status);

    log_line("stock s5100sit closes boot args after COMPLETE; no ipc/rfs holder");
    return 0;
}

int main(int argc, char **argv) {
    const char *cmd = argc > 1 ? argv[1] : "load";
    log_line("SaaiOS panther cp-boot");
    log_line("forbidden: IOCTL_POWER_OFF, original efs, vendor cbd/rild");
    if (strcmp(cmd, "status") == 0) {
        print_status();
        return 0;
    }
    if (strcmp(cmd, "load") == 0) {
        return cmd_load();
    }
    die("usage: cp-boot [status|load]");
}
