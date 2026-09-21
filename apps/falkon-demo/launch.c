/* APP-06: same manifest-exec-shim as apps/pcmanfm-demo/launch.c.
 * Falkon is Qt6 Widgets + QtWebEngine. QT_PLUGIN_PATH and
 * XKB_CONFIG_ROOT follow ADR-026. WebEngine needs the helper
 * process and packed Chromium resources (ADR-281). The appd
 * mount namespace already sandboxes the app; Chromium's nested
 * sandbox is disabled for the first hello-frame.
 */
#define _GNU_SOURCE
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static void setenv_joined(const char *name, const char *cwd, const char *suffix) {
    char buf[PATH_MAX];
    snprintf(buf, sizeof(buf), "%s/%s", cwd, suffix);
    setenv(name, buf, 1);
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

    char exec_path[PATH_MAX];
    char url[PATH_MAX];
    snprintf(exec_path, sizeof(exec_path), "%s/bin/falkon", cwd);
    /* Local page, no NetInternet. Private browsing skips the default
     * session that restores https://www.falkon.org (CLONE_NEWNET has
     * only loopback after ADR-313). */
    snprintf(url, sizeof(url), "file://%s/share/hello.html", cwd);

    char *args[] = {exec_path, "--private-browsing", url, NULL};
    execv(exec_path, args);
    _exit(127);
}
