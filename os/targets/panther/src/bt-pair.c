#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#ifndef AF_BLUETOOTH
#define AF_BLUETOOTH 31
#endif
#define BTPROTO_HCI 1
#define HCI_CHANNEL_CONTROL 3
#define HCI_DEV_NONE 0xffff
#define MGMT_EV_CMD_COMPLETE 0x0001
#define MGMT_EV_CMD_STATUS 0x0002
#define MGMT_EV_NEW_LINK_KEY 0x0009
#define MGMT_EV_NEW_LONG_TERM_KEY 0x000a
#define MGMT_EV_PIN_CODE_REQUEST 0x000e
#define MGMT_EV_USER_CONFIRM_REQUEST 0x000f
#define MGMT_EV_USER_PASSKEY_REQUEST 0x0010
#define MGMT_EV_AUTH_FAILED 0x0011
#define MGMT_EV_NEW_IRK 0x0018
#define MGMT_OP_SET_POWERED 0x0005
#define MGMT_OP_SET_BONDABLE 0x0009
#define MGMT_OP_SET_LE 0x000d
#define MGMT_OP_LOAD_LINK_KEYS 0x0012
#define MGMT_OP_LOAD_LONG_TERM_KEYS 0x0013
#define MGMT_OP_GET_CONNECTIONS 0x0015
#define MGMT_OP_PAIR_DEVICE 0x0019
#define MGMT_OP_UNPAIR_DEVICE 0x001b
#define MGMT_OP_USER_CONFIRM_REPLY 0x001c
#define MGMT_OP_LOAD_IRKS 0x0030
#define MGMT_OP_ADD_DEVICE 0x0033
#define MGMT_OP_REMOVE_DEVICE 0x0034
#define MAX_KEYS 16
#define MAX_DEVICES 8
#define LINK_KEY_SIZE 25
#define LTK_SIZE 36
#define IRK_SIZE 23

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

struct key_store {
    uint8_t magic[8];
    uint32_t version;
    uint32_t ltk_count;
    uint8_t ltks[MAX_KEYS][LTK_SIZE];
    uint32_t irk_count;
    uint8_t irks[MAX_KEYS][IRK_SIZE];
} __attribute__((packed));

struct device_store {
    uint8_t magic[8];
    uint32_t version;
    uint32_t count;
    struct device_record devices[MAX_DEVICES];
} __attribute__((packed));

struct link_key_store {
    uint8_t magic[8];
    uint32_t version;
    uint32_t count;
    uint8_t keys[MAX_KEYS][LINK_KEY_SIZE];
} __attribute__((packed));

static const uint8_t store_magic[8] = {'S','A','A','I','B','T','1',0};
static const uint8_t device_magic[8] = {'S','A','A','I','D','E','V','1'};
static const uint8_t link_key_magic[8] = {'S','A','A','I','L','K','1',0};

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

static void initialize_store(struct key_store *store) {
    memset(store, 0, sizeof(*store));
    memcpy(store->magic, store_magic, sizeof(store_magic));
    store->version = 1;
}

