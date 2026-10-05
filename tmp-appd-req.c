/* One-shot JSON line to /run/saaios/appd.sock. Throwaway APP-06 helper. */
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

int main(int argc, char **argv) {
    const char *path = "/run/saaios/appd.sock";
    const char *msg;
    if (argc < 2) {
        fprintf(stderr, "usage: appd-req '<json>'\n");
        return 2;
    }
    msg = argv[1];

    int fd = socket(AF_UNIX, SOCK_STREAM, 0);
    if (fd < 0) {
        perror("socket");
        return 1;
    }
    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    strncpy(addr.sun_path, path, sizeof(addr.sun_path) - 1);
    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("connect");
        return 1;
    }
    size_t len = strlen(msg);
    if (write(fd, msg, len) != (ssize_t)len || write(fd, "\n", 1) != 1) {
        perror("write");
        return 1;
    }
    shutdown(fd, SHUT_WR);
    char buf[4096];
    ssize_t n;
    while ((n = read(fd, buf, sizeof(buf))) > 0) {
        if (write(STDOUT_FILENO, buf, (size_t)n) < 0) {
            break;
        }
    }
    return 0;
}
