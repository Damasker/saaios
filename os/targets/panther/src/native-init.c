#define _GNU_SOURCE

#include <arpa/inet.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/reboot.h>
#include <linux/watchdog.h>
#include <net/if.h>
#include <signal.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <termios.h>
#include <time.h>
#include <unistd.h>

#define ARRAY_SIZE(x) (sizeof(x) / sizeof((x)[0]))

static const char *const restart_modules[] = {
    "logbuffer.ko",
    "google-bms.ko",
    "exynos-pd_el3.ko",
    "pixel-reboot.ko",
};

static const char *const storage_modules[] = {
    "systrace.ko",
    "sbb-mux.ko",
    "ufs-pixel-fips140.ko",
    "kernel-top.ko",
    "pixel-suspend-diag.ko",
    "dss.ko",
    "gs_acpm.ko",
    "ect_parser.ko",
    "gs-chipid.ko",
    "exynos-pmu-if.ko",
    "exynos-adv-tracer.ko",
    "etm2dram.ko",
    "s3c2410_wdt.ko",
    "cmupmucal.ko",
    "exynos_pm_qos.ko",
    "pinctrl-exynos-gs.ko",
    "sched_tp.ko",
    "gs_perf_mon.ko",
    "vh_sched.ko",
    "pixel_metrics.ko",
    "exynos-cpupm.ko",
    "trusty-core.ko",
    "trusty-ipc.ko",
    "gsa.ko",
    "ufs-exynos-gs.ko",
};

static const char *const usb_modules[] = {
    "gvotable.ko",
    "clk_exynos_gs.ko",
    "i2c-acpm.ko",
    "i2c-exynos5.ko",
    "s2mpg13-mfd.ko",
    "s2mpg12-mfd.ko",
    "s2mpg12-key.ko",
    "pmic_class.ko",
    "s2mpg13-regulator.ko",
    "s2mpg12-regulator.ko",
    "s2mpg1x-gpio.ko",
    "google_tcpci_shim.ko",
    "max77759_helper.ko",
    "max77759_contaminant.ko",
    "max77779_contaminant.ko",
    "max777x9_contaminant.ko",
    "usb_psy.ko",
    "bc_max77759.ko",
    "max1720x-battery.ko",
    "max77729-pmic.ko",
    "max77759-charger.ko",
    "tcpci_max77759.ko",
    "google-cpm.ko",
    "exynos-pd_hsi0.ko",
    "phy-exynos-usbdrd-super.ko",
    "dwc3-exynos-usb.ko",
    "xhci-exynos.ko",
    "pkvm-s2mpu.ko",
    "exynos-pd.ko",
};

static const char *const display_modules[] = {
    "pixel_stat_sysfs.ko",
    "pixel_stat_mm.ko",
    "samsung-secure-iova.ko",
    "samsung_dma_heap.ko",
    "drm_display_helper.ko",
    "phy-exynos-mipi-dsim.ko",
    "phy-exynos-mipi.ko",
    "samsung_iommu.ko",
    "samsung-iommu-group.ko",
    "exynos_tty.ko",
    "exynos-dm.ko",
    "exynos_devfreq.ko",
    "gs-drm-connector.ko",
    "itmon.ko",
    "bts.ko",
    "exynos-drm.ko",
    "panel-samsung-drv.ko",
    "panel-samsung-s6e3fc3-p10.ko",
};

static const char *const touch_modules[] = {
    "pl330.ko",
    "samsung-dma.ko",
    "spi-s3c64xx.ko",
    "touch_bus_negotiator.ko",
    "heatmap.ko",
    "touch_offload.ko",
    "/saaios/focal_touch.ko",
};

static const char *const wifi_modules[] = {
    "acpm_flexpmu_dbg.ko",
    "exynos-pm.ko",
    "pcie-exynos-gs201-rc-cal.ko",
    "exynos-pcie-iommu.ko",
    "pcie-exynos-gs.ko",
    "rfkill.ko",
    "cfg80211.ko",
    "bcmdhd4389.ko",
};

static const char *const bluetooth_modules[] = {
    "bluetooth.ko",
    "btqca.ko",
    "btbcm.ko",
    "hci_uart.ko",
};

static const char *const audio_modules[] = {
    "gsa_gsc.ko",
    "trusty-log.ko",
    "trusty-virtio.ko",
    "audiometrics.ko",
    "mailbox-wc.ko",
    "aoc_core.ko",
    "aoc_char_dev.ko",
    "aoc_control_dev.ko",
    "aoc_channel_dev.ko",
    "google_modemctl.ko",
    "exynos-drm-audio.ko",
    "aoc_alsa_dev_util.ko",
    "aoc_alsa_dev.ko",
    "aoc_usb_driver.ko",
    "snd-soc-wm-adsp.ko",
    "snd-soc-cs35l41.ko",
    "snd-soc-cs35l41-spi.ko",
    "cl_dsp-core.ko",
    "cs40l26-core.ko",
    "cs40l26-i2c.ko",
    "snd-soc-cs40l26.ko",
};

static bool metadata_ready = false;

static void mkdir_one(const char *path, mode_t mode) {
    if (mkdir(path, mode) < 0 && errno != EEXIST) {
        return;
    }
}

static void write_raw(const char *path, const char *text) {
    int fd = open(path, O_WRONLY | O_CLOEXEC | O_APPEND);
    if (fd < 0) {
        return;
    }
    (void)write(fd, text, strlen(text));
    close(fd);
}

