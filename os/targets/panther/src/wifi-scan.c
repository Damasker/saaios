#define _GNU_SOURCE

#include <arpa/inet.h>
#include <errno.h>
#include <linux/genetlink.h>
#include <linux/netlink.h>
#include <linux/nl80211.h>
#include <net/if.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#ifndef NLA_ALIGNTO
#define NLA_ALIGNTO 4
#endif
#ifndef NLA_ALIGN
#define NLA_ALIGN(len) (((len) + NLA_ALIGNTO - 1) & ~(NLA_ALIGNTO - 1))
#endif
#ifndef NLA_HDRLEN
#define NLA_HDRLEN ((int)NLA_ALIGN(sizeof(struct nlattr)))
#endif

#define BUFFER_SIZE 65536

static uint32_t sequence = 1;

static int add_attr(struct nlmsghdr *header, size_t capacity, uint16_t type,
                    const void *data, size_t data_length) {
    size_t offset = NLMSG_ALIGN(header->nlmsg_len);
    size_t length = NLA_HDRLEN + data_length;
    size_t aligned = NLA_ALIGN(length);
    if (offset + aligned > capacity) {
        return -1;
    }
    struct nlattr *attr = (struct nlattr *)((char *)header + offset);
    attr->nla_type = type;
    attr->nla_len = (uint16_t)length;
    if (data_length) {
        memcpy((char *)attr + NLA_HDRLEN, data, data_length);
    }
    memset((char *)attr + length, 0, aligned - length);
    header->nlmsg_len = (uint32_t)(offset + aligned);
    return 0;
}

static int send_message(int fd, struct nlmsghdr *header) {
    struct sockaddr_nl destination = {.nl_family = AF_NETLINK};
    ssize_t sent = sendto(fd, header, header->nlmsg_len, 0,
                          (struct sockaddr *)&destination, sizeof(destination));
    return sent == (ssize_t)header->nlmsg_len ? 0 : -1;
}

static int receive_ack(int fd, uint32_t wanted_sequence) {
    char buffer[BUFFER_SIZE];
    for (;;) {
        ssize_t length = recv(fd, buffer, sizeof(buffer), 0);
        if (length < 0) {
            return -1;
        }
        for (struct nlmsghdr *header = (struct nlmsghdr *)buffer;
             NLMSG_OK(header, (unsigned int)length);
             header = NLMSG_NEXT(header, length)) {
            if (header->nlmsg_seq != wanted_sequence) {
                continue;
            }
            if (header->nlmsg_type == NLMSG_ERROR) {
                struct nlmsgerr *error = NLMSG_DATA(header);
                if (error->error == 0) {
                    return 0;
                }
                errno = -error->error;
                return -1;
            }
        }
    }
}

static int resolve_family(int fd, const char *name) {
    char request[512] = {0};
    struct nlmsghdr *header = (struct nlmsghdr *)request;
    struct genlmsghdr *generic = (struct genlmsghdr *)NLMSG_DATA(header);
    uint32_t request_sequence = sequence++;

    header->nlmsg_len = NLMSG_LENGTH(GENL_HDRLEN);
    header->nlmsg_type = GENL_ID_CTRL;
    header->nlmsg_flags = NLM_F_REQUEST;
    header->nlmsg_seq = request_sequence;
    generic->cmd = CTRL_CMD_GETFAMILY;
    generic->version = 1;
    if (add_attr(header, sizeof(request), CTRL_ATTR_FAMILY_NAME,
                 name, strlen(name) + 1) < 0 || send_message(fd, header) < 0) {
        return -1;
    }

    char response[BUFFER_SIZE];
    for (;;) {
        ssize_t length = recv(fd, response, sizeof(response), 0);
        if (length < 0) {
            return -1;
        }
        for (struct nlmsghdr *reply = (struct nlmsghdr *)response;
             NLMSG_OK(reply, (unsigned int)length);
             reply = NLMSG_NEXT(reply, length)) {
            if (reply->nlmsg_seq != request_sequence) {
                continue;
            }
            if (reply->nlmsg_type == NLMSG_ERROR) {
                struct nlmsgerr *error = NLMSG_DATA(reply);
                errno = error->error ? -error->error : EPROTO;
                return -1;
            }
            struct genlmsghdr *reply_generic = NLMSG_DATA(reply);
            int remaining = (int)reply->nlmsg_len - NLMSG_LENGTH(GENL_HDRLEN);
            struct nlattr *attr = (struct nlattr *)((char *)reply_generic + GENL_HDRLEN);
            for (; remaining >= NLA_HDRLEN && attr->nla_len >= NLA_HDRLEN &&
                   attr->nla_len <= remaining;
                 remaining -= NLA_ALIGN(attr->nla_len),
                 attr = (struct nlattr *)((char *)attr + NLA_ALIGN(attr->nla_len))) {
                if ((attr->nla_type & NLA_TYPE_MASK) == CTRL_ATTR_FAMILY_ID &&
                    attr->nla_len >= NLA_HDRLEN + sizeof(uint16_t)) {
                    uint16_t family;
                    memcpy(&family, (char *)attr + NLA_HDRLEN, sizeof(family));
                    return family;
                }
            }
        }
    }
}

