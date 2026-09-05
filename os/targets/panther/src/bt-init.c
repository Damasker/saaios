#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/gpio.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <termios.h>
#include <unistd.h>

#ifndef N_HCI
#define N_HCI 15
#endif
#define HCIUARTSETPROTO _IOW('U', 200, int)
#define HCI_UART_H4 0
#ifndef AF_BLUETOOTH
#define AF_BLUETOOTH 31
#endif
#define BTPROTO_HCI 1
#define HCIDEVUP _IOW('H', 201, int)

static volatile sig_atomic_t stopping;
static int gpio_lines = -1;

static void stop_handler(int signal_number)
{
    (void)signal_number;
    stopping = 1;
}

static int gpio_set(uint64_t bits)
{
    struct gpio_v2_line_values values = {
        .bits = bits,
        .mask = 0x3,
    };
    return ioctl(gpio_lines, GPIO_V2_LINE_SET_VALUES_IOCTL, &values);
}

static int bluetooth_power_on(void)
{
    int chip = open("/dev/gpiochip36", O_RDONLY | O_CLOEXEC);
    if (chip < 0) {
        perror("bluetooth: open gpiochip36");
        return -1;
    }

    struct gpio_v2_line_request request = {0};
    request.offsets[0] = 2; /* BT_REG_ON */
    request.offsets[1] = 3; /* BT_DEV_WAKE */
    request.num_lines = 2;
    request.config.flags = GPIO_V2_LINE_FLAG_OUTPUT;
    request.config.num_attrs = 1;
    request.config.attrs[0].attr.id = GPIO_V2_LINE_ATTR_ID_OUTPUT_VALUES;
    request.config.attrs[0].attr.values = 0;
    request.config.attrs[0].mask = 0x3;
    snprintf(request.consumer, sizeof(request.consumer), "saaios-bluetooth");

    if (ioctl(chip, GPIO_V2_GET_LINE_IOCTL, &request) < 0) {
        perror("bluetooth: request power GPIOs");
        close(chip);
        return -1;
    }
    close(chip);
    gpio_lines = request.fd;

    /* Match Google's nitrous rfkill power-on sequence exactly: keep both
     * outputs low, raise REG_ON after 30 ms, then assert DEV_WAKE. */
    if (gpio_set(0) < 0) {
        perror("bluetooth: power off before reset");
        return -1;
    }
    usleep(30000);
    if (gpio_set(0x1) < 0 || gpio_set(0x3) < 0) {
        perror("bluetooth: power on");
        return -1;
    }
    usleep(15000);
    return 0;
}

static int uart_open(void)
{
    int fd = open("/dev/ttySAC18", O_RDWR | O_NOCTTY | O_CLOEXEC);
    if (fd < 0) {
        perror("bluetooth: open ttySAC18");
        return -1;
    }

    struct termios tio;
    if (tcgetattr(fd, &tio) < 0) {
        perror("bluetooth: tcgetattr");
        close(fd);
        return -1;
    }
    cfmakeraw(&tio);
    tio.c_cflag |= CLOCAL | CREAD | CRTSCTS;
    tio.c_cflag &= ~(CSTOPB | PARENB);
    tio.c_cflag = (tio.c_cflag & ~CSIZE) | CS8;
    cfsetispeed(&tio, B115200);
    cfsetospeed(&tio, B115200);
    if (tcsetattr(fd, TCSANOW, &tio) < 0) {
        perror("bluetooth: tcsetattr");
        close(fd);
        return -1;
    }
    tcflush(fd, TCIOFLUSH);
    return fd;
}

static int uart_set_speed(int fd, speed_t speed)
{
    struct termios tio;
    if (tcgetattr(fd, &tio) < 0) return -1;
    cfsetispeed(&tio, speed);
    cfsetospeed(&tio, speed);
    return tcsetattr(fd, TCSANOW, &tio);
}

/* Linux termios2 permits the 6 and 9.6 Mbit/s rates used by Google's
 * Broadcom service but not represented by portable termios speed_t values. */
struct saai_termios2 {
    unsigned int c_iflag;
    unsigned int c_oflag;
    unsigned int c_cflag;
    unsigned int c_lflag;
    unsigned char c_line;
    unsigned char c_cc[19];
    unsigned int c_ispeed;
    unsigned int c_ospeed;
};

