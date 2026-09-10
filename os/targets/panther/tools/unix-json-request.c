#define _GNU_SOURCE

#include <errno.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s SOCKET JSON\n", argv[0]);
        return 2;
    }
    if (strlen(argv[1]) >= sizeof(((struct sockaddr_un *)0)->sun_path)) {
        fputs("socket path is too long\n", stderr);
        return 2;
    }

    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0) {
        perror("socket");
        return 1;
    }
    struct sockaddr_un address = {.sun_family = AF_UNIX};
    strcpy(address.sun_path, argv[1]);
    socklen_t length = (socklen_t)(offsetof(struct sockaddr_un, sun_path) +
                                   strlen(address.sun_path) + 1);
    if (connect(fd, (struct sockaddr *)&address, length) < 0) {
        perror("connect");
        close(fd);
        return 1;
    }

    size_t request_length = strlen(argv[2]);
    if (write(fd, argv[2], request_length) != (ssize_t)request_length ||
        write(fd, "\n", 1) != 1) {
        perror("write");
        close(fd);
        return 1;
    }

    char byte;
    while (read(fd, &byte, 1) == 1) {
        if (write(STDOUT_FILENO, &byte, 1) != 1 || byte == '\n') {
            break;
        }
    }
    close(fd);
    return 0;
}