static int trigger_scan(int fd, int family, unsigned int ifindex) {
    char request[512] = {0};
    struct nlmsghdr *header = (struct nlmsghdr *)request;
    struct genlmsghdr *generic = (struct genlmsghdr *)NLMSG_DATA(header);
    uint32_t request_sequence = sequence++;

    header->nlmsg_len = NLMSG_LENGTH(GENL_HDRLEN);
    header->nlmsg_type = (uint16_t)family;
    header->nlmsg_flags = NLM_F_REQUEST | NLM_F_ACK;
    header->nlmsg_seq = request_sequence;
    generic->cmd = NL80211_CMD_TRIGGER_SCAN;
    generic->version = 1;

    if (add_attr(header, sizeof(request), NL80211_ATTR_IFINDEX,
                 &ifindex, sizeof(ifindex)) < 0) {
        return -1;
    }
    size_t nested_offset = NLMSG_ALIGN(header->nlmsg_len);
    struct nlattr *nested = (struct nlattr *)((char *)header + nested_offset);
    nested->nla_type = NL80211_ATTR_SCAN_SSIDS | NLA_F_NESTED;
    nested->nla_len = NLA_HDRLEN;
    header->nlmsg_len = (uint32_t)(nested_offset + NLA_HDRLEN);
    if (add_attr(header, sizeof(request), 1, NULL, 0) < 0) {
        return -1;
    }
    nested->nla_len = (uint16_t)(header->nlmsg_len - nested_offset);

    if (send_message(fd, header) < 0) {
        return -1;
    }
    return receive_ack(fd, request_sequence);
}

static void parse_ies(const unsigned char *ies, size_t length,
                      char *ssid, size_t ssid_size, bool *secured) {
    ssid[0] = '\0';
    *secured = false;
    for (size_t offset = 0; offset + 2 <= length;) {
        unsigned int id = ies[offset];
        unsigned int item_length = ies[offset + 1];
        offset += 2;
        if (offset + item_length > length) {
            break;
        }
        if (id == 0 && ssid[0] == '\0') {
            size_t copy = item_length < ssid_size - 1 ? item_length : ssid_size - 1;
            for (size_t i = 0; i < copy; ++i) {
                unsigned char character = ies[offset + i];
                ssid[i] = character >= 32 && character < 127 ? (char)character : '?';
            }
            ssid[copy] = '\0';
        }
        if (id == 48 || (id == 221 && item_length >= 4 &&
                         ies[offset] == 0x00 && ies[offset + 1] == 0x50 &&
                         ies[offset + 2] == 0xf2 && ies[offset + 3] == 0x01)) {
            *secured = true;
        }
        offset += item_length;
    }
}

static void print_bss(struct nlattr *bss) {
    int remaining = bss->nla_len - NLA_HDRLEN;
    struct nlattr *attr = (struct nlattr *)((char *)bss + NLA_HDRLEN);
    const unsigned char *bssid = NULL;
    const unsigned char *ies = NULL;
    size_t ies_length = 0;
    uint32_t frequency = 0;
    int32_t signal_mbm = 0;
    bool have_signal = false;

    for (; remaining >= NLA_HDRLEN && attr->nla_len >= NLA_HDRLEN &&
           attr->nla_len <= remaining;
         remaining -= NLA_ALIGN(attr->nla_len),
         attr = (struct nlattr *)((char *)attr + NLA_ALIGN(attr->nla_len))) {
        int type = attr->nla_type & NLA_TYPE_MASK;
        const void *data = (char *)attr + NLA_HDRLEN;
        size_t data_length = attr->nla_len - NLA_HDRLEN;
        if (type == NL80211_BSS_BSSID && data_length >= 6) {
            bssid = data;
        } else if (type == NL80211_BSS_FREQUENCY && data_length >= sizeof(frequency)) {
            memcpy(&frequency, data, sizeof(frequency));
        } else if (type == NL80211_BSS_SIGNAL_MBM && data_length >= sizeof(signal_mbm)) {
            memcpy(&signal_mbm, data, sizeof(signal_mbm));
            have_signal = true;
        } else if (type == NL80211_BSS_INFORMATION_ELEMENTS) {
            ies = data;
            ies_length = data_length;
        }
    }

    if (!bssid) {
        return;
    }
    char ssid[64];
    bool secured;
    parse_ies(ies, ies_length, ssid, sizeof(ssid), &secured);
    printf("%s\t%s\t%u MHz\t", ssid[0] ? ssid : "<hidden>",
           secured ? "secured" : "open", frequency);
    if (have_signal) {
        printf("%.2f dBm\t", signal_mbm / 100.0);
    } else {
        printf("? dBm\t");
    }
    printf("%02x:%02x:%02x:%02x:%02x:%02x\n",
           bssid[0], bssid[1], bssid[2], bssid[3], bssid[4], bssid[5]);
}