static void log_message(const char *format, ...) {
    char body[512];
    char line[576];
    va_list args;

    va_start(args, format);
    vsnprintf(body, sizeof(body), format, args);
    va_end(args);
    snprintf(line, sizeof(line), "saaios-init: %s\n", body);

    write_raw("/dev/kmsg", line);
    write_raw("/dev/console", line);
    write_raw("/dev/ttySAC0", line);
    write_raw("/run/boot.log", line);
    if (metadata_ready) {
        write_raw("/metadata/saaios/native-boot.log", line);
        sync();
    }
}

static int write_value(const char *path, const char *value) {
    int fd = open(path, O_WRONLY | O_CLOEXEC);
    if (fd < 0) {
        log_message("open %s failed: %s", path, strerror(errno));
        return -1;
    }
    ssize_t expected = (ssize_t)strlen(value);
    ssize_t written = write(fd, value, (size_t)expected);
    int saved_errno = errno;
    close(fd);
    if (written != expected) {
        log_message("write %s failed: %s", path, strerror(saved_errno));
        return -1;
    }
    return 0;
}

static int load_module_with_options(const char *name, const char *options) {
    char path[256];
    if (name[0] == '/') {
        snprintf(path, sizeof(path), "%s", name);
    } else {
        snprintf(path, sizeof(path), "/lib/modules/%s", name);
    }
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        log_message("module %s missing: %s", name, strerror(errno));
        return -1;
    }
    errno = 0;
    int result = (int)syscall(SYS_finit_module, fd, options, 0);
    int saved_errno = errno;
    close(fd);
    if (result == 0 || saved_errno == EEXIST) {
        log_message("module %s ready", name);
        return 0;
    }
    log_message("module %s failed: %s", name, strerror(saved_errno));
    return -1;
}

static int load_module(const char *name) {
    return load_module_with_options(name, "");
}

static void reboot_with_reason(const char *reason) {
    sync();
    (void)syscall(SYS_reboot,
                  LINUX_REBOOT_MAGIC1,
                  LINUX_REBOOT_MAGIC2,
                  LINUX_REBOOT_CMD_RESTART2,
                  reason);
    (void)syscall(SYS_reboot,
                  LINUX_REBOOT_MAGIC1,
                  LINUX_REBOOT_MAGIC2,
                  LINUX_REBOOT_CMD_RESTART,
                  NULL);
}

static void start_safety_timer(void) {
    pid_t child = fork();
    if (child != 0) {
        return;
    }
    sleep(180);
    if (access("/run/saaios.keep", F_OK) != 0) {
        log_message("safety timeout; restarting");
        reboot_with_reason("saaios-timeout");
    }
    _exit(0);
}

static void start_hardware_watchdog(void) {
    char value[64] = {0};
    int sysfs = open("/sys/class/watchdog/watchdog0/dev", O_RDONLY | O_CLOEXEC);
    if (sysfs < 0) {
        log_message("hardware watchdog did not appear: %s", strerror(errno));
        return;
    }
    ssize_t count = read(sysfs, value, sizeof(value) - 1);
    close(sysfs);
    unsigned int major_number = 0;
    unsigned int minor_number = 0;
    if (count <= 0 || sscanf(value, "%u:%u", &major_number, &minor_number) != 2) {
        log_message("hardware watchdog device number unavailable");
        return;
    }
    unlink("/dev/watchdog0");
    if (mknod("/dev/watchdog0", S_IFCHR | 0600,
              makedev(major_number, minor_number)) < 0) {
        log_message("hardware watchdog node failed: %s", strerror(errno));
        return;
    }
    int watchdog = open("/dev/watchdog0", O_WRONLY | O_CLOEXEC);
    if (watchdog < 0) {
        log_message("hardware watchdog open failed: %s", strerror(errno));
        return;
    }
    int timeout = 60;
    (void)ioctl(watchdog, WDIOC_SETTIMEOUT, &timeout);
    pid_t child = fork();
    if (child == 0) {
        for (;;) {
            (void)ioctl(watchdog, WDIOC_KEEPALIVE, 0);
            sleep(5);
        }
    }
    close(watchdog);
    if (child > 0) {
        log_message("hardware watchdog feeder started");
    }
}

static void mark_userspace_stable(void) {
    int fd = open("/run/saaios.keep",
                  O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                  0644);
    if (fd >= 0) {
        close(fd);
    }
}

static int create_partition_node(const char *partition_name,
                                 const char *device_node) {
    DIR *directory = opendir("/sys/class/block");
    if (!directory) {
        return -1;
    }
    struct dirent *entry;
    int result = -1;
    while ((entry = readdir(directory)) != NULL) {
        if (entry->d_name[0] == '.') {
            continue;
        }
        char uevent_path[256];
        snprintf(uevent_path, sizeof(uevent_path),
                 "/sys/class/block/%s/uevent", entry->d_name);
        int uevent = open(uevent_path, O_RDONLY | O_CLOEXEC);
        if (uevent < 0) {
            continue;
        }
        char contents[1024] = {0};
        ssize_t count = read(uevent, contents, sizeof(contents) - 1);
        close(uevent);
        if (count <= 0) {
            continue;
        }
        char expected[128];
        snprintf(expected, sizeof(expected), "PARTNAME=%s\n", partition_name);
        if (!strstr(contents, expected)) {
            continue;
        }
        unsigned int major_number = 0;
        unsigned int minor_number = 0;
        char *major_text = strstr(contents, "MAJOR=");
        char *minor_text = strstr(contents, "MINOR=");
        if (!major_text || !minor_text ||
            sscanf(major_text, "MAJOR=%u", &major_number) != 1 ||
            sscanf(minor_text, "MINOR=%u", &minor_number) != 1) {
            break;
        }
        unlink(device_node);
        result = mknod(device_node, S_IFBLK | 0600,
                       makedev(major_number, minor_number));
        break;
    }
    closedir(directory);
    return result;
}

