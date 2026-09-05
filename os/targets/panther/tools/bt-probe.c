#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <linux/gpio.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>

#define ARRAY_SIZE(a) (sizeof(a) / sizeof((a)[0]))

static int gpio_set(int fd, uint64_t bits)
{
    struct gpio_v2_line_values values = {
        .bits = bits,
        .mask = 0x3,
    };
    return ioctl(fd, GPIO_V2_LINE_SET_VALUES_IOCTL, &values);
}

static int gpio_power_open(void)
{
    int chip = open("/dev/gpiochip36", O_RDONLY | O_CLOEXEC);
    if (chip < 0) {
        perror("open gpiochip36");
        return -1;
    }

    struct gpio_v2_line_request request = {0};
    request.offsets[0] = 2; /* BT_REG_ON / shutdown-gpios */
    request.offsets[1] = 3; /* BT_DEV_WAKE */
    request.num_lines = 2;
    request.config.flags = GPIO_V2_LINE_FLAG_OUTPUT;
    request.config.num_attrs = 1;
    request.config.attrs[0].attr.id = GPIO_V2_LINE_ATTR_ID_OUTPUT_VALUES;
    request.config.attrs[0].attr.values = 0;
    request.config.attrs[0].mask = 0x3;
    snprintf(request.consumer, sizeof(request.consumer), "saaios-bluetooth");

    if (ioctl(chip, GPIO_V2_GET_LINE_IOCTL, &request) < 0) {
        perror("request Bluetooth GPIOs");
        close(chip);
        return -1;
    }
    close(chip);

    gpio_set(request.fd, 0);
    usleep(50000);
    if (gpio_set(request.fd, 0x3) < 0) {
        perror("power on Bluetooth GPIOs");
        close(request.fd);
        return -1;
    }
    usleep(100000);
    return request.fd;
}

static int uart_open(speed_t speed)
{
    int fd = open("/dev/ttySAC18", O_RDWR | O_NOCTTY | O_CLOEXEC);
    if (fd < 0) {
        perror("open ttySAC18");
        return -1;
    }

    struct termios tio;
    if (tcgetattr(fd, &tio) < 0) {
        perror("tcgetattr");
        close(fd);
        return -1;
    }
    cfmakeraw(&tio);
    tio.c_cflag |= CLOCAL | CREAD | CRTSCTS;
    tio.c_cflag &= ~CSTOPB;
    tio.c_cflag &= ~PARENB;
    tio.c_cflag = (tio.c_cflag & ~CSIZE) | CS8;
    cfsetispeed(&tio, speed);
    cfsetospeed(&tio, speed);
    if (tcsetattr(fd, TCSANOW, &tio) < 0) {
        perror("tcsetattr");
        close(fd);
        return -1;
    }
    tcflush(fd, TCIOFLUSH);
    return fd;
}

static int read_event(int fd, uint8_t *buffer, size_t capacity, int timeout_ms)
{
    size_t used = 0;
    while (used < 3) {
        struct pollfd pfd = {.fd = fd, .events = POLLIN};
        int ready = poll(&pfd, 1, timeout_ms);
        if (ready <= 0) return -1;
        ssize_t count = read(fd, buffer + used, 3 - used);
        if (count <= 0) return -1;
        used += (size_t)count;
    }
    if (buffer[0] != 0x04 || (size_t)buffer[2] + 3 > capacity) return -1;
    size_t total = (size_t)buffer[2] + 3;
    while (used < total) {
        struct pollfd pfd = {.fd = fd, .events = POLLIN};
        int ready = poll(&pfd, 1, timeout_ms);
        if (ready <= 0) return -1;
        ssize_t count = read(fd, buffer + used, total - used);
        if (count <= 0) return -1;
        used += (size_t)count;
    }
    return (int)used;
}

static int command(int fd, uint16_t opcode, const uint8_t *payload, uint8_t length,
                   uint8_t *event, size_t event_capacity)
{
    uint8_t packet[4 + 255];
    packet[0] = 0x01;
    packet[1] = opcode & 0xff;
    packet[2] = opcode >> 8;
    packet[3] = length;
    if (length) memcpy(packet + 4, payload, length);
    if (write(fd, packet, (size_t)length + 4) != (ssize_t)length + 4) return -1;
    tcdrain(fd);

    for (int attempts = 0; attempts < 8; ++attempts) {
        int count = read_event(fd, event, event_capacity, 1500);
        if (count < 0) return -1;
        if (event[1] == 0x0e && count >= 7 &&
            event[4] == (opcode & 0xff) && event[5] == (opcode >> 8)) {
            return count;
        }
    }
    return -1;
}

int main(void)
{
    int gpio = gpio_power_open();
    if (gpio < 0) return 2;

    const speed_t speeds[] = {B115200, B3000000};
    const char *names[] = {"115200", "3000000"};
    int result = 1;

    for (size_t i = 0; i < ARRAY_SIZE(speeds); ++i) {
        int uart = uart_open(speeds[i]);
        if (uart < 0) continue;
        uint8_t event[260];
        printf("probe baud=%s\n", names[i]);
        if (command(uart, 0x0c03, NULL, 0, event, sizeof(event)) >= 7 && event[6] == 0 &&
            command(uart, 0x1001, NULL, 0, event, sizeof(event)) >= 15 && event[6] == 0) {
            printf("controller ready baud=%s hci=%u revision=%u lmp=%u manufacturer=%u subversion=%u\n",
                   names[i], event[7], (unsigned)(event[8] | event[9] << 8), event[10],
                   (unsigned)(event[11] | event[12] << 8),
                   (unsigned)(event[13] | event[14] << 8));
            result = 0;
            close(uart);
            break;
        }
        close(uart);
        gpio_set(gpio, 0);
        usleep(50000);
        gpio_set(gpio, 0x3);
        usleep(100000);
    }

    gpio_set(gpio, 0);
    close(gpio);
    if (result) fprintf(stderr, "controller did not answer\n");
    return result;
}