static int dump_scan(int fd, int family, unsigned int ifindex) {
    char request[512] = {0};
    struct nlmsghdr *header = (struct nlmsghdr *)request;
    struct genlmsghdr *generic = (struct genlmsghdr *)NLMSG_DATA(header);
    uint32_t request_sequence = sequence++;

    header->nlmsg_len = NLMSG_LENGTH(GENL_HDRLEN);
    header->nlmsg_type = (uint16_t)family;
    header->nlmsg_flags = NLM_F_REQUEST | NLM_F_DUMP;
    header->nlmsg_seq = request_sequence;
    generic->cmd = NL80211_CMD_GET_SCAN;
    generic->version = 1;
    if (add_attr(header, sizeof(request), NL80211_ATTR_IFINDEX,
                 &ifindex, sizeof(ifindex)) < 0 || send_message(fd, header) < 0) {
        return -1;
    }

    int count = 0;
    char response[BUFFER_SIZE];
    for (;;) {
        ssize_t length = recv(fd, response, sizeof(response), 0);
        if (length < 0) {
            return -1;
        }
        for (struct nlmsghdr *reply = (struct nlmsghdr *)response;
             NLMSG_OK(reply, (unsigned int)length);
             reply = NLMSG_NEXT(reply, length)) {
            if (reply->nlmsg_seq != request_sequence) {
                continue;
            }
            if (reply->nlmsg_type == NLMSG_DONE) {
                printf("Found %d access points.\n", count);
                return 0;
            }
            if (reply->nlmsg_type == NLMSG_ERROR) {
                struct nlmsgerr *error = NLMSG_DATA(reply);
                if (error->error == 0) {
                    continue;
                }
                errno = -error->error;
                return -1;
            }
            struct genlmsghdr *reply_generic = NLMSG_DATA(reply);
            int remaining = (int)reply->nlmsg_len - NLMSG_LENGTH(GENL_HDRLEN);
            struct nlattr *attr = (struct nlattr *)((char *)reply_generic + GENL_HDRLEN);
            for (; remaining >= NLA_HDRLEN && attr->nla_len >= NLA_HDRLEN &&
                   attr->nla_len <= remaining;
                 remaining -= NLA_ALIGN(attr->nla_len),
                 attr = (struct nlattr *)((char *)attr + NLA_ALIGN(attr->nla_len))) {
                if ((attr->nla_type & NLA_TYPE_MASK) == NL80211_ATTR_BSS) {
                    print_bss(attr);
                    ++count;
                }
            }
        }
    }
}

int main(int argc, char **argv) {
    const char *interface_name = argc > 1 ? argv[1] : "wlan0";
    unsigned int ifindex = if_nametoindex(interface_name);
    if (!ifindex) {
        fprintf(stderr, "%s: interface not found\n", interface_name);
        return 1;
    }

    int fd = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_GENERIC);
    if (fd < 0) {
        perror("netlink socket");
        return 1;
    }
    struct sockaddr_nl local = {
        .nl_family = AF_NETLINK,
        .nl_pid = (uint32_t)getpid(),
    };
    if (bind(fd, (struct sockaddr *)&local, sizeof(local)) < 0) {
        perror("netlink bind");
        close(fd);
        return 1;
    }

    int family = resolve_family(fd, "nl80211");
    if (family < 0) {
        perror("resolve nl80211");
        close(fd);
        return 1;
    }
    if (trigger_scan(fd, family, ifindex) < 0 && errno != EBUSY) {
        perror("trigger scan");
        close(fd);
        return 1;
    }
    fprintf(stderr, "Scanning on %s...\n", interface_name);
    sleep(8);
    int result = dump_scan(fd, family, ifindex);
    if (result < 0) {
        perror("read scan results");
    }
    close(fd);
    return result < 0 ? 1 : 0;
}