static bool read_haptic_calibration(const char *path,
                                    char f0[9], char redc[9], char q[9]) {
    FILE *file = fopen(path, "r");
    if (!file) {
        return false;
    }
    char line[128];
    while (fgets(line, sizeof(line), file)) {
        char value[9] = {0};
        if (sscanf(line, "f0_measured: %8[0-9A-Fa-f]", value) == 1 &&
            strlen(value) == 8) {
            snprintf(f0, 9, "%s", value);
        } else if (sscanf(line, "redc_measured: %8[0-9A-Fa-f]", value) == 1 &&
                   strlen(value) == 8) {
            snprintf(redc, 9, "%s", value);
        } else if (sscanf(line, "q_measured: %8[0-9A-Fa-f]", value) == 1 &&
                   strlen(value) == 8) {
            snprintf(q, 9, "%s", value);
        }
    }
    fclose(file);
    return f0[0] != '\0' && redc[0] != '\0' && q[0] != '\0';
}

static void apply_haptic_factory_calibration(void) {
    const char *device = "/dev/saaios-persist";
    const char *mountpoint = "/run/saaios-persist";
    if (create_partition_node("persist", device) < 0) {
        log_message("haptic persist partition unavailable");
        return;
    }
    mkdir_one(mountpoint, 0700);
    unsigned long flags = MS_RDONLY | MS_NOSUID | MS_NODEV |
                          MS_NOEXEC | MS_NOATIME;
    if (mount(device, mountpoint, "ext4", flags, NULL) < 0) {
        log_message("haptic persist mount failed: %s", strerror(errno));
        unlink(device);
        return;
    }

    char f0[9] = {0};
    char redc[9] = {0};
    char q[9] = {0};
    bool ready = read_haptic_calibration(
        "/run/saaios-persist/haptics/cs40l26.cal", f0, redc, q);
    if (!ready) {
        ready = read_haptic_calibration(
            "/run/saaios-persist/haptics/cs40l26_backup.cal", f0, redc, q);
    }
    if (!ready) {
        log_message("haptic factory calibration unavailable");
    } else {
        const char *base =
            "/sys/devices/platform/10970000.hsi2c/i2c-8/8-0043/calibration";
        char path[256];
        snprintf(path, sizeof(path), "%s/f0_stored", base);
        int result = write_value(path, f0);
        snprintf(path, sizeof(path), "%s/redc_stored", base);
        result |= write_value(path, redc);
        snprintf(path, sizeof(path), "%s/q_stored", base);
        result |= write_value(path, q);
        if (result == 0) {
            log_message("haptic factory calibration applied");
        }
    }
    if (umount2(mountpoint, 0) < 0) {
        log_message("haptic persist unmount failed: %s", strerror(errno));
    }
    unlink(device);
}

static int current_slot_index(void) {
    int fd = open("/proc/bootconfig", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return -1;
    }
    char contents[16384] = {0};
    ssize_t count = read(fd, contents, sizeof(contents) - 1);
    close(fd);
    if (count <= 0) {
        return -1;
    }
    if (strstr(contents, "androidboot.slot_suffix = \"_a\"")) {
        return 0;
    }
    if (strstr(contents, "androidboot.slot_suffix = \"_b\"")) {
        return 1;
    }
    return -1;
}

static void mark_current_slot_successful(void) {
    int slot = current_slot_index();
    if (slot < 0) {
        log_message("current boot slot is unknown");
        return;
    }
    if (create_partition_node("devinfo", "/dev/saaios-devinfo") < 0) {
        log_message("devinfo partition was not found");
        return;
    }
    int fd = open("/dev/saaios-devinfo", O_RDWR | O_DSYNC | O_CLOEXEC);
    if (fd < 0) {
        log_message("devinfo open failed: %s", strerror(errno));
        return;
    }
    uint8_t devinfo[128] = {0};
    ssize_t count = pread(fd, devinfo, sizeof(devinfo), 0);
    uint32_t magic = (uint32_t)devinfo[0] |
                     ((uint32_t)devinfo[1] << 8) |
                     ((uint32_t)devinfo[2] << 16) |
                     ((uint32_t)devinfo[3] << 24);
    uint16_t major_version = (uint16_t)devinfo[4] |
                             ((uint16_t)devinfo[5] << 8);
    uint16_t minor_version = (uint16_t)devinfo[6] |
                             ((uint16_t)devinfo[7] << 8);
    if (count != (ssize_t)sizeof(devinfo) || magic != 0x49564544U ||
        major_version < 3 || (major_version == 3 && minor_version < 3)) {
        log_message("devinfo format is not supported");
        close(fd);
        return;
    }
    off_t flags_offset = 48 + (off_t)slot * 4 + 1;
    uint8_t flags = devinfo[flags_offset];
    flags |= 0x02;
    if (pwrite(fd, &flags, 1, flags_offset) != 1 || fsync(fd) < 0) {
        log_message("failed to mark slot %c successful: %s",
                    slot == 0 ? 'A' : 'B', strerror(errno));
        close(fd);
        return;
    }
    close(fd);
    log_message("slot %c marked successful", slot == 0 ? 'A' : 'B');
}

