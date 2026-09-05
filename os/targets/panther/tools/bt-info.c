#define _GNU_SOURCE
#include <errno.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#ifndef AF_BLUETOOTH
#define AF_BLUETOOTH 31
#endif
#define BTPROTO_HCI 1
#define HCI_CHANNEL_CONTROL 3
#define HCI_DEV_NONE 0xffff
#define MGMT_EV_CMD_COMPLETE 0x0001
#define MGMT_OP_READ_INFO 0x0004
#define MGMT_SETTING_POWERED (1u << 0)
#define MGMT_SETTING_CONNECTABLE (1u << 1)
#define MGMT_SETTING_BREDR (1u << 7)
#define MGMT_SETTING_LE (1u << 9)

struct sockaddr_hci {
    sa_family_t hci_family;
    unsigned short hci_dev;
    unsigned short hci_channel;
};

static uint16_t little16(const uint8_t *p) {
    return (uint16_t)(p[0] | p[1] << 8);
}

static uint32_t little32(const uint8_t *p) {
    return (uint32_t)p[0] | (uint32_t)p[1] << 8 |
           (uint32_t)p[2] << 16 | (uint32_t)p[3] << 24;
}

static void put_little16(uint8_t *p, uint16_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void print_settings(const char *label, uint32_t value) {
    printf("%s raw=%08x powered=%u connectable=%u bredr=%u le=%u\n",
           label, value, !!(value & MGMT_SETTING_POWERED),
           !!(value & MGMT_SETTING_CONNECTABLE),
           !!(value & MGMT_SETTING_BREDR),
           !!(value & MGMT_SETTING_LE));
}

int main(void) {
    int fd = socket(AF_BLUETOOTH, SOCK_RAW | SOCK_CLOEXEC, BTPROTO_HCI);
    if (fd < 0) {
        perror("socket");
        return 1;
    }
    struct sockaddr_hci address = {
        .hci_family = AF_BLUETOOTH,
        .hci_dev = HCI_DEV_NONE,
        .hci_channel = HCI_CHANNEL_CONTROL,
    };
    if (bind(fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("bind");
        return 1;
    }
    uint8_t command[6] = {0};
    put_little16(command, MGMT_OP_READ_INFO);
    put_little16(command + 2, 0);
    if (write(fd, command, sizeof(command)) != (ssize_t)sizeof(command)) {
        perror("write");
        return 1;
    }
    struct pollfd ready = {.fd = fd, .events = POLLIN};
    while (poll(&ready, 1, 3000) > 0) {
        uint8_t event[512];
        ssize_t count = read(fd, event, sizeof(event));
        if (count < 9 || little16(event) != MGMT_EV_CMD_COMPLETE ||
            little16(event + 6) != MGMT_OP_READ_INFO) {
            continue;
        }
        uint16_t length = little16(event + 4);
        if (event[8] != 0 || length < 20 || count < (ssize_t)length + 6) {
            printf("INFO-ERROR status=%u length=%u\n", event[8], length);
            return 1;
        }
        const uint8_t *info = event + 9;
        print_settings("SUPPORTED", little32(info + 9));
        print_settings("CURRENT", little32(info + 13));
        return 0;
    }
    puts("INFO-ERROR timeout");
    return 1;
}
