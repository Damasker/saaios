#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#ifndef AF_BLUETOOTH
#define AF_BLUETOOTH 31
#endif
#define BTPROTO_HCI 1
#define HCI_CHANNEL_RAW 0
#define HCI_CHANNEL_CONTROL 3
#define HCI_DEV_NONE 0xffff
#define HCIDEVUP _IOW('H', 201, int)
#define MGMT_EV_CMD_COMPLETE 0x0001
#define MGMT_EV_CMD_STATUS 0x0002
#define MGMT_EV_DEVICE_FOUND 0x0012
#define MGMT_OP_SET_POWERED 0x0005
#define MGMT_OP_SET_BONDABLE 0x0009
#define MGMT_OP_SET_LE 0x000d
#define MGMT_OP_START_DISCOVERY 0x0023
#define MGMT_OP_STOP_DISCOVERY 0x0024

struct sockaddr_hci {
    sa_family_t hci_family;
    unsigned short hci_dev;
    unsigned short hci_channel;
};

struct device_record {
    uint8_t address[6];
    uint8_t type;
    char name[32];
};

static uint16_t little16(const uint8_t *data) {
    return (uint16_t)(data[0] | data[1] << 8);
}

static void put_little16(uint8_t *data, uint16_t value) {
    data[0] = (uint8_t)value;
    data[1] = (uint8_t)(value >> 8);
}

static long milliseconds(void) {
    struct timespec now = {0};
    clock_gettime(CLOCK_MONOTONIC, &now);
    return now.tv_sec * 1000L + now.tv_nsec / 1000000L;
}

static int make_controller_ready(void) {
    int fd = socket(AF_BLUETOOTH, SOCK_RAW | SOCK_CLOEXEC, BTPROTO_HCI);
    if (fd < 0) {
        return -1;
    }
    int result = ioctl(fd, HCIDEVUP, 0);
    int saved_errno = errno;
    close(fd);
    return result == 0 || saved_errno == EALREADY ? 0 : -1;
}

static int send_management_command(int fd, uint16_t opcode,
                                   const uint8_t *parameters,
                                   uint16_t length) {
    uint8_t packet[64];
    if ((size_t)length + 6 > sizeof(packet)) {
        return -EINVAL;
    }
    put_little16(packet, opcode);
    put_little16(packet + 2, 0);
    put_little16(packet + 4, length);
    if (length) {
        memcpy(packet + 6, parameters, length);
    }
    if (write(fd, packet, (size_t)length + 6) != (ssize_t)length + 6) {
        return -errno;
    }

    long deadline = milliseconds() + 4000;
    while (milliseconds() < deadline) {
        struct pollfd ready = {.fd = fd, .events = POLLIN};
        int remaining = (int)(deadline - milliseconds());
        if (poll(&ready, 1, remaining > 0 ? remaining : 0) <= 0) {
            break;
        }
        uint8_t event[1024];
        ssize_t count = read(fd, event, sizeof(event));
        if (count < 9) {
            continue;
        }
        uint16_t event_code = little16(event);
        uint16_t payload_length = little16(event + 4);
        if ((size_t)count < (size_t)payload_length + 6 ||
            (event_code != MGMT_EV_CMD_COMPLETE &&
             event_code != MGMT_EV_CMD_STATUS) ||
            little16(event + 6) != opcode) {
            continue;
        }
        return event[8] == 0 ? 0 : -(int)event[8];
    }
    return -ETIMEDOUT;
}

static void safe_name(char output[32], const uint8_t *input, size_t length) {
    size_t written = 0;
    while (written < length && written < 31) {
        unsigned char character = input[written];
        if ((character >= 'A' && character <= 'Z') ||
            (character >= 'a' && character <= 'z') ||
            (character >= '0' && character <= '9') ||
            character == ' ' || character == '-' || character == '.' ||
            character == '_') {
            output[written] = (char)character;
        } else {
            output[written] = '-';
        }
        ++written;
    }
    while (written > 0 && (output[written - 1] == ' ' ||
                           output[written - 1] == '-')) {
        --written;
    }
    output[written] = '\0';
}

static void advertising_name(char output[32], const uint8_t *data,
                             size_t length) {
    output[0] = '\0';
    size_t offset = 0;
    while (offset < length) {
        size_t field_length = data[offset];
        if (field_length == 0) {
            break;
        }
        if (offset + field_length >= length || field_length < 1) {
            break;
        }
        uint8_t type = data[offset + 1];
        if (type == 0x09 || (type == 0x08 && output[0] == '\0')) {
            safe_name(output, data + offset + 2, field_length - 1);
            if (type == 0x09) {
                return;
            }
        }
        offset += field_length + 1;
    }
}

