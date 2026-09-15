/* Minimal persistent-listener file receiver -- the missing piece that
 * finally lets a file reach this device without a full fastboot flash
 * of init_boot. Every prior deployment this whole project (saai-shell
 * updates across the S14-S25 series, org.saaios.mahjong's own package)
 * had exactly one channel: bake it into the 8MB init_boot ramdisk and
 * reflash. This is the bootstrap that ends that -- once this one
 * binary is on the device (via one last reflash), every subsequent
 * file arrives over the network instead.
 *
 * Text header, binary body: a client sends "PUT <relative-path>
 * <byte-count>\n" followed by exactly that many raw bytes. Chosen over
 * a binary length-prefixed header specifically so a plain shell client
 * (bash's /dev/tcp, no extra tooling) can drive it without constructing
 * raw multi-byte integers by hand.
 *
 * Every write lands under ALLOWED_PREFIX (a real, persistent partition
 * -- /data/saaios, not the ramdisk) -- the one deliberate safety rule:
 * no ".." component, no leading "/", so a client can never write
 * outside that tree even by accident. This is a bootstrap dev tool for
 * a single-developer device with no other user, not a hardened
 * service -- no auth beyond "reachable on this LAN", matching this
 * project's existing wpa_supplicant/appd socket trust model (anyone
 * who can reach the device's own sockets already has this much
 * access).
 *
 * S31: "GET <relative-path>\n" is the other direction -- pulling a
 * file (a log, a crash report, anything under /data/saaios/apps'
 * data dirs) off the device instead of pushing one on. Deliberately
 * NOT confined to ALLOWED_PREFIX the way PUT's destinations are:
 * that prefix is upload staging, reading it back would just return
 * the same bytes a client already has. GET resolves its path against
 * the real root (same no-".."/no-leading-"/" rule as PUT, kept for
 * header-parsing symmetry, not a meaningful boundary here since the
 * whole filesystem is already the read domain) -- the trust model is
 * the same "reachable on this LAN" one already covering every other
 * socket this project exposes, not a new, wider one. Reply on success
 * is "OK <byte-count>\n" followed by exactly that many raw bytes;
 * on failure, a single "ERR ...\n" line and nothing else.
 */
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define PORT 7777
#define ALLOWED_PREFIX "/data/saaios/incoming/"
#define MAX_RELATIVE_PATH 480
#define MAX_FULL_PATH (MAX_RELATIVE_PATH + sizeof(ALLOWED_PREFIX) + 8)
#define MAX_HEADER_LINE 600
#define MAX_FILE_BYTES (128u * 1024 * 1024)

static int read_full(int fd, void *buf, size_t len) {
    size_t got = 0;
    while (got < len) {
        ssize_t n = read(fd, (char *)buf + got, len - got);
        if (n <= 0) {
            return -1;
        }
        got += (size_t)n;
    }
    return 0;
}

static int write_full(int fd, const void *buf, size_t len) {
    size_t sent = 0;
    while (sent < len) {
        ssize_t n = write(fd, (const char *)buf + sent, len - sent);
        if (n <= 0) {
            return -1;
        }
        sent += (size_t)n;
    }
    return 0;
}

static int read_header_line(int fd, char *buf, size_t max) {
    size_t i = 0;
    while (i + 1 < max) {
        char c;
        ssize_t n = read(fd, &c, 1);
        if (n <= 0) {
            return -1;
        }
        if (c == '\n') {
            buf[i] = '\0';
            return 0;
        }
        buf[i++] = c;
    }
    return -1;
}

/* Creates every ancestor directory of `path` (not `path` itself --
 * the caller always passes a file path, never a bare directory). */
static void make_parent_dirs(const char *path) {
    char work[MAX_FULL_PATH];
    snprintf(work, sizeof(work), "%s", path);
    for (char *slash = work + 1; *slash; slash++) {
        if (*slash == '/') {
            *slash = '\0';
            (void)mkdir(work, 0755);
            *slash = '/';
        }
    }
}

static void reply(int client, const char *message) {
    (void)write_full(client, message, strlen(message));
}