#define SAAI_TCGETS2 _IOR('T', 0x2a, struct saai_termios2)
#define SAAI_TCSETS2 _IOW('T', 0x2b, struct saai_termios2)
#define SAAI_CBAUD   0010017u
#define SAAI_BOTHER  0010000u
#define SAAI_IBSHIFT 16

static int uart_set_custom_speed(int fd, unsigned int baud)
{
    struct saai_termios2 tio;
    if (ioctl(fd, SAAI_TCGETS2, &tio) < 0) return -1;
    tio.c_cflag &= ~(SAAI_CBAUD | (SAAI_CBAUD << SAAI_IBSHIFT));
    tio.c_cflag |= SAAI_BOTHER | (SAAI_BOTHER << SAAI_IBSHIFT);
    tio.c_ispeed = baud;
    tio.c_ospeed = baud;
    return ioctl(fd, SAAI_TCSETS2, &tio);
}

static int read_exact_with_timeout(int fd, uint8_t *buffer, size_t length, int timeout_ms)
{
    size_t used = 0;
    while (used < length) {
        struct pollfd pfd = {.fd = fd, .events = POLLIN};
        int ready = poll(&pfd, 1, timeout_ms);
        if (ready <= 0) return -1;
        ssize_t count = read(fd, buffer + used, length - used);
        if (count <= 0) return -1;
        used += (size_t)count;
    }
    return 0;
}

static int read_event(int fd, uint8_t *event, size_t capacity, int timeout_ms)
{
    if (read_exact_with_timeout(fd, event, 3, timeout_ms) < 0) return -1;
    if (event[0] != 0x04 || (size_t)event[2] + 3 > capacity) return -1;
    if (read_exact_with_timeout(fd, event + 3, event[2], timeout_ms) < 0) return -1;
    return (int)event[2] + 3;
}

static int send_command(int fd, uint16_t opcode, const uint8_t *payload, uint8_t length,
                        int allow_restart)
{
    uint8_t packet[4 + 255];
    uint8_t event[260];
    packet[0] = 0x01;
    packet[1] = opcode & 0xff;
    packet[2] = opcode >> 8;
    packet[3] = length;
    if (length) memcpy(packet + 4, payload, length);

    size_t total = (size_t)length + 4;
    if (write(fd, packet, total) != (ssize_t)total || tcdrain(fd) < 0) return -1;

    for (int attempts = 0; attempts < 16; ++attempts) {
        /* Launch-RAM boots the complete controller firmware and is much slower
         * than an ordinary HCI command on BCM4389. */
        int count = read_event(fd, event, sizeof(event), allow_restart ? 7000 : 2000);
        if (count < 0) {
            if (!allow_restart) fprintf(stderr, "bluetooth: opcode=%04x event timeout\n", opcode);
            return allow_restart ? 0 : -1;
        }
        if (event[1] == 0x0e && count >= 7 &&
            event[4] == (opcode & 0xff) && event[5] == (opcode >> 8)) {
            if (event[6] != 0)
                fprintf(stderr, "bluetooth: opcode=%04x command-complete status=%02x\n",
                        opcode, event[6]);
            return event[6] == 0 ? 0 : -1;
        }
        if (event[1] == 0x0f && count >= 7 &&
            event[5] == (opcode & 0xff) && event[6] == (opcode >> 8)) {
            if (event[3] != 0)
                fprintf(stderr, "bluetooth: opcode=%04x command-status=%02x\n",
                        opcode, event[3]);
            return event[3] == 0 ? 0 : -1;
        }
        fprintf(stderr, "bluetooth: opcode=%04x skipped event=%02x length=%d\n",
                opcode, event[1], count);
    }
    return -1;
}

static int write_all(int fd, const uint8_t *data, size_t length)
{
    while (length) {
        ssize_t count = write(fd, data, length);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return -1;
        data += count;
        length -= (size_t)count;
    }
    return 0;
}

static int flush_firmware_batch(int fd, uint8_t *batch, size_t *used)
{
    if (!*used) return 0;
    if (write_all(fd, batch, *used) < 0 || tcdrain(fd) < 0) return -1;
    *used = 0;
    return 0;
}

