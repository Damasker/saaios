#define _GNU_SOURCE
#ifndef SAAIOS_DIAGNOSTIC_WRAPPER
#error "Diagnostic support only; compile cp-boot-probe.c, never this file alone"
#endif

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <net/if.h>
#include <netinet/in.h>
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
 * 0xC00C write. format=IPC_BOOT forces EXYNOS SINGLE (wire 0xC018).
 * Live 2026-09-21: that frame is consumed (TX tail catches head) and the
 * CP sends no 0xC12B within 30s. Live 2026-09-22: SZ_2K frames are ACKed,
 * including past the old third ring wrap, then the CP stops reading at
 * MAIN offset 0x201b098 with one 0x800 frame left queued.
 *
 * Live 2026-09-22: 0xC000 frames are consumed with no BIN ACK, then the CP
 * publishes zero RX and the driver raises CP_CRASH_REQ (BAD CFG 0x00).
 * That happened on the first wrapping frame (c48, after 42 frames) and
 * again on a non-wrapping frame (fit48, after 3 frames). SZ_2K frames
 * that end on the ring boundary are ACKed through MAIN offset 0x201b098
 * and the CP stays BOOTING. This load uses that path. On the first BIN
 * ACK timeout it sends the stage CRC once and logs the reply.
 */
#define EXYNOS_HEADER_SIZE 12u
#define SIT_HDR_SIZE 12u
/* Payload 0x7E8: wire EXYNOS+SIT+payload = 0x800. Ring-fit so a frame
 * never wraps 0x1FD000. A BIN chunk counts only on 0xC12B. This is the
 * path that stalls at MAIN offset 0x201b098. */
#define SIT_CHUNK 0x7E8u
#define SIT_BIN_REQUIRE_ACK 1
#define SIT_CONSUME_DEADLINE_MS 15000
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
        ssize_t n = read(boot_fd, buf + done, 4 - done);
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

/* NORM_RAW TX pointers. The CP owns the tail; head is where the next
 * EXYNOS frame will be written. head==tail means the CP has consumed
 * every published frame. */
static int norm_raw_tx_ptrs(uint32_t *head_out, uint32_t *tail_out) {
    FILE *file = fopen("/sys/devices/platform/cpif/legacy/status", "r");
    if (!file) {
        return -1;
    }
    char line[256];
    int in_raw = 0;
    int found = -1;
    while (fgets(line, sizeof(line), file)) {
        if (strstr(line, "name:NORM_RAW")) {
            in_raw = 1;
        } else if (strncmp(line, "ID:", 3) == 0) {
            in_raw = 0;
        }
        if (!in_raw) {
            continue;
        }
        unsigned head = 0;
        unsigned tail = 0;
        if (sscanf(line, "TX busy:%*d head:%u tail:%u", &head, &tail) == 2) {
            if (head_out) {
                *head_out = head;
            }
            if (tail_out) {
                *tail_out = tail;
            }
            found = 0;
        }
    }
    fclose(file);
    return found;
}

static int norm_raw_tx_head(uint32_t *head_out) {
    return norm_raw_tx_ptrs(head_out, NULL);
}

/* Read any 4-byte ACKs already queued. Does not block in bootdump_read. */
static int sit_take_acks(uint32_t exp) {
    if (sit_poll_in(0) <= 0) {
        return 0;
    }
    uint8_t buf[64];
    ssize_t n = read(boot_fd, buf, sizeof(buf));
    if (n < 4) {
        return 0;
    }
    int good = 0;
    for (ssize_t i = 0; i + 4 <= n; i += 4) {
        uint32_t ack = 0;
        memcpy(&ack, buf + i, 4);
        if (ack == exp) {
            good++;
        } else {
            log_line("SIT RX during BIN 0x%08x (expect 0x%08x)", ack, exp);
        }
    }
    return good;
}

/* Factory cbd blocks on a 4-byte BIN ACK. Live CP consumes a 0xC018
 * SINGLE and does not send that ACK. Progress is tail==head. */
