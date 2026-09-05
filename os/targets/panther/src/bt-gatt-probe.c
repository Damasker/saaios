#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#ifndef AF_BLUETOOTH
#define AF_BLUETOOTH 31
#endif
#define BTPROTO_L2CAP 0
#define ATT_CID 0x0004
#define ATT_OP_ERROR_RESPONSE 0x01
#define ATT_OP_READ_BY_GROUP_REQUEST 0x10
#define ATT_OP_READ_BY_GROUP_RESPONSE 0x11

struct device_record {
    uint8_t address[6];
    uint8_t type;
    char name[32];
};

struct device_store {
    uint8_t magic[8];
    uint32_t version;
    uint32_t count;
    struct device_record devices[8];
} __attribute__((packed));

struct sockaddr_l2 {
    sa_family_t l2_family;
    uint16_t l2_psm;
    uint8_t l2_bdaddr[6];
    uint16_t l2_cid;
    uint8_t l2_bdaddr_type;
};

static uint16_t little16(const uint8_t *p) {
    return (uint16_t)(p[0] | p[1] << 8);
}

static void put_little16(uint8_t *p, uint16_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static int read_devices(struct device_store *store) {
    int fd = open("/metadata/saaios/bluetooth.devices",
                  O_RDONLY | O_CLOEXEC);
    if (fd < 0) return -1;
    ssize_t count = read(fd, store, sizeof(*store));
    close(fd);
    return count == (ssize_t)sizeof(*store) &&
           !memcmp(store->magic, "SAAIDEV1", 8) &&
           store->version == 1 && store->count <= 8 ? 0 : -1;
}

static int receive_att(int fd, uint8_t *packet, size_t capacity) {
    struct pollfd ready = {.fd = fd, .events = POLLIN};
    int status = poll(&ready, 1, 5000);
    if (status <= 0) return status == 0 ? -ETIMEDOUT : -errno;
    ssize_t count = read(fd, packet, capacity);
    return count < 0 ? -errno : (int)count;
}

int main(int argc, char **argv) {
    unsigned long selected = 0;
    if (argc == 2) {
        char *end = NULL;
        selected = strtoul(argv[1], &end, 10);
        if (!end || *end) return 2;
    }
    struct device_store store;
    if (read_devices(&store) < 0 || selected >= store.count) {
        puts("GATT-ERROR NO-DEVICE");
        return 2;
    }
    const struct device_record *device = &store.devices[selected];
    if (device->type == 0) {
        puts("GATT-ERROR CLASSIC-DEVICE");
        return 2;
    }
    int fd = socket(AF_BLUETOOTH, SOCK_SEQPACKET | SOCK_CLOEXEC,
                    BTPROTO_L2CAP);
    if (fd < 0) {
        printf("GATT-ERROR SOCKET-%d\n", errno);
        return 1;
    }
    struct sockaddr_l2 local = {
        .l2_family = AF_BLUETOOTH,
        .l2_psm = 0,
        .l2_cid = ATT_CID,
        .l2_bdaddr_type = 1,
    };
    if (bind(fd, (struct sockaddr *)&local, sizeof(local)) < 0) {
        printf("GATT-ERROR BIND-%d\n", errno);
        close(fd);
        return 1;
    }
    struct sockaddr_l2 remote = {
        .l2_family = AF_BLUETOOTH,
        .l2_psm = 0,
        .l2_cid = ATT_CID,
        .l2_bdaddr_type = device->type,
    };
    memcpy(remote.l2_bdaddr, device->address, 6);
    if (connect(fd, (struct sockaddr *)&remote, sizeof(remote)) < 0) {
        printf("GATT-ERROR CONNECT-%d\n", errno);
        close(fd);
        return 1;
    }
    printf("GATT-CONNECTED %s\n", device->name);
    uint16_t start = 1;
    unsigned services = 0;
    while (start != 0) {
        uint8_t request[7] = {ATT_OP_READ_BY_GROUP_REQUEST};
        put_little16(request + 1, start);
        put_little16(request + 3, 0xffff);
        put_little16(request + 5, 0x2800);
        if (write(fd, request, sizeof(request)) != (ssize_t)sizeof(request)) {
            printf("GATT-ERROR WRITE-%d\n", errno);
            break;
        }
        uint8_t response[512];
        int count = receive_att(fd, response, sizeof(response));
        if (count < 1) {
            printf("GATT-ERROR READ-%d\n", -count);
            break;
        }
        if (response[0] == ATT_OP_ERROR_RESPONSE) {
            if (count >= 5 && response[4] == 0x0a) break;
            printf("GATT-ERROR ATT-%u\n", count >= 5 ? response[4] : 0xff);
            break;
        }
        if (response[0] != ATT_OP_READ_BY_GROUP_RESPONSE ||
            count < 2 || response[1] < 6) {
            printf("GATT-ERROR RESPONSE-%u\n", response[0]);
            break;
        }
        uint8_t entry_length = response[1];
        uint16_t last = 0;
        for (int offset = 2; offset + entry_length <= count;
             offset += entry_length) {
            uint16_t first = little16(response + offset);
            last = little16(response + offset + 2);
            if (entry_length == 6) {
                printf("SERVICE %04X %04X-%04X\n",
                       little16(response + offset + 4), first, last);
            } else {
                printf("SERVICE 128BIT %04X-%04X\n", first, last);
            }
            ++services;
        }
        if (last == 0 || last == 0xffff) break;
        start = (uint16_t)(last + 1);
    }
    printf("GATT-DONE %u\n", services);
    close(fd);
    return 0;
}