static int load_hcd(int uart, const char *path, int accelerated)
{
    FILE *firmware = fopen(path, "rb");
    if (!firmware) {
        perror("bluetooth: open firmware");
        return -1;
    }

    uint8_t payload[255];
    uint8_t batch[32768];
    size_t batch_used = 0;
    unsigned records = 0;
    int result = -1;
    for (;;) {
        uint8_t header[3];
        size_t got = fread(header, 1, sizeof(header), firmware);
        if (got == 0 && feof(firmware)) {
            result = 0;
            break;
        }
        if (got != sizeof(header)) break;
        uint16_t opcode = (uint16_t)(header[0] | header[1] << 8);
        uint8_t length = header[2];
        if (fread(payload, 1, length, firmware) != length) break;
        int launches_ram = opcode == 0xfc4e;
        int status;
        if (opcode == 0xfc4c && accelerated) {
            /* Match Google's accelerated loader: concatenate complete H4 command
             * packets and write about 32 KiB at a time.  The minidriver does not
             * return command-complete events for these write-RAM records. */
            size_t packet_size = (size_t)length + 4;
            if (batch_used + packet_size > sizeof(batch) &&
                flush_firmware_batch(uart, batch, &batch_used) < 0) {
                status = -1;
            } else {
                batch[batch_used++] = 0x01;
                batch[batch_used++] = header[0];
                batch[batch_used++] = header[1];
                batch[batch_used++] = length;
                if (length) {
                    memcpy(batch + batch_used, payload, length);
                    batch_used += length;
                }
                status = batch_used >= 0x7efc
                    ? flush_firmware_batch(uart, batch, &batch_used) : 0;
            }
        } else {
            status = flush_firmware_batch(uart, batch, &batch_used);
            if (status == 0)
                status = send_command(uart, opcode, payload, length, launches_ram);
        }
        if (status < 0) {
            fprintf(stderr, "bluetooth: firmware record %u opcode=%04x failed\n",
                    records, opcode);
            break;
        }
        ++records;
        if ((records & 511u) == 0) printf("bluetooth: firmware %u records\n", records);
        if (launches_ram) usleep(500000);
    }
    if (result == 0 && flush_firmware_batch(uart, batch, &batch_used) < 0)
        result = -1;
    fclose(firmware);
    if (result == 0) printf("bluetooth: firmware loaded records=%u\n", records);
    return result;
}