static int find_udc(char *result, size_t result_size) {
    DIR *directory = opendir("/sys/class/udc");
    if (!directory) {
        return -1;
    }
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (entry->d_name[0] == '.') {
            continue;
        }
        snprintf(result, result_size, "%s", entry->d_name);
        closedir(directory);
        return 0;
    }
    closedir(directory);
    return -1;
}

static void prepare_mounts(void) {
    struct stat etc_info;
    if (lstat("/etc", &etc_info) == 0 && S_ISLNK(etc_info.st_mode)) {
        (void)unlink("/etc");
    }
    mkdir_one("/etc", 0755);
    mkdir_one("/proc", 0755);
    mkdir_one("/sys", 0755);
    mkdir_one("/run", 0755);
    mkdir_one("/tmp", 01777);
    mkdir_one("/config", 0755);
    mkdir_one("/sys/kernel/debug", 0755);
    mkdir_one("/dev/pts", 0755);
    (void)mount("proc", "/proc", "proc", 0, NULL);
    (void)mount("sysfs", "/sys", "sysfs", 0, NULL);
    (void)mount("tmpfs", "/run", "tmpfs", MS_NOSUID | MS_NODEV, "mode=0755");
    (void)mount("tmpfs", "/tmp", "tmpfs", MS_NOSUID | MS_NODEV, "mode=1777");
    (void)mount("devpts", "/dev/pts", "devpts", 0, NULL);
    (void)mount("configfs", "/config", "configfs", 0, NULL);
    (void)mount("debugfs", "/sys/kernel/debug", "debugfs", 0, NULL);
    int resolv = open("/run/resolv.conf",
                      O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (resolv >= 0) {
        close(resolv);
    }
    (void)unlink("/etc/resolv.conf");
    (void)symlink("/run/resolv.conf", "/etc/resolv.conf");
    if (access("/dev/random", F_OK) != 0) {
        (void)mknod("/dev/random", S_IFCHR | 0666, makedev(1, 8));
    }
    int fd = open("/run/boot.log", O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (fd >= 0) {
        close(fd);
    }
}

static int create_char_node_from_sysfs(const char *sys_path,
                                       const char *path,
                                       mode_t mode) {
    char value[64] = {0};
    int fd = open(sys_path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return -1;
    }
    ssize_t count = read(fd, value, sizeof(value) - 1);
    close(fd);
    unsigned int major_number = 0;
    unsigned int minor_number = 0;
    if (count <= 0 ||
        sscanf(value, "%u:%u", &major_number, &minor_number) != 2) {
        return -1;
    }
    unlink(path);
    return mknod(path, S_IFCHR | mode,
                 makedev(major_number, minor_number));
}

static int create_misc_node(const char *name, const char *path) {
    char sys_path[160];
    snprintf(sys_path, sizeof(sys_path), "/sys/class/misc/%s/dev", name);
    return create_char_node_from_sysfs(sys_path, path, 0660);
}

static void copy_file(const char *source, const char *destination) {
    int input = open(source, O_RDONLY | O_CLOEXEC);
    if (input < 0) {
        return;
    }
    int output = open(destination,
                      O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                      0644);
    if (output < 0) {
        close(input);
        return;
    }
    char buffer[4096];
    ssize_t count;
    while ((count = read(input, buffer, sizeof(buffer))) > 0) {
        (void)write(output, buffer, (size_t)count);
    }
    close(output);
    close(input);
}

static void setup_metadata_log(void) {
    mkdir_one("/metadata", 0755);
    if (mknod("/dev/sda8", S_IFBLK | 0600, makedev(8, 8)) < 0 && errno != EEXIST) {
        log_message("metadata block node failed: %s", strerror(errno));
        return;
    }
    if (mount("/dev/sda8",
              "/metadata",
              "f2fs",
              MS_NOSUID | MS_NODEV | MS_NOATIME,
              NULL) < 0) {
        log_message("metadata mount failed: %s", strerror(errno));
        return;
    }
    mkdir_one("/metadata/saaios", 0755);
    copy_file("/run/boot.log", "/metadata/saaios/native-boot.log");
    metadata_ready = true;
    log_message("persistent metadata log enabled");
}

static void restore_saved_time(void) {
    int fd = open("/metadata/saaios/last-time", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return;
    }
    char value[32] = {0};
    ssize_t count = read(fd, value, sizeof(value) - 1);
    close(fd);
    long long saved = 0;
    struct timespec current = {0};
    if (count <= 0 || sscanf(value, "%lld", &saved) != 1 ||
        saved < 1700000000LL ||
        clock_gettime(CLOCK_REALTIME, &current) < 0 ||
        saved <= (long long)current.tv_sec) {
        return;
    }
    struct timespec restored = {
        .tv_sec = (time_t)saved,
        .tv_nsec = 0,
    };
    if (clock_settime(CLOCK_REALTIME, &restored) == 0) {
        log_message("saved UTC time restored");
    }
}

static void setup_data_storage(void) {
    mkdir_one("/data", 0755);
    if (create_partition_node("userdata", "/dev/saaios-data") < 0) {
        log_message("userdata partition was not found");
        return;
    }
    if (mount("/dev/saaios-data",
              "/data",
              "f2fs",
              MS_NOSUID | MS_NODEV | MS_NOATIME,
              "discard") < 0) {
        log_message("SaaiOS data mount failed: %s", strerror(errno));
        return;
    }
    mkdir_one("/data/saaios", 0755);
    mkdir_one("/data/saaios/apps", 0755);
    mkdir_one("/data/saaios/home", 0700);
    mkdir_one("/data/saaios/var", 0755);
    (void)setenv("HOME", "/data/saaios/home", 1);
    (void)setenv("SAAIOS_DATA", "/data/saaios", 1);
    log_message("SaaiOS data storage mounted");
}

static void prepare_persistent_firmware(void) {
    const char *source_directory = "/data/saaios/firmware";
    const char *target_directory = "/vendor/firmware";
    DIR *directory = opendir(source_directory);
    if (!directory) {
        log_message("persistent firmware directory unavailable: %s",
                    strerror(errno));
        return;
    }
    mkdir_one("/vendor", 0755);
    mkdir_one(target_directory, 0755);
    int linked = 0;
    struct dirent *entry;
    while ((entry = readdir(directory)) != NULL) {
        if (entry->d_name[0] == '.') {
            continue;
        }
        char source[384];
        char target[384];
        snprintf(source, sizeof(source), "%s/%s",
                 source_directory, entry->d_name);
        snprintf(target, sizeof(target), "%s/%s",
                 target_directory, entry->d_name);
        struct stat information;
        if (stat(source, &information) < 0 || !S_ISREG(information.st_mode)) {
            continue;
        }
        (void)unlink(target);
        if (symlink(source, target) == 0) {
            ++linked;
        }
    }
    closedir(directory);
    log_message("persistent firmware linked: %d files", linked);
}

static int setup_gadget(const char *udc) {
    const char *gadget = "/config/usb_gadget/saaios";
    mkdir_one("/config/usb_gadget", 0755);
    mkdir_one(gadget, 0755);
    mkdir_one("/config/usb_gadget/saaios/strings", 0755);
    mkdir_one("/config/usb_gadget/saaios/strings/0x409", 0755);
    mkdir_one("/config/usb_gadget/saaios/configs", 0755);
    mkdir_one("/config/usb_gadget/saaios/configs/c.1", 0755);
    mkdir_one("/config/usb_gadget/saaios/configs/c.1/strings", 0755);
    mkdir_one("/config/usb_gadget/saaios/configs/c.1/strings/0x409", 0755);
    mkdir_one("/config/usb_gadget/saaios/functions", 0755);

    if (write_value("/config/usb_gadget/saaios/idVendor", "0x1d6b") < 0 ||
        write_value("/config/usb_gadget/saaios/idProduct", "0x0104") < 0) {
        return -1;
    }
    (void)write_value("/config/usb_gadget/saaios/bcdUSB", "0x0200");
    (void)write_value("/config/usb_gadget/saaios/bcdDevice", "0x0100");
    (void)write_value("/config/usb_gadget/saaios/bDeviceClass", "0xEF");
    (void)write_value("/config/usb_gadget/saaios/bDeviceSubClass", "0x02");
    (void)write_value("/config/usb_gadget/saaios/bDeviceProtocol", "0x01");
    (void)write_value("/config/usb_gadget/saaios/strings/0x409/serialnumber", "SAAIOS-PANTHER-001");
    (void)write_value("/config/usb_gadget/saaios/strings/0x409/manufacturer", "SaaiOS");
    (void)write_value("/config/usb_gadget/saaios/strings/0x409/product", "SaaiOS native on Pixel 7");
    (void)write_value("/config/usb_gadget/saaios/configs/c.1/strings/0x409/configuration", "SaaiOS console and network");
    (void)write_value("/config/usb_gadget/saaios/configs/c.1/MaxPower", "250");

    mkdir_one("/config/usb_gadget/saaios/functions/acm.GS0", 0755);
    mkdir_one("/config/usb_gadget/saaios/functions/ecm.usb0", 0755);
    (void)symlink("/config/usb_gadget/saaios/functions/acm.GS0",
                  "/config/usb_gadget/saaios/configs/c.1/acm.GS0");
    (void)symlink("/config/usb_gadget/saaios/functions/ecm.usb0",
                  "/config/usb_gadget/saaios/configs/c.1/ecm.usb0");
    return write_value("/config/usb_gadget/saaios/UDC", udc);
}

static void run_child(const char *path, char *const argv[]) {
    pid_t child = fork();
    if (child == 0) {
        execv(path, argv);
        _exit(127);
    }
}

static int create_sound_nodes(void) {
    mkdir_one("/dev/snd", 0755);
    for (int attempt = 0; attempt < 200; ++attempt) {
        DIR *directory = opendir("/sys/class/sound");
        bool control_ready = false;
        if (directory) {
            struct dirent *entry;
            while ((entry = readdir(directory)) != NULL) {
                if (entry->d_name[0] == '.') {
                    continue;
                }
                char dev_path[320];
                snprintf(dev_path, sizeof(dev_path),
                         "/sys/class/sound/%s/dev", entry->d_name);
                int dev_fd = open(dev_path, O_RDONLY | O_CLOEXEC);
                if (dev_fd < 0) {
                    continue;
                }
                char value[64] = {0};
                ssize_t count = read(dev_fd, value, sizeof(value) - 1);
                close(dev_fd);
                unsigned int major_number = 0;
                unsigned int minor_number = 0;
                if (count <= 0 ||
                    sscanf(value, "%u:%u", &major_number, &minor_number) != 2) {
                    continue;
                }
                char node_path[256];
                snprintf(node_path, sizeof(node_path),
                         "/dev/snd/%s", entry->d_name);
                (void)unlink(node_path);
                if (mknod(node_path, S_IFCHR | 0660,
                          makedev(major_number, minor_number)) == 0 &&
                    strcmp(entry->d_name, "controlC0") == 0) {
                    control_ready = true;
                }
            }
            closedir(directory);
        }
        if (control_ready) {
            log_message("ALSA sound devices ready");
            return 0;
        }
        usleep(100000);
    }
    return -1;
}

static int run_audio_initialization(void) {
    pid_t child = fork();
    if (child == 0) {
        int output = open("/run/audio-init.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                          0644);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) {
                close(output);
            }
        }
        execl("/saaios/audio-init.sh", "audio-init.sh", NULL);
        _exit(127);
    }
    if (child < 0) {
        return -1;
    }
    int status = 0;
    if (waitpid(child, &status, 0) < 0) {
        return -1;
    }
    return WIFEXITED(status) && WEXITSTATUS(status) == 0 ? 0 : -1;
}

static void setup_audio(void) {
    prepare_persistent_firmware();
    for (size_t i = 0; i < ARRAY_SIZE(audio_modules); ++i) {
        if (strcmp(audio_modules[i], "aoc_core.ko") == 0) {
            (void)load_module_with_options(audio_modules[i],
                                           "aoc_autoload_firmware=Y");
        } else {
            (void)load_module(audio_modules[i]);
        }
    }
    if (create_sound_nodes() < 0) {
        log_message("ALSA sound card did not appear");
        return;
    }
    if (run_audio_initialization() < 0) {
        log_message("audio route initialization failed");
        return;
    }
    log_message("native audio route ready");
}

static void save_dmesg(const char *destination) {
    pid_t child = fork();
    if (child == 0) {
        int output = open(destination,
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                          0644);
        if (output < 0) {
            _exit(126);
        }
        (void)dup2(output, STDOUT_FILENO);
        (void)dup2(output, STDERR_FILENO);
        if (output > STDERR_FILENO) {
            close(output);
        }
        execl("/saaios/busybox", "busybox", "dmesg", NULL);
        _exit(127);
    }
    if (child > 0) {
        int status = 0;
        (void)waitpid(child, &status, 0);
        sync();
    }
}

static void configure_network(void) {
    char *const argv[] = {
        "busybox", "ifconfig", "usb0", "172.31.7.1",
        "netmask", "255.255.255.0", "up", NULL,
    };
    run_child("/saaios/busybox", argv);
}

static int bring_interface_up(const char *interface_name) {
    int fd = socket(AF_INET, SOCK_DGRAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        return -1;
    }
    struct ifreq request = {0};
    snprintf(request.ifr_name, sizeof(request.ifr_name), "%s", interface_name);
    if (ioctl(fd, SIOCGIFFLAGS, &request) < 0) {
        close(fd);
        return -1;
    }
    request.ifr_flags |= IFF_UP;
    int result = ioctl(fd, SIOCSIFFLAGS, &request);
    close(fd);
    return result;
}

static void start_wifi_scan(void) {
    pid_t child = fork();
    if (child == 0) {
        int output = open("/run/wifi-scan.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                          0644);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) {
                close(output);
            }
        }
        execl("/saaios/wifi-scan", "wifi-scan", "wlan0", NULL);
        _exit(127);
    }
}