static int sit_wait_bin_landed(uint32_t exp_ack, uint32_t off, uint32_t chunk_i,
                               uint32_t head_before) {
    int64_t start = sit_now_ms();
    int64_t last_log = 0;
    int acks = 0;
    while (sit_now_ms() - start < SIT_CONSUME_DEADLINE_MS) {
        int n = sit_take_acks(exp_ack);
        if (n > 0) {
            acks += n;
        }
        uint32_t head = 0;
        uint32_t tail = 0;
        int ptrs = norm_raw_tx_ptrs(&head, &tail);
        /* ACK without consume is not progress: the previous 0xC000 frame
         * was taken from the ring and never ACKed. Wait until tail==head. */
        /* head==tail at the pre-write pointer means sysfs has not
         * observed this write yet. One frame cannot wrap the ring
         * back onto the same head. */
        if (ptrs == 0 && head == tail && head != head_before) {
            return acks;
        }
        int64_t now = sit_now_ms();
        if (last_log == 0) {
            last_log = now;
        }
        if (now - last_log >= 1000) {
            log_line("UDL consume wait off=0x%x chunk=%u head=0x%x tail=0x%x acks=%d",
                     off, chunk_i, head, tail, acks);
            last_log = now;
        }
        usleep(50000);
    }
    return -1;
}

/* Payload that keeps EXYNOS+SIT+payload inside the ring. A frame that
 * would wrap is shortened so it ends exactly at 0x1FD000. */
static uint32_t chunk_fit_ring(uint32_t head, uint32_t remain) {
    uint32_t chunk = remain > SIT_CHUNK ? SIT_CHUNK : remain;
    uint32_t wire = EXYNOS_HEADER_SIZE + SIT_HDR_SIZE + chunk;
    uint32_t room = LEGACY_RAW_TXQ_SIZE - head;
    if (wire <= room) {
        return chunk;
    }
    if (room <= EXYNOS_HEADER_SIZE + SIT_HDR_SIZE) {
        return chunk;
    }
    return room - EXYNOS_HEADER_SIZE - SIT_HDR_SIZE;
}

static int modem_is_online(void) {
    char state[64];
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    return strcmp(state, "ONLINE") == 0;
}

static void log_rmnet_bearer(void) {
    char rx[64];
    char tx[64];
    read_trimmed("/sys/class/net/rmnet0/statistics/rx_bytes", rx, sizeof(rx));
    read_trimmed("/sys/class/net/rmnet0/statistics/tx_bytes", tx, sizeof(tx));
    log_line("rmnet0 rx_bytes=%s tx_bytes=%s", rx, tx);
    int fd = socket(AF_INET, SOCK_DGRAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        log_line("rmnet0 ipv4 socket: %s", strerror(errno));
        return;
    }
    struct ifreq req;
    memset(&req, 0, sizeof(req));
    snprintf(req.ifr_name, sizeof(req.ifr_name), "rmnet0");
    if (ioctl(fd, SIOCGIFADDR, &req) == 0) {
        struct sockaddr_in *addr = (struct sockaddr_in *)&req.ifr_addr;
        log_line("rmnet0 ipv4 %s", inet_ntoa(addr->sin_addr));
    } else {
        log_line("rmnet0 ipv4 none errno=%d (%s)", errno, strerror(errno));
    }
    close(fd);
}

/* Ring is empty: the frame at this offset was not written. COMPLETE,
 * then FIN, then READY. Stop after that. No more BIN. */
static void probe_finish_at_wall(void) {
    int saved = 0;
    int rc = ioctl(boot_fd, IOCTL_COMPLETE_NORMAL_BOOTUP, NULL);
    saved = errno;
    if (rc < 0) {
        log_line("IOCTL_COMPLETE_NORMAL_BOOTUP probe rc=%d errno=%d (%s)",
                 rc, saved, strerror(saved));
    } else {
        log_line("IOCTL_COMPLETE_NORMAL_BOOTUP probe rc=%d", rc);
    }
    print_modem_state("after-complete-probe");
    if (!modem_is_online()) {
        log_line("wall probe FIN 0x%x expect 0x%x", SIT_FIN, SIT_FIN_ACK);
        if (sit_req_resp(SIT_FIN, SIT_FIN_ACK, 5000) < 0) {
            log_line("wall probe FIN no match");
        }
        print_modem_state("after-fin-probe");
    }
    if (!modem_is_online()) {
        log_line("wall probe READY 0x%x expect 0x%x", SIT_READY, SIT_READY_ACK);
        if (sit_req_resp(SIT_READY, SIT_READY_ACK, 5000) < 0) {
            log_line("wall probe READY no match");
        }
        print_modem_state("after-ready-probe");
    }
    log_rmnet_bearer();
}