static int known_name(char names[][32], int count, const char *name) {
    for (int index = 0; index < count; ++index) {
        if (!strcmp(names[index], name)) {
            return 1;
        }
    }
    return 0;
}

int main(void) {
    puts("SCANNING");
    fflush(stdout);
    int state_fd = open("/run/bluetooth-devices.bin",
                        O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
    if (state_fd < 0) {
        perror("bluetooth device list");
        return 1;
    }
    if (make_controller_ready() < 0) {
        perror("bluetooth controller up");
        return 1;
    }

    int fd = socket(AF_BLUETOOTH, SOCK_RAW | SOCK_CLOEXEC, BTPROTO_HCI);
    if (fd < 0) {
        perror("bluetooth management socket");
        return 1;
    }
    struct sockaddr_hci address = {
        .hci_family = AF_BLUETOOTH,
        .hci_dev = HCI_DEV_NONE,
        .hci_channel = HCI_CHANNEL_CONTROL,
    };
    if (bind(fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("bluetooth management bind");
        close(fd);
        return 1;
    }

    const uint8_t address_types = 0x07; /* BR/EDR, LE public and LE random */
    const uint8_t enabled = 0x01;
    const uint8_t disabled = 0x00;
    int setup_status = 0;
    if (access("/run/bluetooth-smp-ready", F_OK) != 0) {
        setup_status = send_management_command(fd, MGMT_OP_SET_POWERED,
                                               &disabled, 1);
        if (setup_status == 0) {
            setup_status = send_management_command(fd, MGMT_OP_SET_LE,
                                                   &enabled, 1);
        }
        if (setup_status == 0) {
            setup_status = send_management_command(fd, MGMT_OP_SET_BONDABLE,
                                                   &enabled, 1);
        }
        if (setup_status == 0) {
            setup_status = send_management_command(fd, MGMT_OP_SET_POWERED,
                                                   &enabled, 1);
        }
        if (setup_status == 0) {
            int marker = open("/run/bluetooth-smp-ready",
                              O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
            if (marker >= 0) close(marker);
        }
    } else {
        setup_status = send_management_command(fd, MGMT_OP_SET_LE,
                                               &enabled, 1);
    }
    if (setup_status != 0) {
        fprintf(stderr, "bluetooth security setup status=%d\n", setup_status);
        close(fd);
        return 1;
    }
    /* Clear discovery state left by an interrupted earlier scan. */
    (void)send_management_command(fd, MGMT_OP_STOP_DISCOVERY,
                                  &address_types, 1);
    int status = send_management_command(fd, MGMT_OP_START_DISCOVERY,
                                         &address_types, 1);
    if (status != 0) {
        fprintf(stderr, "bluetooth discovery start status=%d\n", status);
        close(fd);
        return 1;
    }

    char names[8][32] = {{0}};
    int name_count = 0;
    long deadline = milliseconds() + 8000;
    while (milliseconds() < deadline) {
        struct pollfd ready = {.fd = fd, .events = POLLIN};
        int remaining = (int)(deadline - milliseconds());
        if (poll(&ready, 1, remaining > 0 ? remaining : 0) <= 0) {
            break;
        }
        uint8_t event[1024];
        ssize_t count = read(fd, event, sizeof(event));
        if (count < 20 || little16(event) != MGMT_EV_DEVICE_FOUND) {
            continue;
        }
        size_t payload_length = little16(event + 4);
        if ((size_t)count < payload_length + 6 || payload_length < 14) {
            continue;
        }
        const uint8_t *payload = event + 6;
        size_t data_length = little16(payload + 12);
        if (14 + data_length > payload_length) {
            continue;
        }
        char name[32];
        advertising_name(name, payload + 14, data_length);
        if (name[0] && name_count < 8 &&
            !known_name(names, name_count, name)) {
            struct device_record record = {0};
            memcpy(record.address, payload, sizeof(record.address));
            record.type = payload[6];
            snprintf(record.name, sizeof(record.name), "%s", name);
            if (write(state_fd, &record, sizeof(record)) !=
                (ssize_t)sizeof(record)) {
                perror("bluetooth device list write");
            }
            snprintf(names[name_count], sizeof(names[name_count]),
                     "%s", name);
            printf("DEVICE\t%s\n", names[name_count]);
            printf("TRANSPORT\t%s\t%s\n", names[name_count],
                   record.type == 0 ? "CLASSIC" : "BLE");
            fflush(stdout);
            ++name_count;
        }
    }
    (void)send_management_command(fd, MGMT_OP_STOP_DISCOVERY,
                                  &address_types, 1);
    printf("DONE\t%d\n", name_count);
    close(state_fd);
    close(fd);
    return 0;
}
