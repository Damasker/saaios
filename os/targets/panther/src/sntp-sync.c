#define _GNU_SOURCE

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <time.h>
#include <unistd.h>

#define NTP_UNIX_EPOCH 2208988800ULL

static uint32_t read_be32(const uint8_t *bytes) {
    return ((uint32_t)bytes[0] << 24) |
           ((uint32_t)bytes[1] << 16) |
           ((uint32_t)bytes[2] << 8) |
           (uint32_t)bytes[3];
}

static void save_time(time_t seconds) {
    int fd = open("/metadata/saaios/last-time",
                  O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0600);
    if (fd < 0) {
        return;
    }
    char value[32];
    int length = snprintf(value, sizeof(value), "%lld\n",
                          (long long)seconds);
    if (length > 0) {
        (void)write(fd, value, (size_t)length);
        (void)fsync(fd);
    }
    close(fd);
}

int main(int argc, char **argv) {
    const char *host = argc > 1 ? argv[1] : "time.google.com";
    struct addrinfo hints = {0};
    struct addrinfo *addresses = NULL;
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_DGRAM;
    int result = getaddrinfo(host, "123", &hints, &addresses);
    if (result != 0) {
        fprintf(stderr, "sntp-sync: resolve %s failed: %s\n",
                host, gai_strerror(result));
        return 2;
    }

    int fd = socket(addresses->ai_family,
                    addresses->ai_socktype | SOCK_CLOEXEC,
                    addresses->ai_protocol);
    if (fd < 0) {
        fprintf(stderr, "sntp-sync: socket failed: %s\n", strerror(errno));
        freeaddrinfo(addresses);
        return 3;
    }
    struct timeval timeout = {.tv_sec = 5, .tv_usec = 0};
    (void)setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));

    uint8_t packet[48] = {0};
    packet[0] = 0x23;
    ssize_t sent = sendto(fd, packet, sizeof(packet), 0,
                          addresses->ai_addr, addresses->ai_addrlen);
    freeaddrinfo(addresses);
    if (sent != (ssize_t)sizeof(packet)) {
        fprintf(stderr, "sntp-sync: request failed: %s\n", strerror(errno));
        close(fd);
        return 4;
    }

    ssize_t received = recv(fd, packet, sizeof(packet), 0);
    close(fd);
    if (received < (ssize_t)sizeof(packet)) {
        fprintf(stderr, "sntp-sync: response failed: %s\n", strerror(errno));
        return 5;
    }
    unsigned int mode = packet[0] & 7U;
    if ((mode != 4U && mode != 5U) || packet[1] == 0 || packet[1] > 15) {
        fprintf(stderr, "sntp-sync: invalid server response\n");
        return 6;
    }

    uint64_t ntp_seconds = read_be32(packet + 40);
    uint64_t fraction = read_be32(packet + 44);
    if (ntp_seconds <= NTP_UNIX_EPOCH) {
        fprintf(stderr, "sntp-sync: invalid timestamp\n");
        return 7;
    }
    struct timespec now = {
        .tv_sec = (time_t)(ntp_seconds - NTP_UNIX_EPOCH),
        .tv_nsec = (long)((fraction * 1000000000ULL) >> 32),
    };
    if (clock_settime(CLOCK_REALTIME, &now) < 0) {
        fprintf(stderr, "sntp-sync: clock_settime failed: %s\n",
                strerror(errno));
        return 8;
    }
    save_time(now.tv_sec);
    printf("sntp-sync: %lld.%09ld UTC from %s\n",
           (long long)now.tv_sec, now.tv_nsec, host);
    return 0;
}