static void start_wifi_supplicant(void) {
    mkdir_one("/run/wpa_supplicant", 0755);
    pid_t child = fork();
    if (child == 0) {
        sleep(10);
        const char *config_path =
            access("/metadata/saaios/wifi.conf", R_OK) == 0
                ? "/metadata/saaios/wifi.conf"
                : "/saaios/wpa_supplicant.conf";
        int output = open("/run/wpa_supplicant.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                          0644);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) {
                close(output);
            }
        }
        execl("/saaios/wpa_supplicant", "wpa_supplicant",
              "-Dnl80211", "-iwlan0", "-c", config_path,
              "-C/run/wpa_supplicant", NULL);
        _exit(127);
    }
}

static void start_wifi_action(void) {
    pid_t child = fork();
    if (child == 0) {
        sleep(12);
        execl("/saaios/wpa_cli", "wpa_cli",
              "-p", "/run/wpa_supplicant", "-i", "wlan0",
              "-a/saaios/wifi-action.sh", NULL);
        _exit(127);
    }
}

static void setup_wifi(void) {
    for (size_t i = 0; i < ARRAY_SIZE(wifi_modules); ++i) {
        if (strcmp(wifi_modules[i], "bcmdhd4389.ko") == 0) {
            (void)load_module_with_options(
                wifi_modules[i],
                "firmware_path=fw_bcmdhd.bin nvram_path=bcmdhd.cal "
                "clm_path=bcmdhd_clm.blob iface_name=wlan");
        } else {
            (void)load_module(wifi_modules[i]);
        }
    }
    if (create_misc_node("rfkill", "/dev/rfkill") < 0) {
        log_message("rfkill device node setup failed");
    }
    if (bring_interface_up("wlan0") < 0) {
        log_message("Wi-Fi interface setup failed: %s", strerror(errno));
        return;
    }
    log_message("Wi-Fi interface wlan0 is up");
    start_wifi_scan();
    start_wifi_supplicant();
    start_wifi_action();
}