static int dmesg_has_bad_cfg(void);

/* IPC shared region and the MSI block the ROM uses for the BOOT image.
 * Neither holds a MAIN m_off/size descriptor; this dump only reads them. */
#define IPC_PHYS_BASE 0xea400000ul
#define MSI_PHYS_BASE 0xf6200000ul
#define IPC_DUMP_PATH "/data/saaios/var/cp-boot-ipc-20260924.txt"

static uint8_t snap_ipc[256];
static uint8_t snap_msi[32];
static char snap_status_txt[8192];
static char snap_info_txt[4096];
static int snap_saved;
static int snap_status;

static int read_phys(unsigned long addr, uint8_t *buf, size_t len) {
    int fd = open("/dev/mem", O_RDONLY | O_SYNC | O_CLOEXEC);
    if (fd < 0) {
        log_line("devmem open: %s", strerror(errno));
        return -1;
    }
    ssize_t n = pread(fd, buf, len, (off_t)addr);
    int saved = errno;
    close(fd);
    if (n != (ssize_t)len) {
        log_line("devmem pread 0x%lx n=%zd errno=%d (%s)",
                 addr, n, saved, n < 0 ? strerror(saved) : "short");
        return -1;
    }
    return 0;
}

static void write_dump_block(FILE *out, const char *when, const char *tag,
                             unsigned long addr, const uint8_t *buf, size_t len) {
    fprintf(out, "%s %s phys=0x%lx len=%zu\n", when, tag, addr, len);
    for (size_t i = 0; i < len; i += 16) {
        fprintf(out, "%04zx", i);
        for (size_t j = 0; j < 16 && i + j < len; ++j) {
            fprintf(out, " %02x", buf[i + j]);
        }
        fputc('\n', out);
    }
}

static void log_u32s(const char *when, const char *tag, const uint8_t *buf, int nwords) {
    char line[256];
    int used = snprintf(line, sizeof(line), "diag %s %s", when, tag);
    for (int i = 0; i < nwords && used < (int)sizeof(line) - 16; ++i) {
        uint32_t word = 0;
        memcpy(&word, buf + (size_t)i * 4, 4);
        used += snprintf(line + used, sizeof(line) - (size_t)used, " [%d]=0x%x", i, word);
    }
    log_line("%s", line);
}

