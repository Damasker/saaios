/* Freestanding PID 1: no libc. Android 4.19 + aarch64 syscalls only. */
#include <asm/unistd.h>
#include <linux/fb.h>
#include <linux/fcntl.h>
#include <linux/mman.h>
#include <linux/stat.h>
#include <linux/time.h>
#include <linux/input-event-codes.h>
#include "font8x8.h"

#ifndef AT_FDCWD
#define AT_FDCWD -100
#endif
#ifndef O_RDONLY
#define O_RDONLY 0
#endif
#ifndef O_WRONLY
#define O_WRONLY 1
#endif
#ifndef O_CREAT
#define O_CREAT 00000100
#endif
#ifndef O_TRUNC
#define O_TRUNC 00001000
#endif
#ifndef O_DIRECTORY
#define O_DIRECTORY 00040000
#endif
#ifndef O_RDWR
#define O_RDWR 2
#endif
#define SIGCHLD 17
#define WNOHANG 1
#define TIOCSCTTY 0x540E

static long sys(long n, long a, long b, long c, long d, long e, long f) {
    register long x8 asm("x8") = n;
    register long x0 asm("x0") = a;
    register long x1 asm("x1") = b;
    register long x2 asm("x2") = c;
    register long x3 asm("x3") = d;
    register long x4 asm("x4") = e;
    register long x5 asm("x5") = f;
    asm volatile("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5) : "memory");
    return x0;
}

static int is_err(long r) {
    return (unsigned long)r >= (unsigned long)-4095;
}

static unsigned slen(const char *s) {
    unsigned n = 0;
    while (s[n])
        n++;
    return n;
}

static void scopy(char *d, unsigned cap, const char *s) {
    unsigned i = 0;
    if (!cap)
        return;
    while (s[i] && i + 1 < cap) {
        d[i] = s[i];
        i++;
    }
    d[i] = 0;
}

static void append(char *d, unsigned cap, const char *s) {
    unsigned n = slen(d), i = 0;
    while (s[i] && n + 1 < cap)
        d[n++] = s[i++];
    d[n] = 0;
}

static void append_uint(char *d, unsigned cap, unsigned v) {
    char tmp[16];
    int i = 0;
    if (!v) {
        append(d, cap, "0");
        return;
    }
    while (v && i < 15) {
        tmp[i++] = (char)('0' + (v % 10));
        v /= 10;
    }
    while (i--) {
        char c[2] = {tmp[i], 0};
        append(d, cap, c);
    }
}

static unsigned makedev(unsigned maj, unsigned min) {
    return (min & 0xff) | ((maj & 0xfff) << 8) | ((min & ~0xffu) << 12);
}

static void sleep_ms(unsigned ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (long)(ms % 1000) * 1000000L;
    sys(__NR_nanosleep, (long)&ts, 0, 0, 0, 0, 0);
}

static void kmsg(const char *s) {
    long fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/kmsg", O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/console", O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        return;
    sys(__NR_write, fd, (long)s, slen(s), 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
}

static void write_file(const char *path, const char *s) {
    long fd = sys(__NR_openat, AT_FDCWD, (long)path, O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        fd = sys(__NR_openat, AT_FDCWD, (long)path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644, 0, 0);
    if (is_err(fd))
        return;
    sys(__NR_write, fd, (long)s, slen(s), 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
}

static int mkdir1(const char *path) {
    long r = sys(__NR_mkdirat, AT_FDCWD, (long)path, 0755, 0, 0, 0);
    return !is_err(r);
}

static int exists_path(const char *path) {
    long fd = sys(__NR_openat, AT_FDCWD, (long)path, O_RDONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        fd = sys(__NR_openat, AT_FDCWD, (long)path, O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        return 0;
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
    return 1;
}

static int read_file(const char *path, char *buf, unsigned cap) {
    long fd, n;
    if (!cap)
        return -1;
    fd = sys(__NR_openat, AT_FDCWD, (long)path, O_RDONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        return -1;
    n = sys(__NR_read, fd, (long)buf, cap - 1, 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
    if (is_err(n) || n < 0)
        return -1;
    buf[n] = 0;
    return (int)n;
}

/* 1 if node exists and first non-space char is '0'. Missing/unreadable → 0. */
static int sysfs_reads_zero(const char *path) {
    char buf[24];
    unsigned i = 0;
    if (read_file(path, buf, sizeof(buf)) <= 0)
        return 0;
    while (buf[i] == ' ' || buf[i] == '\t' || buf[i] == '\n')
        i++;
    return buf[i] == '0';
}

/*
 * Samsung sec_input: /sys/class/sec/tsp/input/enabled (and eventN/device/enabled)
 * store() calls the same resume as FB unblank. Writing 1 after a successful
 * unblank is another syna_tcm_resume. Only enable if the node still reads 0.
 * Never write cmd (probe_enable is a factory path). Once, not a loop.
 * Stock/v011: sec_touchscreen is event3. After HDL 0x45 (v019) it is event6.
 * These sysfs nodes are absent on this unit (tsp skip). Never unbind
 * synaptics_tcm_spi. Never pulse gpio_lcd_rst.
 */
static void tsp_enable_once(void) {
    static const char *nodes[] = {
        "/sys/class/sec/tsp/input/enabled",
        "/sys/class/sec/tsp/enabled",
        "/sys/class/input/event3/device/enabled",
        "/sys/devices/virtual/sec/tsp/input/enabled",
        0
    };
    char line[96];
    int i;
    for (i = 0; nodes[i]; i++) {
        if (!sysfs_reads_zero(nodes[i]))
            continue;
        write_file(nodes[i], "1\n");
        scopy(line, sizeof(line), "tsp enable ");
        append(line, sizeof(line), nodes[i]);
        kmsg(line);
        kmsg("\n");
        return;
    }
    kmsg("tsp skip (on or absent)\n");
}

static void mkdir_p(const char *path) {
    char buf[160];
    unsigned n = slen(path);
    if (n >= sizeof(buf))
        return;
    for (unsigned i = 0; i <= n; i++) {
        buf[i] = path[i];
        if (i && (buf[i] == '/' || buf[i] == 0)) {
            char c = buf[i];
            buf[i] = 0;
            sys(__NR_mkdirat, AT_FDCWD, (long)buf, 0755, 0, 0, 0);
            buf[i] = c;
        }
    }
}

static void pmsg(const char *s) {
    long fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/pmsg0", O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        return;
    sys(__NR_write, fd, (long)s, slen(s), 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
}

struct fbctx {
    long mem;
    long fd;
    unsigned long len;
    unsigned bpp, xres, yres, line;
    unsigned pix_bg, pix_fg;
    unsigned log_y;
    int ok;
    struct fb_var_screeninfo v;
};

static struct fbctx fb;

static unsigned pack_rgb(const struct fb_var_screeninfo *v, unsigned r, unsigned g, unsigned b) {
    unsigned sr = v->red.length ? r >> (8 - v->red.length) : 0;
    unsigned sg = v->green.length ? g >> (8 - v->green.length) : 0;
    unsigned sb = v->blue.length ? b >> (8 - v->blue.length) : 0;
    return (sr << v->red.offset) | (sg << v->green.offset) | (sb << v->blue.offset);
}

static void put_pixel(unsigned x, unsigned y, unsigned pix) {
    unsigned char *p;
    if (!fb.ok || x >= fb.xres || y >= fb.yres)
        return;
    p = (unsigned char *)fb.mem + (unsigned long)y * fb.line + (unsigned long)x * ((fb.bpp + 7) / 8);
    if (fb.bpp <= 16)
        *(unsigned short *)p = (unsigned short)pix;
    else
        *(unsigned *)p = pix;
}

#ifndef O_CLOEXEC
#define O_CLOEXEC 02000000
#endif
#ifndef O_NONBLOCK
#define O_NONBLOCK 04000
#endif
#ifndef O_WRONLY
#define O_WRONLY 1
#endif
#ifndef __NR_ppoll
#define __NR_ppoll 73
#endif
#ifndef POLLIN
#define POLLIN 0x0001
#endif
#define CLOCK_MONOTONIC 1
#define LINUX_REBOOT_MAGIC1 0xfee1dead
#define LINUX_REBOOT_MAGIC2 672274793
#define LINUX_REBOOT_CMD_RESTART 0x01234567
#ifndef __NR_clock_gettime
#define __NR_clock_gettime 113
#endif
#ifndef __NR_reboot
#define __NR_reboot 142
#endif
#ifndef __NR_sync
#define __NR_sync 81
#endif

struct input_event {
    long sec;
    long usec;
    unsigned short type;
    unsigned short code;
    int value;
};

struct pollfd {
    int fd;
    short events;
    short revents;
};

static void draw_char(unsigned x, unsigned y, char ch, unsigned fg, unsigned bg);

static void fill_rect(unsigned x0, unsigned y0, unsigned x1, unsigned y1, unsigned pix) {
    unsigned x, y;
    if (!fb.ok)
        return;
    if (x1 > fb.xres)
        x1 = fb.xres;
    if (y1 > fb.yres)
        y1 = fb.yres;
    for (y = y0; y < y1; y++)
        for (x = x0; x < x1; x++)
            put_pixel(x, y, pix);
}

static void draw_str(unsigned x, unsigned y, const char *s, unsigned fg, unsigned bg) {
    unsigned i;
    if (!fb.ok)
        return;
    for (i = 0; s[i]; i++) {
        draw_char(x, y, s[i], fg, bg);
        x += 16;
        if (x + 16 >= fb.xres)
            break;
    }
}

static unsigned menu_y0(void) {
    if (!fb.ok)
        return 0;
    if (fb.yres > 320)
        return fb.yres - 300;
    return fb.yres / 2;
}

static void fill_screen(unsigned pix) {
    unsigned y, x;
    if (!fb.ok)
        return;
    for (y = 0; y < fb.yres; y++)
        for (x = 0; x < fb.xres; x++)
            put_pixel(x, y, pix);
}

static void draw_char(unsigned x, unsigned y, char ch, unsigned fg, unsigned bg) {
    unsigned gx, gy, b;
    const unsigned char *g;
    if ((unsigned char)ch < 32 || (unsigned char)ch > 126)
        ch = '?';
    g = font8x8[(unsigned char)ch - 32];
    for (gy = 0; gy < 8; gy++) {
        b = g[gy];
        for (gx = 0; gx < 8; gx++) {
            unsigned pix = (b & (1u << gx)) ? fg : bg;
            put_pixel(x + gx * 2, y + gy * 2, pix);
            put_pixel(x + gx * 2 + 1, y + gy * 2, pix);
            put_pixel(x + gx * 2, y + gy * 2 + 1, pix);
            put_pixel(x + gx * 2 + 1, y + gy * 2 + 1, pix);
        }
    }
}

static void log_line(const char *s) {
    unsigned x = 8, i;
    kmsg(s);
    kmsg("\n");
    if (!fb.ok)
        return;
    if (fb.log_y + 20 >= menu_y0())
        fb.log_y = 8;
    for (i = 0; s[i]; i++) {
        draw_char(x, fb.log_y, s[i], fb.pix_fg, fb.pix_bg);
        x += 16;
        if (x + 16 >= fb.xres)
            break;
    }
    fb.log_y += 20;
}

static int setup_fb(void) {
    const char *paths[] = {"/dev/graphics/fb0", "/dev/fb0", 0};
    long fd = -1;
    struct fb_fix_screeninfo f;
    unsigned long pixels, len, mem;
    int i;

    for (i = 0; paths[i]; i++) {
        fd = sys(__NR_openat, AT_FDCWD, (long)paths[i], O_RDWR | O_CLOEXEC, 0, 0, 0);
        if (!is_err(fd))
            break;
        fd = -1;
    }
    if (fd < 0)
        return -1;
    if (is_err(sys(__NR_ioctl, fd, FBIOGET_VSCREENINFO, (long)&fb.v, 0, 0, 0)) ||
        is_err(sys(__NR_ioctl, fd, FBIOGET_FSCREENINFO, (long)&f, 0, 0, 0))) {
        sys(__NR_close, fd, 0, 0, 0, 0, 0);
        return -1;
    }
    fb.xres = fb.v.xres;
    fb.yres = fb.v.yres;
    fb.bpp = fb.v.bits_per_pixel;
    fb.line = f.line_length ? f.line_length : fb.xres * ((fb.bpp + 7) / 8);
    pixels = (unsigned long)fb.xres * fb.yres * ((fb.bpp + 7) / 8);
    len = f.smem_len;
    if (!len || len > 32ul * 1024 * 1024)
        len = pixels;
    mem = (unsigned long)sys(__NR_mmap, 0, (long)len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if (is_err((long)mem)) {
        sys(__NR_close, fd, 0, 0, 0, 0, 0);
        return -1;
    }
    fb.v.activate = FB_ACTIVATE_NOW | FB_ACTIVATE_FORCE;
    sys(__NR_ioctl, fd, FBIOPUT_VSCREENINFO, (long)&fb.v, 0, 0, 0);
    fb.mem = (long)mem;
    fb.len = len;
    fb.fd = fd;
    fb.ok = 1;
    fb.log_y = 8;
    fb.pix_bg = pack_rgb(&fb.v, 0x10, 0x18, 0x28);
    fb.pix_fg = pack_rgb(&fb.v, 0xE8, 0xF0, 0xFF);
    fill_screen(fb.pix_bg);
    /* Exactly one FB_EVENT_BLANK. ioctl FBIOBLANK and sysfs fb0/blank both
     * notify syna_tcm; v010 did both and the second resume never ended. */
    sys(__NR_ioctl, fd, FBIOBLANK, FB_BLANK_UNBLANK, 0, 0, 0);
    sys(__NR_ioctl, fd, FBIOPAN_DISPLAY, (long)&fb.v, 0, 0, 0);
    tsp_enable_once();
    return 0;
}

static int first_named(const char *dir, char *out, unsigned cap) {
    long fd = sys(__NR_openat, AT_FDCWD, (long)dir, O_RDONLY | O_DIRECTORY | O_CLOEXEC, 0, 0, 0);
    char buf[1024];
    long n;
    unsigned pos;
    if (is_err(fd))
        return -1;
    n = sys(__NR_getdents64, fd, (long)buf, sizeof(buf), 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
    if (is_err(n) || n <= 0)
        return -1;
    pos = 0;
    while (pos < (unsigned)n) {
        unsigned short reclen = *(unsigned short *)(buf + pos + 16);
        char *name = buf + pos + 19;
        if (name[0] != '.') {
            scopy(out, cap, name);
            return 0;
        }
        if (!reclen)
            break;
        pos += reclen;
    }
    return -1;
}

static void set_usb_device_role(void) {
    char name[64], path[160];
    long fd = sys(__NR_openat, AT_FDCWD, (long)"/sys/class/usb_role", O_RDONLY | O_DIRECTORY | O_CLOEXEC, 0, 0, 0);
    char buf[1024];
    long n;
    unsigned pos;
    if (is_err(fd))
        return;
    n = sys(__NR_getdents64, fd, (long)buf, sizeof(buf), 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
    if (is_err(n) || n <= 0)
        return;
    pos = 0;
    while (pos < (unsigned)n) {
        unsigned short reclen = *(unsigned short *)(buf + pos + 16);
        char *nm = buf + pos + 19;
        if (nm[0] != '.') {
            scopy(name, sizeof(name), nm);
            scopy(path, sizeof(path), "/sys/class/usb_role/");
            append(path, sizeof(path), name);
            append(path, sizeof(path), "/role");
            write_file(path, "device\n");
        }
        if (!reclen)
            break;
        pos += reclen;
    }
}

static void setup_android_usb(void) {
    /* Not Google 18D1 — Windows bound Nexus ADB INF to that. NetChip RNDIS. */
    write_file("/sys/class/android_usb/android0/enable", "0\n");
    write_file("/sys/class/android_usb/android0/idVendor", "0525\n");
    write_file("/sys/class/android_usb/android0/idProduct", "A4A2\n");
    write_file("/sys/class/android_usb/android0/functions", "rndis\n");
    write_file("/sys/class/android_usb/android0/enable", "1\n");
}

static void setup_configfs_gadget(void) {
    char udc[64], line[96];
    int have_rndis, have_acm, have_ecm;

    mkdir_p("/config");
    sys(__NR_mount, (long)"configfs", (long)"/config", (long)"configfs", 0, 0, 0);
    mkdir_p("/config/usb_gadget/g1/strings/0x409");
    mkdir_p("/config/usb_gadget/g1/functions");
    mkdir_p("/config/usb_gadget/g1/configs/c.1/strings/0x409");
    /* Linux gadget RNDIS: NetChip 0x0525:0xA4A2 — not a Google ADB PID. */
    write_file("/config/usb_gadget/g1/idVendor", "0x0525\n");
    write_file("/config/usb_gadget/g1/idProduct", "0xA4A2\n");
    write_file("/config/usb_gadget/g1/bcdDevice", "0x0100\n");
    write_file("/config/usb_gadget/g1/bcdUSB", "0x0200\n");
    write_file("/config/usb_gadget/g1/strings/0x409/serialnumber", "saaios\n");
    write_file("/config/usb_gadget/g1/strings/0x409/manufacturer", "SaaiOS\n");
    write_file("/config/usb_gadget/g1/strings/0x409/product", "RNDIS\n");
    write_file("/config/usb_gadget/g1/configs/c.1/strings/0x409/configuration", "RNDIS\n");
    write_file("/config/usb_gadget/g1/UDC", "\n");

    log_line("vid 0525 pid a4a2");

    have_rndis = mkdir1("/config/usb_gadget/g1/functions/rndis.usb0");
    have_acm = mkdir1("/config/usb_gadget/g1/functions/acm.gs0");
    have_ecm = mkdir1("/config/usb_gadget/g1/functions/ecm.usb0");

    scopy(line, sizeof(line), "fn rndis=");
    append(line, sizeof(line), have_rndis ? "y" : "n");
    append(line, sizeof(line), " acm=");
    append(line, sizeof(line), have_acm ? "y" : "n");
    append(line, sizeof(line), " ecm=");
    append(line, sizeof(line), have_ecm ? "y" : "n");
    log_line(line);

    if (have_rndis)
        sys(__NR_symlinkat, (long)"/config/usb_gadget/g1/functions/rndis.usb0", AT_FDCWD,
            (long)"/config/usb_gadget/g1/configs/c.1/rndis.usb0", 0, 0, 0);
    else if (have_ecm)
        sys(__NR_symlinkat, (long)"/config/usb_gadget/g1/functions/ecm.usb0", AT_FDCWD,
            (long)"/config/usb_gadget/g1/configs/c.1/ecm.usb0", 0, 0, 0);
    if (have_acm)
        sys(__NR_symlinkat, (long)"/config/usb_gadget/g1/functions/acm.gs0", AT_FDCWD,
            (long)"/config/usb_gadget/g1/configs/c.1/acm.gs0", 0, 0, 0);

    if (first_named("/sys/class/udc", udc, sizeof(udc)) == 0) {
        scopy(line, sizeof(line), "udc ");
        append(line, sizeof(line), udc);
        log_line(line);
        write_file("/config/usb_gadget/g1/UDC", udc);
    } else {
        log_line("udc none");
    }
}

static void pet_watchdog(void) {
    long fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/watchdog", O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/watchdog0", O_WRONLY | O_CLOEXEC, 0, 0, 0);
    if (is_err(fd))
        return;
    sys(__NR_write, fd, (long)"\0", 1, 0, 0, 0);
    sys(__NR_close, fd, 0, 0, 0, 0, 0);
}

static int shell_running;

static void run_shell(void) {
    long fd = -1;
    int i;
    for (i = 0; i < 40; i++) {
        fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/ttyGS0", O_RDWR, 0, 0, 0);
        if (!is_err(fd))
            break;
        fd = sys(__NR_openat, AT_FDCWD, (long)"/dev/ttyGS1", O_RDWR, 0, 0, 0);
        if (!is_err(fd))
            break;
        fd = -1;
        sleep_ms(200);
    }
    if (fd < 0)
        sys(__NR_exit_group, 1, 0, 0, 0, 0, 0);
    sys(__NR_setsid, 0, 0, 0, 0, 0, 0);
    sys(__NR_ioctl, fd, TIOCSCTTY, 1, 0, 0, 0);
    sys(__NR_dup3, fd, 0, 0, 0, 0, 0);
    sys(__NR_dup3, fd, 1, 0, 0, 0, 0);
    sys(__NR_dup3, fd, 2, 0, 0, 0, 0);
    if (fd > 2)
        sys(__NR_close, fd, 0, 0, 0, 0, 0);
    {
        char *argv[] = {"ash", "-i", 0};
        char *envp[] = {"PATH=/bin:/sbin", "HOME=/", "PS1=saaios# ", "TERM=linux", 0};
        sys(__NR_execve, (long)"/bin/busybox", (long)argv, (long)envp, 0, 0, 0);
    }
    sys(__NR_exit_group, 1, 0, 0, 0, 0, 0);
}

static void spawn_shell(void) {
    long pid;
    if (shell_running)
        return;
    if (!exists_path("/dev/ttyGS0") && !exists_path("/dev/ttyGS1"))
        return;
    pid = sys(__NR_clone, SIGCHLD, 0, 0, 0, 0, 0);
    if (is_err(pid))
        return;
    if (pid == 0)
        run_shell();
    shell_running = 1;
    log_line("ash on ttyGS");
}

static void spawn_rcs(void) {
    long pid = sys(__NR_clone, SIGCHLD, 0, 0, 0, 0, 0);
    char *argv[] = {"ash", "/etc/rcS", 0};
    char *envp[] = {"PATH=/bin:/sbin", "HOME=/", 0};
    if (is_err(pid)) {
        log_line("clone rcS fail");
        return;
    }
    if (pid == 0) {
        sys(__NR_execve, (long)"/bin/busybox", (long)argv, (long)envp, 0, 0, 0);
        sys(__NR_exit_group, 1, 0, 0, 0, 0, 0);
    }
}

static void refresh_net_status(void) {
    if (exists_path("/tmp/net-ok")) {
        /* rcS wrote a one-line status */
        long fd = sys(__NR_openat, AT_FDCWD, (long)"/tmp/net-ok", O_RDONLY | O_CLOEXEC, 0, 0, 0);
        char buf[80];
        long n;
        if (is_err(fd))
            return;
        n = sys(__NR_read, fd, (long)buf, sizeof(buf) - 1, 0, 0, 0);
        sys(__NR_close, fd, 0, 0, 0, 0, 0);
        if (n > 0) {
            unsigned i;
            buf[n] = 0;
            for (i = 0; buf[i]; i++)
                if (buf[i] == '\n')
                    buf[i] = 0;
            log_line(buf);
            sys(__NR_unlinkat, AT_FDCWD, (long)"/tmp/net-ok", 0, 0, 0, 0);
        }
    }
}

static const char *menu_tokens[] = { "live20", "run_app", "enable_report", "no_doze" };
static const char *menu_labels[] = {
    "LIVE20 0x20",
    "RUN_APP 0x14",
    "ENABLE_REPORT 0x05",
    "NO_DOZE 0x24",
};
#define MENU_N 4

static int menu_sel;
static int fd_vol = -1;
static int fd_pwr = -1;
static int power_down;
static unsigned long power_t0_ms;
static unsigned long last_vol_ms;
static char st_state[24];
static char st_response[8];
static int st_seq, st_retval, st_live20, st_mode, st_attn, st_irq, st_rx, st_rt;
static int st_action;
static int st_ok;

static unsigned long now_ms(void) {
    struct timespec ts;
    if (is_err(sys(__NR_clock_gettime, CLOCK_MONOTONIC, (long)&ts, 0, 0, 0, 0)))
        return 0;
    return (unsigned long)ts.tv_sec * 1000ul + (unsigned long)ts.tv_nsec / 1000000ul;
}

static int strncmp_local(const char *a, const char *b, unsigned n) {
    unsigned i;
    for (i = 0; i < n; i++) {
        if (a[i] != b[i])
            return (unsigned char)a[i] - (unsigned char)b[i];
        if (!a[i])
            return 0;
    }
    return 0;
}

static const char *find_key(const char *s, const char *key) {
    unsigned klen = slen(key), i;
    for (i = 0; s[i]; i++) {
        if (!strncmp_local(s + i, key, klen))
            return s + i + klen;
    }
    return 0;
}

static int parse_int(const char *s, int hex) {
    int sign = 1, n = 0;
    if (!s)
        return 0;
    if (*s == '-') {
        sign = -1;
        s++;
    }
    if (hex && s[0] == '0' && (s[1] == 'x' || s[1] == 'X'))
        s += 2;
    if (hex) {
        while ((*s >= '0' && *s <= '9') || (*s >= 'a' && *s <= 'f') ||
               (*s >= 'A' && *s <= 'F')) {
            int d = (*s >= 'a') ? *s - 'a' + 10 :
                    (*s >= 'A') ? *s - 'A' + 10 : *s - '0';
            n = (n << 4) + d;
            s++;
        }
        return sign * n;
    }
    while (*s >= '0' && *s <= '9') {
        n = n * 10 + (*s - '0');
        s++;
    }
    return sign * n;
}

static void copy_tok(char *d, unsigned cap, const char *s) {
    unsigned i = 0;
    if (!cap)
        return;
    while (s[i] && s[i] != ' ' && s[i] != '\n' && s[i] != '\t' && i + 1 < cap) {
        d[i] = s[i];
        i++;
    }
    d[i] = 0;
}

static int open_event_n(unsigned n) {
    char path[64];
    long fd;
    scopy(path, sizeof(path), "/dev/input/event");
    append_uint(path, sizeof(path), n);
    fd = sys(__NR_openat, AT_FDCWD, (long)path,
        O_RDONLY | O_NONBLOCK | O_CLOEXEC, 0, 0, 0);
    if (!is_err(fd))
        return (int)fd;
    return -1;
}

static int open_named_event(const char *want) {
    unsigned i;
    char path[80], name[64];
    for (i = 0; i < 12; i++) {
        scopy(path, sizeof(path), "/sys/class/input/event");
        append_uint(path, sizeof(path), i);
        append(path, sizeof(path), "/device/name");
        if (read_file(path, name, sizeof(name)) <= 0)
            continue;
        if (name[0] && slen(want) && !strncmp_local(name, want, slen(want)))
            return open_event_n(i);
    }
    return -1;
}

static const char *pick_touch_path(int want_status) {
    static const char *st;
    static const char *ac;
    const char *p;
    char line[96];

    if (want_status && st)
        return st;
    if (!want_status && ac)
        return ac;
    if (want_status) {
        if (exists_path("/sys/kernel/saaios_touch/status"))
            p = "/sys/kernel/saaios_touch/status";
        else if (exists_path("/sys/class/sec/tsp/saaios_touch/status"))
            p = "/sys/class/sec/tsp/saaios_touch/status";
        else
            p = "/sys/class/sec/tsp/saaios_status";
        st = p;
        scopy(line, sizeof(line), "status_path ");
    } else {
        if (exists_path("/sys/kernel/saaios_touch/action"))
            p = "/sys/kernel/saaios_touch/action";
        else if (exists_path("/sys/class/sec/tsp/saaios_touch/action"))
            p = "/sys/class/sec/tsp/saaios_touch/action";
        else
            p = "/sys/class/sec/tsp/saaios_action";
        ac = p;
        scopy(line, sizeof(line), "action_path ");
    }
    append(line, sizeof(line), p);
    log_line(line);
    return p;
}

static const char *status_path(void) {
    return pick_touch_path(1);
}

static const char *action_path(void) {
    return pick_touch_path(0);
}

static void refresh_status(void) {
    char buf[192];
    const char *v;
    if (read_file(status_path(), buf, sizeof(buf)) <= 0) {
        st_ok = 0;
        return;
    }
    st_ok = 1;
    v = find_key(buf, "seq=");
    st_seq = parse_int(v, 0);
    v = find_key(buf, "state=");
    if (v)
        copy_tok(st_state, sizeof(st_state), v);
    else
        scopy(st_state, sizeof(st_state), "?");
    v = find_key(buf, "action=0x");
    st_action = parse_int(v, 1);
    v = find_key(buf, "retval=");
    st_retval = parse_int(v, 0);
    v = find_key(buf, "response=");
    if (v)
        copy_tok(st_response, sizeof(st_response), v);
    else
        scopy(st_response, sizeof(st_response), "?");
    v = find_key(buf, "live20=");
    st_live20 = parse_int(v, 0);
    v = find_key(buf, "mode=");
    st_mode = parse_int(v, 1);
    v = find_key(buf, "attn=");
    st_attn = parse_int(v, 0);
    v = find_key(buf, "irq=");
    st_irq = parse_int(v, 0);
    v = find_key(buf, "rx=");
    st_rx = parse_int(v, 0);
    v = find_key(buf, "report_touch=");
    st_rt = parse_int(v, 0);
}

static const char *ui_state(void) {
    if (!st_ok)
        return "waiting tsp";
    if (!strncmp_local(st_state, "dead", 4))
        return "DEAD";
    if (!strncmp_local(st_state, "busy", 4))
        return "BUSY";
    if (st_retval == -62)
        return "TIMEOUT";
    if (st_seq && st_retval == 0)
        return "OK";
    return st_state;
}

static void draw_menu(void) {
    unsigned y, i;
    unsigned fg, bg, hi;
    char line[80];

    if (!fb.ok)
        return;
    fg = fb.pix_fg;
    bg = fb.pix_bg;
    hi = pack_rgb(&fb.v, 0x40, 0xC0, 0x80);
    y = menu_y0();
    fill_rect(0, y, fb.xres, fb.yres, bg);
    draw_str(8, y, "SaaiOS TOUCH LAB", fg, bg);
    y += 20;
    for (i = 0; i < MENU_N; i++) {
        scopy(line, sizeof(line), i == (unsigned)menu_sel ? "> " : "  ");
        append(line, sizeof(line), menu_labels[i]);
        draw_str(8, y, line, i == (unsigned)menu_sel ? hi : fg, bg);
        y += 20;
    }
    scopy(line, sizeof(line), "state: ");
    append(line, sizeof(line), ui_state());
    draw_str(8, y, line, fg, bg);
    y += 20;
    scopy(line, sizeof(line), "retval: ");
    if (st_retval < 0) {
        append(line, sizeof(line), "-");
        append_uint(line, sizeof(line), (unsigned)(-st_retval));
    } else {
        append_uint(line, sizeof(line), (unsigned)st_retval);
    }
    append(line, sizeof(line), "  response: ");
    append(line, sizeof(line), st_ok ? st_response : "?");
    draw_str(8, y, line, fg, bg);
    y += 20;
    scopy(line, sizeof(line), "live 0x20: ");
    if (st_live20 < 0) {
        append(line, sizeof(line), "-");
        append_uint(line, sizeof(line), (unsigned)(-st_live20));
    } else {
        append_uint(line, sizeof(line), (unsigned)st_live20);
    }
    draw_str(8, y, line, fg, bg);
    y += 20;
    scopy(line, sizeof(line), "mode: ");
    append_uint(line, sizeof(line), (unsigned)st_mode);
    append(line, sizeof(line), "  ATTN: ");
    if (st_attn < 0) {
        append(line, sizeof(line), "-");
        append_uint(line, sizeof(line), (unsigned)(-st_attn));
    } else {
        append_uint(line, sizeof(line), (unsigned)st_attn);
    }
    draw_str(8, y, line, fg, bg);
    y += 20;
    scopy(line, sizeof(line), "IRQ: ");
    append_uint(line, sizeof(line), (unsigned)st_irq);
    append(line, sizeof(line), "  RX: ");
    append_uint(line, sizeof(line), (unsigned)st_rx);
    append(line, sizeof(line), "  RT: ");
    append_uint(line, sizeof(line), (unsigned)st_rt);
    draw_str(8, y, line, fg, bg);
    y += 20;
    draw_str(8, y, "LONG POWER: REBOOT", fg, bg);
}

static void do_reboot(void) {
    log_line("reboot");
    sys(__NR_sync, 0, 0, 0, 0, 0, 0);
    sys(__NR_reboot, LINUX_REBOOT_MAGIC1, LINUX_REBOOT_MAGIC2,
        LINUX_REBOOT_CMD_RESTART, 0, 0, 0);
}

static void run_selected(void) {
    char line[80];
    scopy(line, sizeof(line), "run ");
    append(line, sizeof(line), menu_tokens[menu_sel]);
    log_line(line);
    write_file(action_path(), menu_tokens[menu_sel]);
    refresh_status();
    draw_menu();
}

static void handle_key(unsigned short code, int value) {
    unsigned long t = now_ms();
    if (value == 2)
        return;
    if (code == KEY_VOLUMEUP && value == 1) {
        if (last_vol_ms && t - last_vol_ms < 150)
            return;
        last_vol_ms = t;
        menu_sel++;
        if (menu_sel >= MENU_N)
            menu_sel = 0;
        draw_menu();
    } else if (code == KEY_VOLUMEDOWN && value == 1) {
        if (last_vol_ms && t - last_vol_ms < 150)
            return;
        last_vol_ms = t;
        menu_sel--;
        if (menu_sel < 0)
            menu_sel = MENU_N - 1;
        draw_menu();
    } else if (code == KEY_POWER) {
        if (value == 1) {
            power_down = 1;
            power_t0_ms = t;
        } else if (value == 0 && power_down) {
            unsigned long dt = t - power_t0_ms;
            power_down = 0;
            if (dt < 2000)
                run_selected();
        }
    }
}

static void drain_evdev(int fd) {
    struct input_event ev;
    long n;
    if (fd < 0)
        return;
    for (;;) {
        n = sys(__NR_read, fd, (long)&ev, sizeof(ev), 0, 0, 0);
        if (is_err(n) || n != (long)sizeof(ev))
            break;
        if (ev.type != EV_KEY)
            continue;
        handle_key(ev.code, ev.value);
    }
}

static void menu_poll_keys(void) {
    struct pollfd pfd[2];
    struct timespec ts;
    int n = 0;
    if (fd_vol >= 0) {
        pfd[n].fd = fd_vol;
        pfd[n].events = POLLIN;
        pfd[n].revents = 0;
        n++;
    }
    if (fd_pwr >= 0) {
        pfd[n].fd = fd_pwr;
        pfd[n].events = POLLIN;
        pfd[n].revents = 0;
        n++;
    }
    ts.tv_sec = 0;
    ts.tv_nsec = 150000000L;
    if (n)
        sys(__NR_ppoll, (long)pfd, n, (long)&ts, 0, 0, 0);
    else
        sleep_ms(80);
    drain_evdev(fd_vol);
    drain_evdev(fd_pwr);
    if (power_down && now_ms() - power_t0_ms >= 2000)
        do_reboot();
}

static void menu_open_keys(void) {
    if (fd_vol < 0) {
        fd_vol = open_named_event("gpio_keys");
        if (fd_vol < 0)
            fd_vol = open_event_n(1);
    }
    if (fd_pwr < 0) {
        fd_pwr = open_named_event("sec-pmic-key");
        if (fd_pwr < 0)
            fd_pwr = open_event_n(2);
    }
}

void _start(void) {
    char line[96];

    sys(__NR_mkdirat, AT_FDCWD, (long)"/dev", 0755, 0, 0, 0);
    sys(__NR_mkdirat, AT_FDCWD, (long)"/proc", 0755, 0, 0, 0);
    sys(__NR_mkdirat, AT_FDCWD, (long)"/sys", 0755, 0, 0, 0);
    sys(__NR_mount, (long)"proc", (long)"/proc", (long)"proc", 0, 0, 0);
    sys(__NR_mount, (long)"sysfs", (long)"/sys", (long)"sysfs", 0, 0, 0);
    sys(__NR_mount, (long)"devtmpfs", (long)"/dev", (long)"devtmpfs", 0, 0, 0);

    sys(__NR_mkdirat, AT_FDCWD, (long)"/dev/graphics", 0755, 0, 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/kmsg", S_IFCHR | 0600, makedev(1, 11), 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/console", S_IFCHR | 0600, makedev(5, 1), 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/fb0", S_IFCHR | 0600, makedev(29, 0), 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/graphics/fb0", S_IFCHR | 0600, makedev(29, 0), 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/pmsg0", S_IFCHR | 0600, makedev(2, 32), 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/watchdog", S_IFCHR | 0600, makedev(10, 130), 0, 0);
    sys(__NR_mkdirat, AT_FDCWD, (long)"/dev/pts", 0755, 0, 0, 0);
    sys(__NR_mknodat, AT_FDCWD, (long)"/dev/ptmx", S_IFCHR | 0666, makedev(5, 2), 0, 0);
    sys(__NR_mount, (long)"devpts", (long)"/dev/pts", (long)"devpts", 0, 0, 0);
    sys(__NR_mkdirat, AT_FDCWD, (long)"/tmp", 01777, 0, 0, 0);
    sys(__NR_mkdirat, AT_FDCWD, (long)"/run", 0755, 0, 0, 0);
    sys(__NR_mkdirat, AT_FDCWD, (long)"/bin", 0755, 0, 0, 0);

    kmsg("SaaiOS booting...\n");
    pmsg("SaaiOS booting\n");
    write_file("/proc/sys/kernel/hung_task_timeout_secs", "0\n");
    write_file("/proc/sys/kernel/panic", "0\n");

    if (setup_fb() == 0) {
        log_line("SaaiOS v019 since76");
        scopy(line, sizeof(line), "fb ");
        append_uint(line, sizeof(line), fb.xres);
        append(line, sizeof(line), "x");
        append_uint(line, sizeof(line), fb.yres);
        append(line, sizeof(line), " bpp");
        append_uint(line, sizeof(line), fb.bpp);
        append(line, sizeof(line), " R");
        append_uint(line, sizeof(line), fb.v.red.offset);
        append(line, sizeof(line), " G");
        append_uint(line, sizeof(line), fb.v.green.offset);
        append(line, sizeof(line), " B");
        append_uint(line, sizeof(line), fb.v.blue.offset);
        log_line(line);
    } else {
        kmsg("SaaiOS: no framebuffer\n");
    }

    set_usb_device_role();
    setup_android_usb();
    setup_configfs_gadget();
    spawn_rcs();
    spawn_shell();
    log_line("telnet :23 :2323");
    log_line("ssh :22");
    log_line("Vol+/- menu  Power run  2s reboot");
    draw_menu();

    for (;;) {
        pet_watchdog();
        {
            long w = sys(__NR_wait4, -1, 0, WNOHANG, 0, 0, 0);
            if (w > 0)
                shell_running = 0;
        }
        spawn_shell();
        refresh_net_status();
        menu_open_keys();
        menu_poll_keys();
        {
            static unsigned tick;
            tick++;
            if ((tick & 3) == 0) {
                refresh_status();
                draw_menu();
            }
        }
    }
}
