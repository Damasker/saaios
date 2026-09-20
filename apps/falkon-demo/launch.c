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

    const char *data_dir = getenv("SAAIOS_DATA_DIR");
    if (data_dir != NULL && data_dir[0] != '\0') {
        setenv("HOME", data_dir, 1);
    }

    char exec_path[PATH_MAX];
    snprintf(exec_path, sizeof(exec_path), "%s/bin/falkon", cwd);

    char *args[] = {exec_path, NULL};
    execv(exec_path, args);
    _exit(127);
}