static void setup_bluetooth(void) {
    for (size_t i = 0; i < ARRAY_SIZE(bluetooth_modules); ++i) {
        if (load_module(bluetooth_modules[i]) < 0) {
            log_message("Bluetooth module chain incomplete");
            return;
        }
    }

    bool nodes_ready = false;
    for (int attempt = 0; attempt < 50; ++attempt) {
        int gpio_status = create_char_node_from_sysfs(
            "/sys/bus/gpio/devices/gpiochip36/dev",
            "/dev/gpiochip36", 0600);
        int uart_status = create_char_node_from_sysfs(
            "/sys/class/tty/ttySAC18/dev",
            "/dev/ttySAC18", 0600);
        if (gpio_status == 0 && uart_status == 0) {
            nodes_ready = true;
            break;
        }
        usleep(100000);
    }
    if (!nodes_ready) {
        log_message("Bluetooth device nodes unavailable");
        return;
    }

    pid_t child = fork();
    if (child < 0) {
        log_message("Bluetooth loader fork failed: %s", strerror(errno));
        return;
    }
    if (child == 0) {
        int output = open("/run/bluetooth.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) {
                close(output);
            }
        }
        execl("/saaios/bt-init", "bt-init",
              "/saaios/firmware/BCM.hcd", "4000000", NULL);
        _exit(127);
    }
    log_message("Bluetooth firmware loader started");
}

