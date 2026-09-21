/* APP-06: same manifest-exec-shim as apps/pcmanfm-demo/launch.c.
 * Falkon is Qt6 Widgets + QtWebEngine. QT_PLUGIN_PATH and
 * XKB_CONFIG_ROOT follow ADR-026. WebEngine needs the helper
 * process and packed Chromium resources (ADR-281). The appd
 * mount namespace already sandboxes the app; Chromium's nested
 * sandbox is disabled for the first hello-frame.
 */
#define _GNU_SOURCE
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
    setenv("QTWEBENGINE_DISABLE_SANDBOX", "1", 1);
    setenv("QT_QPA_PLATFORMTHEME", "", 1);
    /* ADR-306 host qemu needed these so QtWebEngine does not wait on
     * EGL/Vulkan and never attach a wl_shm buffer. Same on panther:
     * Mali is not the scanout path (ADR-024). */
    setenv(
        "QTWEBENGINE_CHROMIUM_FLAGS",
        "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled "
        "--allow-file-access-from-files",
        1);
    setenv("LIBGL_ALWAYS_SOFTWARE", "1", 1);
    setenv("QT_QUICK_BACKEND", "software", 1);
    setenv("QSG_RENDER_LOOP", "basic", 1);

    const char *data_dir = getenv("SAAIOS_DATA_DIR");
    if (data_dir != NULL && data_dir[0] != '\0') {
        setenv("HOME", data_dir, 1);
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