static int parse_address(uint8_t address[6])
{
    int fd = open("/sys/firmware/devicetree/base/chosen/config/bt_addr", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    uint8_t raw[32];
    ssize_t count = read(fd, raw, sizeof(raw));
    close(fd);
    if (count == 6) {
        for (int i = 0; i < 6; ++i) address[i] = raw[5 - i];
        return 0;
    }
    if (count >= 17) {
        unsigned parsed[6];
        if (sscanf((char *)raw, "%02x:%02x:%02x:%02x:%02x:%02x",
                   &parsed[0], &parsed[1], &parsed[2], &parsed[3], &parsed[4], &parsed[5]) == 6) {
            for (int i = 0; i < 6; ++i) address[i] = (uint8_t)parsed[5 - i];
            return 0;
        }
    }
    return -1;
}

static int bring_hci0_up(void)
{
    for (int attempt = 0; attempt < 30; ++attempt) {
        int control = socket(AF_BLUETOOTH,
                             SOCK_RAW | SOCK_CLOEXEC, BTPROTO_HCI);
        if (control >= 0) {
            int result = ioctl(control, HCIDEVUP, 0);
            int saved_errno = errno;
            close(control);
            if (result == 0 || saved_errno == EALREADY) return 0;
            errno = saved_errno;
        }
        usleep(100000);
    }
    return -1;
}

int main(int argc, char **argv)
{
    const char *firmware_path = argc > 1 ? argv[1] : "/saaios/firmware/BCM.hcd";
    unsigned int runtime_baud = argc > 2 ? (unsigned int)strtoul(argv[2], NULL, 10) : 4000000u;
    int accelerated = argc <= 3 || strcmp(argv[3], "slow") != 0;
    signal(SIGTERM, stop_handler);
    signal(SIGINT, stop_handler);

    if (bluetooth_power_on() < 0) goto fail;
    int uart = uart_open();
    if (uart < 0) goto fail;

    if (send_command(uart, 0x0c03, NULL, 0, 0) < 0) {
        fprintf(stderr, "bluetooth: controller reset failed\n");
        close(uart);
        goto fail;
    }
    const uint8_t baud_4m[6] = {0, 0, 0x00, 0x09, 0x3d, 0x00};
    if (send_command(uart, 0xfc18, baud_4m, sizeof(baud_4m), 0) < 0 ||
        uart_set_speed(uart, B4000000) < 0) {
        fprintf(stderr, "bluetooth: switch to firmware baud failed\n");
        close(uart);
        goto fail;
    }
    usleep(20000);
    if (send_command(uart, 0x0c03, NULL, 0, 0) < 0 ||
        (accelerated && send_command(uart, 0xfc79, NULL, 0, 0) < 0)) {
        fprintf(stderr, "bluetooth: accelerated download setup failed\n");
        close(uart);
        goto fail;
    }
    if (send_command(uart, 0xfc2e, NULL, 0, 0) < 0) {
        fprintf(stderr, "bluetooth: minidriver command failed\n");
        close(uart);
        goto fail;
    }
    usleep(50000);
    if (load_hcd(uart, firmware_path, accelerated) < 0) {
        close(uart);
        goto fail;
    }

    /* Launch-RAM starts the production firmware at 115200. Google's loader
     * changes only the host UART here, waits 250 ms, resets HCI, and then
     * asks the running firmware to move to the final operating rate. */
    if (uart_set_speed(uart, B115200) < 0) {
        perror("bluetooth: restore launch baud");
        close(uart);
        goto fail;
    }
    tcflush(uart, TCIOFLUSH);
    usleep(250000);
    if (send_command(uart, 0x0c03, NULL, 0, 0) < 0) {
        fprintf(stderr, "bluetooth: post-firmware reset failed\n");
        close(uart);
        goto fail;
    }

    if (runtime_baud != 115200u) {
        uint8_t runtime_speed[6] = {
            0, 0,
            (uint8_t)(runtime_baud & 0xff),
            (uint8_t)((runtime_baud >> 8) & 0xff),
            (uint8_t)((runtime_baud >> 16) & 0xff),
            (uint8_t)((runtime_baud >> 24) & 0xff),
        };
        if (send_command(uart, 0xfc18, runtime_speed, sizeof(runtime_speed), 0) < 0) {
            fprintf(stderr, "bluetooth: runtime baud command failed\n");
            close(uart);
            goto fail;
        }
        int speed_status = runtime_baud == 4000000u
            ? uart_set_speed(uart, B4000000)
            : uart_set_custom_speed(uart, runtime_baud);
        if (speed_status < 0) {
            perror("bluetooth: set runtime baud");
            close(uart);
            goto fail;
        }
        tcflush(uart, TCIOFLUSH);
        usleep(20000);
        if (send_command(uart, 0x0c03, NULL, 0, 0) < 0) {
            fprintf(stderr, "bluetooth: reset at runtime baud failed\n");
            close(uart);
            goto fail;
        }
    }
    printf("bluetooth: firmware runtime baud=%u\n", runtime_baud);
    uint8_t address[6];
    if (parse_address(address) == 0 && send_command(uart, 0xfc01, address, 6, 0) == 0)
        printf("bluetooth: device address applied\n");

    int discipline = N_HCI;
    if (ioctl(uart, TIOCSETD, &discipline) < 0) {
        perror("bluetooth: set HCI line discipline");
        close(uart);
        goto fail;
    }
    if (ioctl(uart, HCIUARTSETPROTO, HCI_UART_H4) < 0) {
        perror("bluetooth: set H4 protocol");
        close(uart);
        goto fail;
    }
    printf("bluetooth: hci0 attached\n");
    if (bring_hci0_up() < 0) {
        perror("bluetooth: hci0 up");
        close(uart);
        goto fail;
    }
    printf("bluetooth: hci0 powered\n");
    fflush(stdout);

    pid_t key_loader = fork();
    if (key_loader == 0) {
        int output = open("/run/bluetooth-keys.log",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
        if (output >= 0) {
            (void)dup2(output, STDOUT_FILENO);
            (void)dup2(output, STDERR_FILENO);
            if (output > STDERR_FILENO) close(output);
        }
        execl("/saaios/bt-pair", "bt-pair", "--restore", NULL);
        _exit(127);
    }
    if (key_loader > 0) {
        printf("bluetooth: persistent key loader started\n");
        fflush(stdout);
    }

    while (!stopping) pause();
    close(uart);
fail:
    if (gpio_lines >= 0) {
        gpio_set(0);
        close(gpio_lines);
    }
    return stopping ? 0 : 1;
}