static void start_runtime(void) {
    char *const argv[] = {
        "saaios-runtime", "--real-linux", "--tcp", "0.0.0.0:38127", NULL,
    };
    run_child("/saaios/saaios-runtime", argv);
}

static int create_drm_card_node(void) {
    char value[64] = {0};
    for (int attempt = 0; attempt < 50; ++attempt) {
        int fd = open("/sys/class/drm/card0/dev", O_RDONLY | O_CLOEXEC);
        if (fd >= 0) {
            ssize_t count = read(fd, value, sizeof(value) - 1);
            close(fd);
            if (count > 0) {
                break;
            }
        }
        usleep(100000);
    }
    unsigned int major_number = 0;
    unsigned int minor_number = 0;
    if (sscanf(value, "%u:%u", &major_number, &minor_number) != 2) {
        return -1;
    }
    mkdir_one("/dev/dri", 0755);
    unlink("/dev/dri/card0");
    return mknod("/dev/dri/card0", S_IFCHR | 0660,
                 makedev(major_number, minor_number));
}

static void start_display_splash(void) {
    if (create_drm_card_node() < 0) {
        log_message("DRM card did not appear");
        return;
    }
    pid_t child = fork();
    if (child == 0) {
        int output = open("/run/drm-splash.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                          0644);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) {
                close(output);
            }
        }
        execl("/saaios/drm-splash", "drm-splash", NULL);
        _exit(127);
    }
    if (child > 0) {
        log_message("native display splash started");
    }
}

static int create_input_node(const char *wanted_name,
                             const char *symlink_path) {
    for (int attempt = 0; attempt < 50; ++attempt) {
        DIR *directory = opendir("/sys/class/input");
        if (directory) {
            struct dirent *entry;
            while ((entry = readdir(directory)) != NULL) {
                if (strncmp(entry->d_name, "event", 5) != 0) {
                    continue;
                }
                char name_path[256];
                snprintf(name_path, sizeof(name_path),
                         "/sys/class/input/%s/device/name", entry->d_name);
                int name_fd = open(name_path, O_RDONLY | O_CLOEXEC);
                if (name_fd < 0) {
                    continue;
                }
                char input_name[64] = {0};
                ssize_t name_count = read(name_fd, input_name,
                                          sizeof(input_name) - 1);
                close(name_fd);
                if (name_count <= 0) {
                    continue;
                }
                input_name[strcspn(input_name, "\r\n")] = '\0';
                if (strcmp(input_name, wanted_name) != 0) {
                    continue;
                }
                char dev_path[256];
                snprintf(dev_path, sizeof(dev_path),
                         "/sys/class/input/%s/dev", entry->d_name);
                int dev_fd = open(dev_path, O_RDONLY | O_CLOEXEC);
                if (dev_fd < 0) {
                    continue;
                }
                char value[64] = {0};
                ssize_t count = read(dev_fd, value, sizeof(value) - 1);
                close(dev_fd);
                unsigned int major_number = 0;
                unsigned int minor_number = 0;
                if (count <= 0 ||
                    sscanf(value, "%u:%u", &major_number, &minor_number) != 2) {
                    continue;
                }
                mkdir_one("/dev/input", 0755);
                char node_path[128];
                snprintf(node_path, sizeof(node_path),
                         "/dev/input/%s", entry->d_name);
                unlink(node_path);
                int result = mknod(node_path, S_IFCHR | 0600,
                                   makedev(major_number, minor_number));
                closedir(directory);
                if (result == 0) {
                    unlink(symlink_path);
                    (void)symlink(node_path, symlink_path);
                    log_message("input %s ready at %s", wanted_name, node_path);
                }
                return result;
            }
            closedir(directory);
        }
        usleep(100000);
    }
    return -1;
}

static void start_touch_monitor(void) {
    pid_t child = fork();
    if (child == 0) {
        int output = open("/run/touch.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC,
                          0644);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) {
                close(output);
            }
        }
        execl("/saaios/touch-monitor", "touch-monitor", NULL);
        _exit(127);
    }
    if (child > 0) {
        log_message("native touch monitor started");
    }
}

static void configure_touch_controller(void) {
    const char *mode_path = "/proc/focaltech_touch/touch_mode";
    for (int attempt = 0; attempt < 50; ++attempt) {
        if (access(mode_path, W_OK) == 0) {
            break;
        }
        usleep(100000);
    }
    if (write_value(mode_path, "1") < 0) {
        log_message("touch controller mode setup failed");
        return;
    }
    (void)write_value("/proc/focaltech_touch/sense_onoff", "1");
    (void)write_value("/proc/focaltech_touch/irq_onoff", "1");
    log_message("touch controller set to normal active mode");
}

