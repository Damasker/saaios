/* APP-05: same manifest-exec-shim pattern as apps/kirigami-demo/launch.c
 * (S08 Change 7) -- MANIFEST_SCHEMA_V1 only supports one exec path, so
 * this static wrapper sets the env vars pcmanfm-qt's Qt5/QtWidgets
 * runtime needs (QT_PLUGIN_PATH, XKB_CONFIG_ROOT -- same two ADR-026
 * established as necessary for a Qt5 Wayland client on this hardware;
 * no QML2_IMPORT_PATH here, pcmanfm-qt is QtWidgets, not QtQuick) and
 * execs the real pcmanfm-qt binary with no arguments -- it opens
 * $HOME by default, which is set to this app's own sandboxed
 * SAAIOS_DATA_DIR below, the only directory this app's mount
 * namespace actually reveals (services/saai-appd/src/sandbox.rs).
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
    setenv_joined("FONTCONFIG_PATH", cwd, "etc/fonts");
    setenv_joined("FONTCONFIG_FILE", cwd, "etc/fonts/fonts.conf");
    /* Do not set QT_IM_MODULE to empty: that disables Wayland
     * text-input-v2 (ADR-323). The ibus plugin is deleted from the
     * package so QPA can own IM. */
    unsetenv("QT_IM_MODULE");
    setenv("QT_QPA_PLATFORMTHEME", "", 1);

    const char *data_dir = getenv("SAAIOS_DATA_DIR");
    if (data_dir != NULL && data_dir[0] != '\0') {
        setenv("HOME", data_dir, 1);
    }

    char exec_path[PATH_MAX];
    snprintf(exec_path, sizeof(exec_path), "%s/bin/pcmanfm-qt", cwd);

    char *args[] = {exec_path, NULL};
    execv(exec_path, args);
    _exit(127);
}
