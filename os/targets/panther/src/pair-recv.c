/* SSH pairing gate -- the policy layer this project's own critique
 * session flagged as missing before any remote-exec channel should
 * exist (see docs/adr for "remote access policy"). Modeled on
 * Android's adb: a brand new client's public key is USELESS until a
 * human taps "Allow" on the device screen; after that, normal SSH
 * key auth handles every later connection with no further prompts.
 *
 * Wire shape from the network client: two text lines, "PAIR
 * <client-name>\n" then the raw SSH public key line (whatever
 * ssh-keygen -y produces, e.g. "ssh-ed25519 AAAA... comment\n").
 *
 * This process never decides yes/no itself -- it is a dumb relay to
 * saai-shell's own Unix socket at /run/saaios/remote-pair.sock,
 * which is what actually shows the consent screen and knows the
 * user's decision. If that socket is unreachable, or the master
 * "Remote access" toggle is off (checked via a marker file saai-
 * shell itself writes/removes when the setting changes), a request
 * is refused immediately with no prompt shown -- fail closed, not
 * fail open.
 *
 * On approval, this process (not saai-shell) appends the key to
 * /data/saaios/var/dropbear/authorized_keys -- saai-shell only
 * returns yes/no, filesystem specifics of "how dropbear finds
 * trusted keys" stay this daemon's own concern, the same "don't
 * reach into another component's storage format" boundary ADR-030
 * already established between saai-taskd and saaios-runtime.
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
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#define PORT 7779
#define ENABLED_MARKER "/data/saaios/var/remote-access-enabled"
#define AUTHORIZED_KEYS "/data/saaios/var/dropbear/.ssh/authorized_keys"
#define PAIR_SOCKET_PATH "/run/saaios/remote-pair.sock"
#define MAX_LINE 1024

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

static void reply(int client, const char *message) {
    (void)write_full(client, message, strlen(message));
}

/* Escapes '"' and '\\' only -- the two characters that could break
 * out of a JSON string in a client-supplied name/key. Neither an
 * SSH public key line nor a reasonable client name ever legitimately
 * contains either, so this is a safety net, not a real feature. */
static void json_escape_into(char *dest, size_t dest_size, const char *src) {
    size_t out = 0;
    for (const char *p = src; *p && out + 2 < dest_size; p++) {
        if (*p == '"' || *p == '\\') {
            dest[out++] = '\\';
        }
        dest[out++] = *p;
    }
    dest[out] = '\0';
}

/* Asks saai-shell whether this pairing request is approved. Returns
 * 1 (approved), 0 (declined or unreachable/disabled -- fail closed
 * either way from this function's caller's point of view). */
static int ask_shell(const char *client_name, const char *public_key) {
    if (access(ENABLED_MARKER, F_OK) != 0) {
        return 0;
    }

    int sock = socket(AF_UNIX, SOCK_STREAM, 0);
    if (sock < 0) {
        return 0;
    }
    struct sockaddr_un address = {0};
    address.sun_family = AF_UNIX;
    snprintf(address.sun_path, sizeof(address.sun_path), "%s", PAIR_SOCKET_PATH);
    if (connect(sock, (struct sockaddr *)&address, sizeof(address)) < 0) {
        close(sock);
        return 0;
    }

    char escaped_name[600];
    char escaped_key[900];
    json_escape_into(escaped_name, sizeof(escaped_name), client_name);
    json_escape_into(escaped_key, sizeof(escaped_key), public_key);

    char request[2048];
    int written = snprintf(
        request, sizeof(request),
        "{\"command\":\"pair_request\",\"schema\":1,\"request_id\":\"pair:%ld-%d\","
        "\"client_name\":\"%s\",\"public_key\":\"%s\"}\n",
        (long)time(NULL), getpid(), escaped_name, escaped_key);
    if (written < 0 || (size_t)written >= sizeof(request)) {
        close(sock);
        return 0;
    }
    if (write_full(sock, request, (size_t)written) < 0) {
        close(sock);
        return 0;
    }

    /* A human has to physically look at the screen and tap -- give
     * this far longer than any of this project's other daemon-to-
     * daemon round-trips. */
    struct timeval read_timeout = {.tv_sec = 120, .tv_usec = 0};
    setsockopt(sock, SOL_SOCKET, SO_RCVTIMEO, &read_timeout, sizeof(read_timeout));

    char response[256];
    int approved = 0;
    if (read_header_line(sock, response, sizeof(response)) == 0) {
        approved = strstr(response, "\"approved\":true") != NULL;
    }
    close(sock);
    return approved;
}

static int append_authorized_key(const char *client_name, const char *public_key) {
    char key_type[64];
    char key_base64[900];
    if (sscanf(public_key, "%63s %899s", key_type, key_base64) != 2) {
        return -1;
    }
    (void)mkdir("/data/saaios/var/dropbear", 0755);
    /* dropbear resolves authorized_keys from the authenticating
     * user's own $HOME/.ssh/, not an arbitrary configured path --
     * this must stay in sync with native-init.c's /etc/passwd entry
     * for root and its HOME. */
    (void)mkdir("/data/saaios/var/dropbear/.ssh", 0700);
    int fd = open(AUTHORIZED_KEYS, O_WRONLY | O_CREAT | O_APPEND | O_CLOEXEC, 0600);
    if (fd < 0) {
        return -1;
    }
    char line[1024];
    int length = snprintf(line, sizeof(line), "%s %s %s\n", key_type, key_base64, client_name);
    int result = (length > 0 && write_full(fd, line, (size_t)length) == 0) ? 0 : -1;
    close(fd);
    return result;
}

static void handle_client(int client) {
    char header[MAX_LINE];
    if (read_header_line(client, header, sizeof(header)) < 0) {
        return;
    }
    char verb[8] = {0};
    char client_name[600] = {0};
    if (sscanf(header, "%7s %599[^\n]", verb, client_name) != 2 || strcmp(verb, "PAIR") != 0) {
        reply(client, "ERR bad header, expected: PAIR <client-name>\n");
        return;
    }

    char public_key[MAX_LINE];
    if (read_header_line(client, public_key, sizeof(public_key)) < 0) {
        return;
    }
    if (strncmp(public_key, "ssh-", 4) != 0) {
        reply(client, "ERR second line must be an SSH public key\n");
        return;
    }

    if (!ask_shell(client_name, public_key)) {
        reply(client, "ERR denied (remote access off, or declined on-device)\n");
        return;
    }
    if (append_authorized_key(client_name, public_key) < 0) {
        reply(client, "ERR approved, but failed to save the key\n");
        return;
    }
    printf("pair-recv: paired client %s\n", client_name);
    fflush(stdout);
    reply(client, "OK\n");
}

int main(void) {
    int server = socket(AF_INET, SOCK_STREAM, 0);
    if (server < 0) {
        perror("pair-recv: socket");
        return 1;
    }
    int yes = 1;
    (void)setsockopt(server, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));

    struct sockaddr_in address = {0};
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = INADDR_ANY;
    address.sin_port = htons(PORT);
    if (bind(server, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("pair-recv: bind");
        return 1;
    }
    if (listen(server, 4) < 0) {
        perror("pair-recv: listen");
        return 1;
    }

    printf("pair-recv: listening on :%d\n", PORT);
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