static void load_store(struct key_store *store) {
    initialize_store(store);
    int fd = open("/metadata/saaios/bluetooth.keys", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return;
    struct key_store candidate;
    ssize_t count = read(fd, &candidate, sizeof(candidate));
    close(fd);
    if (count == (ssize_t)sizeof(candidate) &&
        !memcmp(candidate.magic, store_magic, sizeof(store_magic)) &&
        candidate.version == 1 && candidate.ltk_count <= MAX_KEYS &&
        candidate.irk_count <= MAX_KEYS) {
        *store = candidate;
    }
}

static int save_store(const struct key_store *store) {
    const char *temporary = "/metadata/saaios/bluetooth.keys.new";
    int fd = open(temporary, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
    if (fd < 0) return -1;
    ssize_t count = write(fd, store, sizeof(*store));
    if (count == (ssize_t)sizeof(*store)) (void)fsync(fd);
    close(fd);
    if (count != (ssize_t)sizeof(*store)) {
        (void)unlink(temporary);
        return -1;
    }
    (void)chmod(temporary, 0600);
    return rename(temporary, "/metadata/saaios/bluetooth.keys");
}

static void initialize_devices(struct device_store *store) {
    memset(store, 0, sizeof(*store));
    memcpy(store->magic, device_magic, sizeof(device_magic));
    store->version = 1;
}

static void load_devices(struct device_store *store) {
    initialize_devices(store);
    int fd = open("/metadata/saaios/bluetooth.devices",
                  O_RDONLY | O_CLOEXEC);
    if (fd < 0) return;
    struct device_store candidate;
    ssize_t count = read(fd, &candidate, sizeof(candidate));
    close(fd);
    if (count == (ssize_t)sizeof(candidate) &&
        !memcmp(candidate.magic, device_magic, sizeof(device_magic)) &&
        candidate.version == 1 && candidate.count <= MAX_DEVICES) {
        *store = candidate;
    }
}

static int save_devices(const struct device_store *store) {
    const char *temporary = "/metadata/saaios/bluetooth.devices.new";
    int fd = open(temporary, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
    if (fd < 0) return -1;
    ssize_t count = write(fd, store, sizeof(*store));
    if (count == (ssize_t)sizeof(*store)) (void)fsync(fd);
    close(fd);
    if (count != (ssize_t)sizeof(*store)) {
        (void)unlink(temporary);
        return -1;
    }
    (void)chmod(temporary, 0600);
    return rename(temporary, "/metadata/saaios/bluetooth.devices");
}

static void initialize_link_keys(struct link_key_store *store) {
    memset(store, 0, sizeof(*store));
    memcpy(store->magic, link_key_magic, sizeof(link_key_magic));
    store->version = 1;
}

static void load_link_keys(struct link_key_store *store) {
    initialize_link_keys(store);
    int fd = open("/metadata/saaios/bluetooth.linkkeys",
                  O_RDONLY | O_CLOEXEC);
    if (fd < 0) return;
    struct link_key_store candidate;
    ssize_t count = read(fd, &candidate, sizeof(candidate));
    close(fd);
    if (count == (ssize_t)sizeof(candidate) &&
        !memcmp(candidate.magic, link_key_magic, sizeof(link_key_magic)) &&
        candidate.version == 1 && candidate.count <= MAX_KEYS) {
        *store = candidate;
    }
}

static int save_link_keys(const struct link_key_store *store) {
    const char *temporary = "/metadata/saaios/bluetooth.linkkeys.new";
    int fd = open(temporary, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
    if (fd < 0) return -1;
    ssize_t count = write(fd, store, sizeof(*store));
    if (count == (ssize_t)sizeof(*store)) (void)fsync(fd);
    close(fd);
    if (count != (ssize_t)sizeof(*store)) {
        (void)unlink(temporary);
        return -1;
    }
    (void)chmod(temporary, 0600);
    return rename(temporary, "/metadata/saaios/bluetooth.linkkeys");
}

static void remember_device(struct device_store *store,
                            const struct device_record *device) {
    for (uint32_t index = 0; index < store->count; ++index) {
        if (!memcmp(store->devices[index].address, device->address, 6) &&
            store->devices[index].type == device->type) {
            store->devices[index] = *device;
            return;
        }
    }
    if (store->count < MAX_DEVICES) {
        store->devices[store->count++] = *device;
    }
}

static void seed_devices_from_keys(struct device_store *devices,
                                   const struct key_store *keys) {
    for (uint32_t index = 0; index < keys->ltk_count; ++index) {
        struct device_record device = {0};
        memcpy(device.address, keys->ltks[index], 6);
        device.type = keys->ltks[index][6];
        snprintf(device.name, sizeof(device.name), "PAIRED DEVICE");
        remember_device(devices, &device);
    }
}

static void publish_devices(const struct device_store *store) {
    int fd = open("/run/bluetooth-saved.log",
                  O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (fd < 0) return;
    for (uint32_t index = 0; index < store->count; ++index) {
        char line[64];
        int length = snprintf(line, sizeof(line), "SAVED\t%s\n",
                              store->devices[index].name);
        if (length > 0) (void)write(fd, line, (size_t)length);
    }
    close(fd);
}

static void remember_ltk(struct key_store *store, const uint8_t *payload,
                         size_t length) {
    if (!store || length < 1 + LTK_SIZE || payload[0] == 0) return;
    const uint8_t *key = payload + 1;
    for (uint32_t index = 0; index < store->ltk_count; ++index) {
        if (!memcmp(store->ltks[index], key, 9)) {
            memcpy(store->ltks[index], key, LTK_SIZE);
            return;
        }
    }
    if (store->ltk_count < MAX_KEYS) {
        memcpy(store->ltks[store->ltk_count++], key, LTK_SIZE);
    }
}

static void remember_irk(struct key_store *store, const uint8_t *payload,
                         size_t length) {
    if (!store || length < 7 + IRK_SIZE || payload[0] == 0) return;
    const uint8_t *key = payload + 7;
    for (uint32_t index = 0; index < store->irk_count; ++index) {
        if (!memcmp(store->irks[index], key, 7)) {
            memcpy(store->irks[index], key, IRK_SIZE);
            return;
        }
    }
    if (store->irk_count < MAX_KEYS) {
        memcpy(store->irks[store->irk_count++], key, IRK_SIZE);
    }
}

static void remember_link_key(struct link_key_store *store,
                              const uint8_t *payload, size_t length) {
    if (!store || length < 1 + LINK_KEY_SIZE || payload[0] == 0) return;
    const uint8_t *key = payload + 1;
    for (uint32_t index = 0; index < store->count; ++index) {
        if (!memcmp(store->keys[index], key, 7)) {
            memcpy(store->keys[index], key, LINK_KEY_SIZE);
            return;
        }
    }
    if (store->count < MAX_KEYS) {
        memcpy(store->keys[store->count++], key, LINK_KEY_SIZE);
    }
}

static int open_management(void) {
    int fd = socket(AF_BLUETOOTH, SOCK_RAW | SOCK_CLOEXEC, BTPROTO_HCI);
    if (fd < 0) return -1;
    struct sockaddr_hci address = {
        .hci_family = AF_BLUETOOTH,
        .hci_dev = HCI_DEV_NONE,
        .hci_channel = HCI_CHANNEL_CONTROL,
    };
    if (bind(fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

static int write_command(int fd, uint16_t opcode,
                         const uint8_t *parameters, uint16_t length) {
    uint8_t packet[2048];
    if ((size_t)length + 6 > sizeof(packet)) return -EINVAL;
    put_little16(packet, opcode);
    put_little16(packet + 2, 0);
    put_little16(packet + 4, length);
    if (length) memcpy(packet + 6, parameters, length);
    if (write(fd, packet, (size_t)length + 6) != (ssize_t)length + 6) {
        return -errno;
    }
    return 0;
}

static int wait_for_command(int fd, uint16_t opcode, int timeout_ms,
                            const struct device_record *device,
                            struct key_store *store,
                            struct link_key_store *link_keys) {
    long deadline = milliseconds() + timeout_ms;
    while (milliseconds() < deadline) {
        struct pollfd ready = {.fd = fd, .events = POLLIN};
        int remaining = (int)(deadline - milliseconds());
        if (poll(&ready, 1, remaining > 0 ? remaining : 0) <= 0) break;
        uint8_t event[2048];
        ssize_t count = read(fd, event, sizeof(event));
        if (count < 6) continue;
        uint16_t event_code = little16(event);
        uint16_t payload_length = little16(event + 4);
        if ((size_t)count < (size_t)payload_length + 6) continue;
        const uint8_t *payload = event + 6;
        if (event_code == MGMT_EV_NEW_LINK_KEY) {
            remember_link_key(link_keys, payload, payload_length);
            continue;
        }
        if (event_code == MGMT_EV_NEW_LONG_TERM_KEY) {
            remember_ltk(store, payload, payload_length);
            continue;
        }
        if (event_code == MGMT_EV_NEW_IRK) {
            remember_irk(store, payload, payload_length);
            continue;
        }
        if ((event_code == MGMT_EV_CMD_COMPLETE ||
             event_code == MGMT_EV_CMD_STATUS) && payload_length >= 3 &&
            little16(payload) == opcode) {
            return payload[2] == 0 ? 0 : -(int)payload[2];
        }
        if (event_code == MGMT_EV_USER_CONFIRM_REQUEST &&
            payload_length >= 12 && device && payload[7] == 1) {
            (void)write_command(fd, MGMT_OP_USER_CONFIRM_REPLY, payload, 7);
            continue;
        }
        if ((event_code == MGMT_EV_PIN_CODE_REQUEST ||
             event_code == MGMT_EV_USER_PASSKEY_REQUEST) && device) {
            return -0x20;
        }
        if (event_code == MGMT_EV_AUTH_FAILED &&
            payload_length >= 8 && device) {
            return -(int)payload[7];
        }
    }
    return -ETIMEDOUT;
}

static int management_command(int fd, uint16_t opcode,
                              const uint8_t *parameters, uint16_t length,
                              int timeout_ms,
                              const struct device_record *device,
                              struct key_store *store,
                              struct link_key_store *link_keys) {
    int status = write_command(fd, opcode, parameters, length);
    if (status != 0) return status;
    return wait_for_command(fd, opcode, timeout_ms, device, store, link_keys);
}

static int restore_keys(void) {
    puts("RESTORING");
    fflush(stdout);
    int fd = open_management();
    if (fd < 0) {
        puts("RESTORE-ERROR\tNO-CONTROLLER");
        return 1;
    }
    struct key_store store;
    load_store(&store);
    struct link_key_store link_keys;
    load_link_keys(&link_keys);
    struct device_store devices;
    load_devices(&devices);
    if (devices.count == 0 && store.ltk_count > 0) {
        seed_devices_from_keys(&devices, &store);
        (void)save_devices(&devices);
    }
    publish_devices(&devices);
    const uint8_t disabled = 0;
    const uint8_t enabled = 1;
    int status = management_command(fd, MGMT_OP_SET_POWERED,
                                    &disabled, 1, 5000, NULL, NULL, NULL);
    if (status == 0) status = management_command(
        fd, MGMT_OP_SET_LE, &enabled, 1, 5000, NULL, NULL, NULL);
    if (status == 0) status = management_command(
        fd, MGMT_OP_SET_BONDABLE, &enabled, 1, 5000, NULL, NULL, NULL);
    if (status == 0 && link_keys.count > 0) {
        uint8_t payload[3 + MAX_KEYS * LINK_KEY_SIZE];
        payload[0] = 0; /* Do not load debug keys. */
        put_little16(payload + 1, (uint16_t)link_keys.count);
        memcpy(payload + 3, link_keys.keys,
               link_keys.count * LINK_KEY_SIZE);
        status = management_command(
            fd, MGMT_OP_LOAD_LINK_KEYS, payload,
            (uint16_t)(3 + link_keys.count * LINK_KEY_SIZE),
            5000, NULL, NULL, NULL);
    }
    if (status == 0 && store.ltk_count > 0) {
        uint8_t payload[2 + MAX_KEYS * LTK_SIZE];
        put_little16(payload, (uint16_t)store.ltk_count);
        memcpy(payload + 2, store.ltks, store.ltk_count * LTK_SIZE);
        status = management_command(fd, MGMT_OP_LOAD_LONG_TERM_KEYS,
                                    payload,
                                    (uint16_t)(2 + store.ltk_count * LTK_SIZE),
                                    5000, NULL, NULL, NULL);
    }
    if (status == 0 && store.irk_count > 0) {
        uint8_t payload[2 + MAX_KEYS * IRK_SIZE];
        put_little16(payload, (uint16_t)store.irk_count);
        memcpy(payload + 2, store.irks, store.irk_count * IRK_SIZE);
        status = management_command(fd, MGMT_OP_LOAD_IRKS, payload,
                                    (uint16_t)(2 + store.irk_count * IRK_SIZE),
                                    5000, NULL, NULL, NULL);
    }
    if (status == 0) status = management_command(
        fd, MGMT_OP_SET_POWERED, &enabled, 1, 10000, NULL, NULL, NULL);
    if (status == 0) {
        int marker = open("/run/bluetooth-smp-ready",
                          O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
        if (marker >= 0) close(marker);
        for (uint32_t index = 0; index < store.ltk_count; ++index) {
            uint8_t device[8];
            memcpy(device, store.ltks[index], 7);
            device[7] = 0x02; /* Auto-connect */
            (void)management_command(fd, MGMT_OP_ADD_DEVICE,
                                     device, sizeof(device),
                                     4000, NULL, NULL, NULL);
        }
        for (uint32_t index = 0; index < link_keys.count; ++index) {
            uint8_t device[8];
            memcpy(device, link_keys.keys[index], 7);
            device[7] = 0x02; /* Auto-connect */
            (void)management_command(fd, MGMT_OP_ADD_DEVICE,
                                     device, sizeof(device),
                                     4000, NULL, NULL, NULL);
        }
        printf("RESTORED\t%u\t%u\t%u\n",
               store.ltk_count, store.irk_count, link_keys.count);
    } else {
        printf("RESTORE-ERROR\tSTATUS-%d\n", -status);
    }
    close(fd);
    return status == 0 ? 0 : 1;
}

static int report_connections(void) {
    int fd = open_management();
    if (fd < 0 || write_command(fd, MGMT_OP_GET_CONNECTIONS, NULL, 0) != 0) {
        if (fd >= 0) close(fd);
        puts("CONNECTED\t0");
        return 1;
    }
    long deadline = milliseconds() + 4000;
    while (milliseconds() < deadline) {
        struct pollfd ready = {.fd = fd, .events = POLLIN};
        int remaining = (int)(deadline - milliseconds());
        if (poll(&ready, 1, remaining > 0 ? remaining : 0) <= 0) break;
        uint8_t event[1024];
        ssize_t count = read(fd, event, sizeof(event));
        if (count < 11 || little16(event) != MGMT_EV_CMD_COMPLETE) continue;
        uint16_t length = little16(event + 4);
        const uint8_t *payload = event + 6;
        if ((size_t)count < (size_t)length + 6 || length < 5 ||
            little16(payload) != MGMT_OP_GET_CONNECTIONS || payload[2] != 0) {
            continue;
        }
        printf("CONNECTED\t%u\n", little16(payload + 3));
        close(fd);
        return 0;
    }
    close(fd);
    puts("CONNECTED\t0");
    return 1;
}

static int forget_saved_device(const char *argument) {
    char *end = NULL;
    long selected = strtol(argument, &end, 10);
    struct key_store keys;
    struct link_key_store link_keys;
    struct device_store devices;
    load_store(&keys);
    load_link_keys(&link_keys);
    load_devices(&devices);
    if (!end || *end || selected < 0 ||
        selected >= (long)devices.count) {
        puts("FORGET-ERROR\tNO-DEVICE");
        return 2;
    }
    struct device_record forgotten = devices.devices[selected];
    int fd = open_management();
    if (fd >= 0) {
        uint8_t address[8];
        memcpy(address, forgotten.address, 6);
        address[6] = forgotten.type;
        address[7] = 0x02;
        (void)management_command(fd, MGMT_OP_REMOVE_DEVICE,
                                 address, 7, 4000, NULL, NULL, NULL);
        address[7] = 1;
        (void)management_command(fd, MGMT_OP_UNPAIR_DEVICE,
                                 address, 8, 6000, NULL, NULL, NULL);
        close(fd);
    }
    uint32_t target = 0;
    for (uint32_t index = 0; index < keys.ltk_count; ++index) {
        if (memcmp(keys.ltks[index], forgotten.address, 6) ||
            keys.ltks[index][6] != forgotten.type) {
            if (target != index) {
                memcpy(keys.ltks[target], keys.ltks[index], LTK_SIZE);
            }
            ++target;
        }
    }
    keys.ltk_count = target;
    target = 0;
    for (uint32_t index = 0; index < keys.irk_count; ++index) {
        if (memcmp(keys.irks[index], forgotten.address, 6) ||
            keys.irks[index][6] != forgotten.type) {
            if (target != index) {
                memcpy(keys.irks[target], keys.irks[index], IRK_SIZE);
            }
            ++target;
        }
    }
    keys.irk_count = target;
    target = 0;
    for (uint32_t index = 0; index < link_keys.count; ++index) {
        if (memcmp(link_keys.keys[index], forgotten.address, 6) ||
            link_keys.keys[index][6] != forgotten.type) {
            if (target != index) {
                memcpy(link_keys.keys[target], link_keys.keys[index],
                       LINK_KEY_SIZE);
            }
            ++target;
        }
    }
    link_keys.count = target;
    for (uint32_t index = (uint32_t)selected + 1;
         index < devices.count; ++index) {
        devices.devices[index - 1] = devices.devices[index];
    }
    --devices.count;
    if (save_store(&keys) < 0 || save_link_keys(&link_keys) < 0 ||
        save_devices(&devices) < 0) {
        puts("FORGET-ERROR\tSAVE-FAILED");
        return 1;
    }
    publish_devices(&devices);
    printf("FORGOTTEN\t%s\n", forgotten.name);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--restore")) return restore_keys();
    if (argc == 2 && !strcmp(argv[1], "--status")) return report_connections();
    if (argc == 3 && !strcmp(argv[1], "--forget")) {
        return forget_saved_device(argv[2]);
    }
    puts("PAIRING");
    fflush(stdout);
    if (argc != 2) {
        puts("PAIR-ERROR\tBAD-SELECTION");
        return 2;
    }
    char *end = NULL;
    long selected = strtol(argv[1], &end, 10);
    if (!end || *end || selected < 0 || selected > 7) {
        puts("PAIR-ERROR\tBAD-SELECTION");
        return 2;
    }
    int list_fd = open("/run/bluetooth-devices.bin", O_RDONLY | O_CLOEXEC);
    if (list_fd < 0 ||
        lseek(list_fd, selected * (off_t)sizeof(struct device_record),
              SEEK_SET) < 0) {
        puts("PAIR-ERROR\tNO-DEVICE");
        if (list_fd >= 0) close(list_fd);
        return 2;
    }
    struct device_record device = {0};
    if (read(list_fd, &device, sizeof(device)) != (ssize_t)sizeof(device)) {
        puts("PAIR-ERROR\tNO-DEVICE");
        close(list_fd);
        return 2;
    }
    close(list_fd);

    int fd = open_management();
    if (fd < 0) {
        puts("PAIR-ERROR\tNO-CONTROLLER");
        return 1;
    }
    struct key_store store;
    load_store(&store);
    struct link_key_store link_keys;
    load_link_keys(&link_keys);
    struct device_store devices;
    load_devices(&devices);
    const uint8_t enabled = 1;
    int status = management_command(fd, MGMT_OP_SET_LE,
                                    &enabled, 1, 4000, NULL, NULL, NULL);
    if (status == 0) status = management_command(
        fd, MGMT_OP_SET_BONDABLE, &enabled, 1, 4000, NULL, NULL, NULL);
    uint8_t parameters[8];
    memcpy(parameters, device.address, 6);
    parameters[6] = device.type;
    parameters[7] = 0x03;
    if (status == 0) status = management_command(
        fd, MGMT_OP_PAIR_DEVICE, parameters, sizeof(parameters),
        45000, &device, &store, &link_keys);
    if (status == 0 || status == -0x13) {
        remember_device(&devices, &device);
        if (save_store(&store) < 0 || save_link_keys(&link_keys) < 0 ||
            save_devices(&devices) < 0) {
            puts("PAIR-ERROR\tKEY-SAVE-FAILED");
            close(fd);
            return 1;
        }
        publish_devices(&devices);
        printf("PAIRED\t%s\n", device.name);
        printf("KEYS-SAVED\t%u\t%u\t%u\n",
               store.ltk_count, store.irk_count, link_keys.count);
        close(fd);
        return 0;
    }
    if (status == -0x20) puts("PAIR-ERROR\tCODE-REQUIRED");
    else if (status == -0x08) puts("PAIR-ERROR\tTIMEOUT");
    else if (status == -0x04) puts("PAIR-ERROR\tCONNECT-FAILED");
    else if (status == -0x05) puts("PAIR-ERROR\tAUTH-FAILED");
    else if (status == -0x0e) puts("PAIR-ERROR\tDISCONNECTED");
    else printf("PAIR-ERROR\tSTATUS-%d\n", -status);
    close(fd);
    return 1;
}