static int create_tty_node(void) {
    char value[64] = {0};
    int fd = open("/sys/class/tty/ttyGS0/dev", O_RDONLY | O_CLOEXEC);
    if (fd < 0) {
        return -1;
    }
    ssize_t count = read(fd, value, sizeof(value) - 1);
    close(fd);
    if (count <= 0) {
        return -1;
    }
    unsigned int major_number = 0;
    unsigned int minor_number = 0;
    if (sscanf(value, "%u:%u", &major_number, &minor_number) != 2) {
        return -1;
    }
    unlink("/dev/ttyGS0");
    return mknod("/dev/ttyGS0", S_IFCHR | 0600,
                 makedev(major_number, minor_number));
}

static pid_t start_console(void) {
    pid_t child = fork();
    if (child != 0) {
        return child;
    }
    (void)setsid();
    int fd = open("/dev/ttyGS0", O_RDWR);
    if (fd < 0) {
        _exit(126);
    }
    (void)ioctl(fd, TIOCSCTTY, 1);
    (void)dup2(fd, STDIN_FILENO);
    (void)dup2(fd, STDOUT_FILENO);
    (void)dup2(fd, STDERR_FILENO);
    if (fd > STDERR_FILENO) {
        close(fd);
    }
    setenv("PATH", "/saaios:/bin:/usr/bin", 1);
    dprintf(STDOUT_FILENO,
            "\r\nSaaiOS native console on Pixel 7\r\n"
            "Run: touch /run/saaios.keep\r\n\r\n");
    execl("/saaios/busybox", "sh", NULL);
    _exit(127);
}

int main(void) {
    prepare_mounts();
    log_message("native static PID 1 started");

    for (size_t i = 0; i < ARRAY_SIZE(restart_modules); ++i) {
        (void)load_module(restart_modules[i]);
    }
    start_safety_timer();

    for (size_t i = 0; i < ARRAY_SIZE(storage_modules); ++i) {
        if (strcmp(storage_modules[i], "ufs-pixel-fips140.ko") == 0) {
            (void)load_module_with_options(
                storage_modules[i],
                "fips_first_lba=86406 fips_last_lba=86917 "
                "fips_lu=0 use_hw_keys=true");
        } else {
            (void)load_module(storage_modules[i]);
        }
    }
    start_hardware_watchdog();
    setup_metadata_log();
    restore_saved_time();
    setup_data_storage();

    for (size_t i = 0; i < ARRAY_SIZE(usb_modules); ++i) {
        if (strcmp(usb_modules[i], "tcpci_max77759.ko") == 0) {
            (void)load_module_with_options(usb_modules[i], "conf_sbu=0");
        } else {
            (void)load_module(usb_modules[i]);
        }
    }

    for (size_t i = 0; i < ARRAY_SIZE(display_modules); ++i) {
        (void)load_module(display_modules[i]);
    }
    for (size_t i = 0; i < ARRAY_SIZE(touch_modules); ++i) {
        (void)load_module(touch_modules[i]);
    }
    setup_audio();
    configure_touch_controller();
    if (create_input_node("fts_ts", "/dev/input/touchscreen") < 0) {
        log_message("touch input did not appear");
    } else {
        start_touch_monitor();
    }
    if (create_input_node("gpio_keys", "/dev/input/volume-buttons") < 0) {
        log_message("volume buttons did not appear");
    }
    if (create_input_node("s2mpg12-power-keys", "/dev/input/power-button") < 0) {
        log_message("power button did not appear");
    }
    if (create_input_node("cs40l26_input", "/dev/input/haptic") < 0) {
        log_message("haptic input did not appear");
    } else if (write_value(
                   "/sys/bus/i2c/devices/8-0043/power/control", "on") < 0) {
        log_message("haptic runtime power lock failed: %s", strerror(errno));
    } else {
        log_message("haptic runtime power locked active");
        apply_haptic_factory_calibration();
    }
    start_display_splash();
    char *const brightness_argv[] = {
        "display-brightness.sh", "restore", NULL,
    };
    run_child("/saaios/display-brightness.sh", brightness_argv);
    setup_wifi();
    setup_bluetooth();

    char udc[128] = {0};
    for (int attempt = 0; attempt < 20; ++attempt) {
        if (find_udc(udc, sizeof(udc)) == 0) {
            break;
        }
        sleep(1);
    }
    if (udc[0] == '\0') {
        log_message("no USB device controller appeared");
        if (metadata_ready) {
            copy_file("/sys/kernel/debug/devices_deferred",
                      "/metadata/saaios/devices-deferred.txt");
            save_dmesg("/metadata/saaios/native-dmesg.log");
        }
        sleep(5);
        reboot_with_reason("saaios-no-udc");
        for (;;) pause();
    }

    log_message("USB device controller %s appeared", udc);
    if (setup_gadget(udc) < 0) {
        log_message("USB gadget setup failed");
        if (metadata_ready) {
            save_dmesg("/metadata/saaios/native-dmesg.log");
        }
        sleep(5);
        reboot_with_reason("saaios-gadget-fail");
        for (;;) pause();
    }

    sleep(2);
    (void)create_tty_node();
    configure_network();
    start_runtime();
    pid_t console_pid = start_console();
    mark_userspace_stable();
    log_message("native userspace ready");
    mark_current_slot_successful();

    for (;;) {
        int status = 0;
        pid_t ended = waitpid(-1, &status, 0);
        if (ended == console_pid) {
            usleep(250000);
            console_pid = start_console();
            log_message("USB console restarted");
        }
    }
}