static void handle_get(int client, const char *relative) {
    if (relative[0] == '/' || strstr(relative, "..") != NULL) {
        reply(client, "ERR path must be relative with no .. component\n");
        return;
    }

    char full_path[MAX_RELATIVE_PATH + 8];
    snprintf(full_path, sizeof(full_path), "/%s", relative);

    struct stat info;
    if (stat(full_path, &info) < 0 || !S_ISREG(info.st_mode)) {
        reply(client, "ERR file not found or not a regular file\n");
        return;
    }
    if ((unsigned long long)info.st_size > MAX_FILE_BYTES) {
        reply(client, "ERR file exceeds the 128MiB limit\n");
        return;
    }

    int in = open(full_path, O_RDONLY | O_CLOEXEC);
    if (in < 0) {
        reply(client, "ERR could not open source file\n");
        return;
    }

    char header[64];
    int header_len = snprintf(header, sizeof(header), "OK %lld\n", (long long)info.st_size);
    if (header_len < 0 || write_full(client, header, (size_t)header_len) < 0) {
        close(in);
        return;
    }

    char buffer[65536];
    long long remaining = (long long)info.st_size;
    while (remaining > 0) {
        size_t chunk = remaining < (long long)sizeof(buffer) ? (size_t)remaining : sizeof(buffer);
        ssize_t n = read(in, buffer, chunk);
        if (n <= 0 || write_full(client, buffer, (size_t)n) < 0) {
            break;
        }
        remaining -= n;
    }
    close(in);
    printf("file-recv: read %s (%lld bytes)\n", full_path, (long long)info.st_size);
    fflush(stdout);
}

static void handle_client(int client) {
    char header[MAX_HEADER_LINE];
    if (read_header_line(client, header, sizeof(header)) < 0) {
        return;
    }

    char verb[8] = {0};
    char relative[MAX_RELATIVE_PATH] = {0};
    unsigned long long content_length = 0;
    int put_fields = sscanf(header, "%7s %479s %llu", verb, relative, &content_length);
    if (put_fields != 3 || strcmp(verb, "PUT") != 0) {
        if (sscanf(header, "%7s %479s", verb, relative) == 2 && strcmp(verb, "GET") == 0) {
            handle_get(client, relative);
            return;
        }
        reply(client, "ERR bad header, expected: PUT <path> <length> or GET <path>\n");
        return;
    }
    if (relative[0] == '/' || strstr(relative, "..") != NULL) {
        reply(client, "ERR path must be relative with no .. component\n");
        return;
    }
    if (content_length > MAX_FILE_BYTES) {
        reply(client, "ERR file exceeds the 128MiB limit\n");
        return;
    }

    char full_path[MAX_FULL_PATH];
    snprintf(full_path, sizeof(full_path), "%s%s", ALLOWED_PREFIX, relative);
    make_parent_dirs(full_path);

    int out = open(full_path, O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0644);
    if (out < 0) {
        reply(client, "ERR could not open destination file\n");
        return;
    }

    char buffer[65536];
    unsigned long long remaining = content_length;
    int failed = 0;
    while (remaining > 0) {
        size_t chunk = remaining < sizeof(buffer) ? (size_t)remaining : sizeof(buffer);
        if (read_full(client, buffer, chunk) < 0 || write_full(out, buffer, chunk) < 0) {
            failed = 1;
            break;
        }
        remaining -= chunk;
    }
    close(out);
    if (failed) {
        (void)unlink(full_path);
        reply(client, "ERR transfer interrupted\n");
        return;
    }
    /* 0755 unconditionally: harmless on a non-executable file, and
     * saves a second round-trip for the common case of pushing a
     * binary that needs to be runnable right away. */
    (void)chmod(full_path, 0755);
    printf("file-recv: wrote %s (%llu bytes)\n", full_path, content_length);
    fflush(stdout);
    reply(client, "OK\n");
}

int main(void) {
    int server = socket(AF_INET, SOCK_STREAM, 0);
    if (server < 0) {
        perror("file-recv: socket");
        return 1;
    }
    int yes = 1;
    (void)setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));

    struct sockaddr_in address = {0};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY;
    address.sin_port = htons(PORT);
    if (bind(server, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("file-recv: bind");
        return 1;
    }
    if (listen(server, 4) < 0) {
        perror("file-recv: listen");
        return 1;
    }

    (void)mkdir(ALLOWED_PREFIX, 0755);
    printf("file-recv: listening on :%d, writing under %s\n", PORT, ALLOWED_PREFIX);
    fflush(stdout);

    for (;;) {
        int client = accept(server, NULL, NULL);
        if (client < 0) {
            continue;
        }
        int one = 1;
        (void)setsockopt(client, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
        handle_client(client);
        close(client);
    }
}