static void dump_ipc_diag(const char *when) {
    uint8_t ipc[256];
    uint8_t msi[32];
    memset(ipc, 0, sizeof(ipc));
    memset(msi, 0, sizeof(msi));
    int ipc_ok = read_phys(IPC_PHYS_BASE, ipc, sizeof(ipc));
    int msi_ok = read_phys(MSI_PHYS_BASE, msi, sizeof(msi));
    int status = ioctl(boot_fd, IOCTL_GET_CP_STATUS, NULL);
    log_line("diag %s GET_CP_STATUS=%d devmem_ipc=%d devmem_msi=%d bad_cfg=%d",
             when, status, ipc_ok, msi_ok, dmesg_has_bad_cfg());
    print_modem_state(when);
    print_legacy_status(when, 0, 0);
    if (ipc_ok == 0) {
        log_u32s(when, "ipc", ipc, 10);
    }
    if (msi_ok == 0) {
        log_u32s(when, "msi", msi, 8);
    }
    char status_txt[8192];
    char info_txt[4096];
    status_txt[0] = '\0';
    info_txt[0] = '\0';
    FILE *status_file = fopen("/sys/devices/platform/cpif/legacy/status", "r");
    if (!status_file) {
        log_line("diag %s legacy status: %s", when, strerror(errno));
    } else {
        size_t n = fread(status_txt, 1, sizeof(status_txt) - 1, status_file);
        status_txt[n] = '\0';
        fclose(status_file);
    }
    FILE *info = fopen("/sys/devices/platform/cpif/info_region", "r");
    if (!info) {
        log_line("diag %s info_region: %s", when, strerror(errno));
    } else {
        size_t n = fread(info_txt, 1, sizeof(info_txt) - 1, info);
        info_txt[n] = '\0';
        fclose(info);
        char line[256];
        size_t pos = 0;
        while (info_txt[pos] != '\0') {
            size_t k = 0;
            while (info_txt[pos] != '\0' && info_txt[pos] != '\n' && k + 1 < sizeof(line)) {
                line[k++] = info_txt[pos++];
            }
            if (info_txt[pos] == '\n') {
                pos++;
            }
            line[k] = '\0';
            log_line("diag %s info %s", when, line);
        }
    }
    FILE *kmsg = popen("dmesg", "r");
    char boot_lines[1024];
    boot_lines[0] = '\0';
    if (!kmsg) {
        log_line("diag %s dmesg: %s", when, strerror(errno));
    } else {
        char line[512];
        while (fgets(line, sizeof(line), kmsg)) {
            if (!strstr(line, "boot_stage") && !strstr(line, "err_report") &&
                !strstr(line, "boot_img")) {
                continue;
            }
            line[strcspn(line, "\r\n")] = '\0';
            log_line("diag %s dmesg %s", when, line);
            size_t used = strlen(boot_lines);
            if (used + strlen(line) + 2 < sizeof(boot_lines)) {
                memcpy(boot_lines + used, line, strlen(line));
                boot_lines[used + strlen(line)] = '\n';
                boot_lines[used + strlen(line) + 1] = '\0';
            }
        }
        pclose(kmsg);
    }
    FILE *out = fopen(IPC_DUMP_PATH, snap_saved ? "a" : "w");
    if (!out) {
        log_line("diag file: %s", strerror(errno));
    } else {
        fprintf(out, "== %s GET_CP_STATUS=%d ipc=%d msi=%d\n",
                when, status, ipc_ok, msi_ok);
        fputs("--- legacy/status\n", out);
        fputs(status_txt, out);
        fputs("--- info_region\n", out);
        fputs(info_txt, out);
        fputs("--- dmesg boot\n", out);
        fputs(boot_lines, out);
        if (snap_saved && strcmp(status_txt, snap_status_txt) != 0) {
            fputs("legacy/status changed since after-start\n", out);
            log_line("diag %s legacy/status changed since after-start", when);
        } else if (snap_saved) {
            log_line("diag %s legacy/status unchanged since after-start", when);
        }
        if (snap_saved && strcmp(info_txt, snap_info_txt) != 0) {
            fputs("info_region changed since after-start\n", out);
            log_line("diag %s info_region changed since after-start", when);
        } else if (snap_saved) {
            log_line("diag %s info_region unchanged since after-start", when);
        }
        if (ipc_ok == 0) {
            write_dump_block(out, when, "ipc", IPC_PHYS_BASE, ipc, sizeof(ipc));
        }
        if (msi_ok == 0) {
            write_dump_block(out, when, "msi", MSI_PHYS_BASE, msi, sizeof(msi));
        }
        if (snap_saved && ipc_ok == 0) {
            for (int i = 0; i < 64; ++i) {
                uint32_t a = 0;
                uint32_t b = 0;
                memcpy(&a, snap_ipc + (size_t)i * 4, 4);
                memcpy(&b, ipc + (size_t)i * 4, 4);
                if (a != b) {
                    fprintf(out, "ipc changed [%d] +0x%x 0x%x -> 0x%x\n",
                            i, i * 4, a, b);
                    log_line("diag ipc changed [%d] +0x%x 0x%x -> 0x%x",
                             i, i * 4, a, b);
                }
            }
        }
        if (snap_saved && msi_ok == 0) {
            for (int i = 0; i < 8; ++i) {
                uint32_t a = 0;
                uint32_t b = 0;
                memcpy(&a, snap_msi + (size_t)i * 4, 4);
                memcpy(&b, msi + (size_t)i * 4, 4);
                if (a != b) {
                    fprintf(out, "msi changed [%d] +0x%x 0x%x -> 0x%x\n",
                            i, i * 4, a, b);
                    log_line("diag msi changed [%d] +0x%x 0x%x -> 0x%x",
                             i, i * 4, a, b);
                }
            }
        }
        if (snap_saved && status != snap_status) {
            fprintf(out, "GET_CP_STATUS %d -> %d\n", snap_status, status);
            log_line("diag GET_CP_STATUS %d -> %d", snap_status, status);
        }
        fflush(out);
        fsync(fileno(out));
        fclose(out);
    }
    if (!snap_saved) {
        if (ipc_ok == 0) {
            memcpy(snap_ipc, ipc, sizeof(snap_ipc));
        }
        if (msi_ok == 0) {
            memcpy(snap_msi, msi, sizeof(snap_msi));
        }
        snap_status = status;
        snprintf(snap_status_txt, sizeof(snap_status_txt), "%s", status_txt);
        snprintf(snap_info_txt, sizeof(snap_info_txt), "%s", info_txt);
        snap_saved = 1;
    }
    log_rmnet_bearer();
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
    uint32_t tx_head = 16;
    if (norm_raw_tx_head(&tx_head) < 0) {
        log_line("UDL %s: TX head unreadable, assuming 16", name);
    } else {
        log_line("UDL %s TX head before BIN 0x%x", name, tx_head);
    }
    uint8_t *frame = malloc(sizeof(struct sit_udl_hdr) + SIT_CHUNK);
    if (!frame) {
        die("oom UDL frame");
    }
    while (off < size) {
        /* No MAIN descriptor was missing. Stop before this offset and
         * do not write BIN, CRC, DONE, READY, FIN, or COMPLETE. */
        if (off >= 0x02400000u
#ifdef PROBE_FULL_MAIN
            && false
#endif
        ) {
            uint32_t head = 0;
            uint32_t tail = 0;
            int ptrs = norm_raw_tx_ptrs(&head, &tail);
            int empty = (ptrs == 0 && head == tail);
            log_line("UDL %s stop before BIN off=0x%x chunk=%u last_good=0x%x head=0x%x tail=0x%x empty=%d",
                     name, off, chunks, last_good, head, tail, empty);
            log_line("PROBE BOUND REACHED; no CRC and no further stages");
            free(frame);
            return 2;
        }
        uint32_t chunk = chunk_fit_ring(tx_head, size - off);
#ifndef PROBE_FULL_MAIN
        if (chunk > 0x02400000u - off) chunk = 0x02400000u - off;
#endif
        if (chunk != SIT_CHUNK && chunk != size - off) {
            log_line("UDL %s shrink payload 0x%x -> 0x%x so wire ends at ring (head 0x%x)",
                     name, SIT_CHUNK, chunk, tx_head);
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
        uint32_t head_before = tx_head;
        if (sit_write_bytes(frame, wrote) < 0) {
            log_line("UDL %s chunk write: %s", name, strerror(errno));
            free(frame);
            return -1;
        }
        tx_head += EXYNOS_HEADER_SIZE + (uint32_t)wrote;
        if (tx_head >= LEGACY_RAW_TXQ_SIZE) {
            tx_head -= LEGACY_RAW_TXQ_SIZE;
        }
#if SIT_BIN_REQUIRE_ACK
        if (sit_req_resp_ex(0, sit_cmd(SIT_BIN_ACK, idx), SIT_ACK_DEADLINE_MS,
                            off == 0) < 0) {
            uint32_t head = 0;
            uint32_t tail = 0;
            int ptrs_ok = norm_raw_tx_ptrs(&head, &tail);
            log_line("UDL %s BIN ACK fail at 0x%x chunk=%u last_good=0x%x head=0x%x tail=0x%x consumed=%d",
                     name, off, chunks, last_good, head, tail,
                     (ptrs_ok == 0 && head == tail) ? 1 : 0);
            print_legacy_status("bin-ack-fail", chunks, off);
            print_modem_state("bin-ack-fail");
            log_line("UDL %s bad_cfg=%d", name, dmesg_has_bad_cfg());
            free(frame);
            return -1;
        }
#else
        int landed = sit_wait_bin_landed(sit_cmd(SIT_BIN_ACK, idx), off, chunks,
                                         head_before);
        if (landed < 0) {
            uint32_t head = 0;
            uint32_t tail = 0;
            (void)norm_raw_tx_ptrs(&head, &tail);
            log_line("UDL %s BIN not consumed at 0x%x chunk=%u last_good=0x%x head=0x%x tail=0x%x",
                     name, off, chunks, last_good, head, tail);
            free(frame);
            return -1;
        }
        if (chunks < 3 || (chunks & 0x3fu) == 0) {
            log_line("UDL %s BIN landed off=0x%x chunk=%u acks=%d",
                     name, off, chunks, landed);
        }
#endif
        last_good = off;
        chunks++;
        off += chunk;
        if (last_good < 0x201b098u && off >= 0x201b098u) {
            log_line("UDL %s ACKs continued past 0x201b098 next=0x%x chunk=%u",
                     name, off, chunks);
            print_legacy_status("past-wall", chunks, last_good);
        }
        if ((chunks & 0x1ffu) == 0) {
            print_legacy_status("progress", chunks, last_good);
            char state[64];
            read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
            if (strcmp(state, "BOOTING") != 0) {
                log_line("UDL %s state left BOOTING: %s at off=0x%x",
                         name, state, off);
                free(frame);
                return -1;
            }
        }
    }
    free(frame);

    if (strcmp(name, "MAIN") == 0) {
    sit_drain_rx("pre-crc");
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

static void log_no_reset_sample(int sec) {
    print_modem_state(sec > 0 ? "no-reset-wait" : "no-reset");
    FILE *file = fopen("/sys/devices/platform/cpif/legacy/status", "r");
    if (!file) {
        log_line("no-reset t=%d legacy status missing", sec);
        return;
    }
    char line[256];
    int in_raw = 0;
    log_line("no-reset t=%d", sec);
    while (fgets(line, sizeof(line), file)) {
        if (strstr(line, "name:NORM_RAW")) {
            in_raw = 1;
            continue;
        }
        if (strncmp(line, "ID:", 3) == 0) {
            in_raw = 0;
        }
        if (!in_raw) {
            continue;
        }
        if (strncmp(line, "TX ", 3) == 0 || strncmp(line, "RX ", 3) == 0) {
            line[strcspn(line, "\r\n")] = '\0';
            log_line("  %s", line);
        }
    }
    fclose(file);
}

/* 1 = BAD CFG present, 0 = not present, -1 = dmesg unreadable. */
static int dmesg_has_bad_cfg(void) {
    FILE *pipe = popen("dmesg", "r");
    if (!pipe) {
        log_line("dmesg popen: %s", strerror(errno));
        return -1;
    }
    char line[512];
    int found = 0;
    while (fgets(line, sizeof(line), pipe)) {
        if (strstr(line, "BAD CFG")) {
            found = 1;
            break;
        }
    }
    int rc = pclose(pipe);
    if (rc < 0 && !found) {
        log_line("dmesg pclose: %s", strerror(errno));
        return -1;
    }
    return found;
}

/* One factory-sized MAIN BIN. No second frame, no COMPLETE. */
static int send_one_c000_bin(uint32_t idx, const uint8_t *data, uint32_t size) {
    uint32_t chunk = 0xC000u;
    uint32_t start = sit_cmd(SIT_START, idx);
    uint32_t start_ack = sit_cmd(SIT_START_ACK, idx);
    log_line("one BIN MAIN start=0x%x expect=0x%x size=0x%x", start, start_ack, size);
    if (sit_req_resp(start, start_ack, SIT_START_DEADLINE_MS) < 0) {
        log_line("one BIN START fail");
        return -1;
    }
    uint8_t *frame = malloc(sizeof(struct sit_udl_hdr) + chunk);
    if (!frame) {
        die("oom one BIN");
    }
    struct sit_udl_hdr *hdr = (struct sit_udl_hdr *)frame;
    memset(frame, 0, sizeof(*hdr) + chunk);
    hdr->cmd = (uint16_t)sit_cmd(SIT_BIN, idx);
    hdr->total = size;
    hdr->offset = 0;
    memcpy(frame + sizeof(*hdr), data, chunk);
    hdr->len = (uint16_t)(chunk + 8);
    size_t wrote = sizeof(*hdr) + chunk;
    log_line("one BIN write=0x%zx cmd=0x%x len=0x%x (no shrink)",
             wrote, hdr->cmd, hdr->len);
    if (sit_write_bytes(frame, wrote) < 0) {
        log_line("one BIN write failed");
        free(frame);
        return -1;
    }
    free(frame);
    uint32_t ack = sit_cmd(SIT_BIN_ACK, idx);
    int got = sit_req_resp(0, ack, 10000);
    uint32_t head = 0;
    uint32_t tail = 0;
    int ptrs = norm_raw_tx_ptrs(&head, &tail);
    int bad = dmesg_has_bad_cfg();
    char state[64];
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (got == 0) {
        log_line("one BIN result: ACK 0x%x", ack);
    } else if (ptrs == 0 && head == tail) {
        log_line("one BIN result: consumed no ACK head=0x%x tail=0x%x", head, tail);
    } else {
        log_line("one BIN result: no ACK head=0x%x tail=0x%x ptrs=%d", head, tail, ptrs);
    }
    log_line("one BIN after: bad_cfg=%d modem_state=%s", bad, state);
    if (strcmp(state, "CRASH_EXIT") == 0 || bad == 1) {
        log_line("one BIN followed by %s",
                 strcmp(state, "CRASH_EXIT") == 0 ? "CRASH_EXIT" : "BAD CFG");
    }
    return got;
}

#if 0 /* Preserve historical snapshot for provenance, never expose its loader. */
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
    log_line("POWER_RESET skipped; wait 3s before START");
    for (int sec = 1; sec <= 3; ++sec) {
        sleep(1);
        log_no_reset_sample(sec);
    }
    int bad_cfg = dmesg_has_bad_cfg();
    char state_now[64];
    read_trimmed(MODEM_STATE_PATH, state_now, sizeof(state_now));
    if (bad_cfg < 0 || bad_cfg == 1 || strcmp(state_now, "OFFLINE") != 0) {
        log_line("no-reset: not starting (bad_cfg=%d modem_state=%s)",
                 bad_cfg, state_now);
        print_modem_state("no-start");
        free(bin);
        free(nv_norm);
        free(nv_prot);
        return bad_cfg < 0 ? 1 : 0;
    }

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
    log_no_reset_sample(0);
    if (dmesg_has_bad_cfg() == 1) {
        log_line("no-reset: BAD CFG after START, no BIN");
        print_modem_state("after-start-bad");
        free(bin);
        free(nv_norm);
        free(nv_prot);
        return 1;
    }
    read_trimmed(MODEM_STATE_PATH, state_now, sizeof(state_now));
    if (strcmp(state_now, "BOOTING") != 0) {
        log_line("no-reset: state after START is %s, no BIN", state_now);
        free(bin);
        free(nv_norm);
        free(nv_prot);
        return 1;
    }
    dump_ipc_diag("after-start");

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
            log_line("UDL %s failed — stop, no further stages", udl_from_bin[i]);
            print_modem_state("bin-fail");
            free(bin);
            free(nv_norm);
            free(nv_prot);
            return 1;
        }
    }

    const struct toc_entry *nvn = find_toc(toc, toc_count, "NV_NORM");
    const struct toc_entry *nvp = find_toc(toc, toc_count, "NV_PROT");
    if (nvn && nv_norm && nv_norm_size > 0) {
        if (sit_send_stage(nvn->idx, "NV_NORM", nv_norm,
                           (uint32_t)nv_norm_size, nvn->crc) < 0) {
            log_line("UDL NV_NORM failed — stop");
            print_modem_state("nv-fail");
            free(bin);
            free(nv_norm);
            free(nv_prot);
            return 1;
        }
    }
    if (nvp && nv_prot && nv_prot_size > 0) {
        if (sit_send_stage(nvp->idx, "NV_PROT", nv_prot,
                           (uint32_t)nv_prot_size, nvp->crc) < 0) {
            log_line("UDL NV_PROT failed — stop");
            print_modem_state("nv-fail");
            free(bin);
            free(nv_norm);
            free(nv_prot);
            return 1;
        }
    }

    log_line("UDL finish handshake READY 0x%x then FIN 0x%x",
             SIT_READY, SIT_FIN);
    if (sit_req_resp(SIT_READY, SIT_READY_ACK, SIT_ACK_DEADLINE_MS) < 0) {
        log_line("UDL READY fail — stop");
        print_modem_state("ready-fail");
        free(bin);
        free(nv_norm);
        free(nv_prot);
        return 1;
    }
    if (sit_req_resp(SIT_FIN, SIT_FIN_ACK, SIT_ACK_DEADLINE_MS) < 0) {
        log_line("UDL FIN fail — stop");
        print_modem_state("fin-fail");
        free(bin);
        free(nv_norm);
        free(nv_prot);
        return 1;
    }

    (void)do_ioctl("IOCTL_COMPLETE_NORMAL_BOOTUP",
                   IOCTL_COMPLETE_NORMAL_BOOTUP, NULL);
    print_modem_state("after");
    log_rmnet_bearer();

    int status = ioctl(boot_fd, IOCTL_GET_CP_STATUS, NULL);
    log_line("GET_CP_STATUS after=%d", status);
    free(bin);
    free(nv_norm);
    free(nv_prot);
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
#endif
