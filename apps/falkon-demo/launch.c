/* APP-06: same manifest-exec-shim as apps/pcmanfm-demo/launch.c.
 * Falkon is Qt6 Widgets + QtWebEngine. QT_PLUGIN_PATH and
 * XKB_CONFIG_ROOT follow ADR-026. WebEngine needs the helper
 * process and packed Chromium resources (ADR-281). The appd
 * mount namespace already sandboxes the app; Chromium's nested
 * sandbox is disabled for the first hello-frame.
 *
 * ADR-321: serve share/hello.html on 127.0.0.1 so QtWebEngine
 * navigates HTTP inside empty NEWNET. Loopback is not NetInternet.
 */
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <fcntl.h>
#include <limits.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

#define LOOPBACK_PORT 8765

static void setenv_joined(const char *name, const char *cwd, const char *suffix) {
    char buf[PATH_MAX];
    snprintf(buf, sizeof(buf), "%s/%s", cwd, suffix);
    setenv(name, buf, 1);
}

static void write_all(int fd, const char *buf, size_t n) {
    while (n > 0) {
        ssize_t w = write(fd, buf, n);
        if (w <= 0) {
            return;
        }
        buf += (size_t)w;
        n -= (size_t)w;
    }
}

static void serve_hello(int cfd, const char *html_path) {
    char req[1024];
    ssize_t n = read(cfd, req, sizeof(req) - 1);
    if (n <= 0) {
        return;
    }
    req[n] = '\0';

    struct stat st;
    if (stat(html_path, &st) != 0 || st.st_size > (1 << 20)) {
        const char *nf = "HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        write_all(cfd, nf, strlen(nf));
        return;
    }

    int hfd = open(html_path, O_RDONLY);
    if (hfd < 0) {
        const char *nf = "HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        write_all(cfd, nf, strlen(nf));
        return;
    }

    char hdr[160];
    int hdr_len = snprintf(
        hdr,
        sizeof(hdr),
        "HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n"
        "Content-Length: %lld\r\nConnection: close\r\n\r\n",
        (long long)st.st_size);
    write_all(cfd, hdr, (size_t)hdr_len);

    char buf[4096];
    ssize_t r;
    while ((r = read(hfd, buf, sizeof(buf))) > 0) {
        write_all(cfd, buf, (size_t)r);
    }
    close(hfd);
}

static int listen_loopback(void) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return -1;
    }
    int one = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &one, sizeof(one));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(LOOPBACK_PORT);
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(fd);
        return -1;
    }
    if (listen(fd, 8) < 0) {
        close(fd);
        return -1;
    }
    return fd;
}

static void serve_loop(int lfd, const char *html_path) {
    for (;;) {
        int cfd = accept(lfd, NULL, NULL);
        if (cfd < 0) {
            continue;
        }
        serve_hello(cfd, html_path);
        close(cfd);
    }
}

int main(void) {
    char cwd[PATH_MAX];
    if (!getcwd(cwd, sizeof(cwd))) {
        _exit(126);
    }

    setenv_joined("QT_PLUGIN_PATH", cwd, "plugins");
    setenv_joined("XKB_CONFIG_ROOT", cwd, "share/X11/xkb");
    setenv_joined("QTWEBENGINEPROCESS_PATH", cwd, "libexec/QtWebEngineProcess");
    setenv_joined("QTWEBENGINE_RESOURCES_PATH", cwd, "share/qt6/resources");
    setenv_joined("QTWEBENGINE_LOCALES_PATH", cwd, "share/qt6/translations/qtwebengine_locales");
    setenv_joined("FONTCONFIG_PATH", cwd, "etc/fonts");
    setenv_joined("FONTCONFIG_FILE", cwd, "etc/fonts/fonts.conf");
    setenv("QTWEBENGINE_DISABLE_SANDBOX", "1", 1);
    setenv("QT_QPA_PLATFORMTHEME", "", 1);
    /* ADR-306/312: drop Wayland EGL so QBackingStore attaches wl_shm.
     * `--use-gl=disabled` then left QtWebEngine with nothing to rasterize
     * into that store (white hello-frame, ADR-313). Keep GPU off; let
     * Chromium software-composite. */
    setenv(
        "QTWEBENGINE_CHROMIUM_FLAGS",
        "--no-sandbox --disable-gpu --disable-gpu-compositing "
        "--allow-file-access-from-files --enable-logging --log-level=0",
        1);
    setenv_joined("LIBGL_DRIVERS_PATH", cwd, "lib/dri");
    setenv("LIBGL_ALWAYS_SOFTWARE", "1", 1);
    setenv("GALLIUM_DRIVER", "llvmpipe", 1);
    setenv("MESA_LOADER_DRIVER_OVERRIDE", "swrast", 1);
    setenv("LIBGL_DEBUG", "verbose", 1);
    setenv("QT_OPENGL", "software", 1);
    setenv("QT_QUICK_BACKEND", "software", 1);
    setenv("QSG_RENDER_LOOP", "basic", 1);

    const char *data_dir = getenv("SAAIOS_DATA_DIR");
    if (data_dir != NULL && data_dir[0] != '\0') {
        setenv("HOME", data_dir, 1);
        char logpath[PATH_MAX];
        snprintf(logpath, sizeof(logpath), "%s/webengine.log", data_dir);
        int logfd = open(logpath, O_WRONLY | O_CREAT | O_TRUNC, 0644);
        if (logfd >= 0) {
            dup2(logfd, STDERR_FILENO);
            dup2(logfd, STDOUT_FILENO);
            if (logfd > STDERR_FILENO) {
                close(logfd);
            }
        }
    }

    char html_path[PATH_MAX];
    snprintf(html_path, sizeof(html_path), "%s/share/hello.html", cwd);
    int lfd = listen_loopback();
    if (lfd < 0) {
        _exit(125);
    }
    pid_t child = fork();
    if (child < 0) {
        close(lfd);
        _exit(125);
    }
    if (child == 0) {
        prctl(PR_SET_PDEATHSIG, SIGKILL);
        if (getppid() == 1) {
            _exit(0);
        }
        serve_loop(lfd, html_path);
        _exit(0);
    }
    close(lfd);

    char exec_path[PATH_MAX];
    char url[64];
    snprintf(exec_path, sizeof(exec_path), "%s/bin/falkon", cwd);
    /* HTTP on loopback, no NetInternet. Private browsing skips the
     * default session that restores https://www.falkon.org. */
    snprintf(url, sizeof(url), "http://127.0.0.1:%d/hello.html", LOOPBACK_PORT);

    char *args[] = {exec_path, "--private-browsing", url, NULL};
    execv(exec_path, args);
    _exit(127);
}
