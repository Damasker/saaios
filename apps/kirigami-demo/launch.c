/* S08 Change 7: saai-appd's manifest schema (ADR-020/MANIFEST_SCHEMA_V1)
 * only supports a single `exec` path, no argv/env fields -- this tiny
 * static wrapper is the manifest's exec target: it sets the few env
 * vars qmlscene-qt5 needs (QML2_IMPORT_PATH/QT_PLUGIN_PATH/
 * XKB_CONFIG_ROOT -- physically established in ADR-026; QT_QPA_PLATFORM
 * and LD_LIBRARY_PATH are already set unconditionally by
 * AppSupervisor::spawn() itself) and execs the real qmlscene-qt5
 * binary with this app's QML file as its one argument. No shell
 * involved -- the sandbox reveals no /bin/sh for apps to depend on.
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

    setenv_joined("QML2_IMPORT_PATH", cwd, "qml");
    setenv_joined("QT_PLUGIN_PATH", cwd, "plugins");
    setenv_joined("XKB_CONFIG_ROOT", cwd, "share/X11/xkb");

    const char *data_dir = getenv("SAAIOS_DATA_DIR");
    if (data_dir != NULL && data_dir[0] != '\0') {
        setenv("HOME", data_dir, 1);
    }

    char qml_path[PATH_MAX];
    snprintf(qml_path, sizeof(qml_path), "%s/app.qml", cwd);
    char exec_path[PATH_MAX];
    snprintf(exec_path, sizeof(exec_path), "%s/bin/qmlscene-qt5", cwd);

    char *args[] = {exec_path, qml_path, NULL};
    execv(exec_path, args);
    _exit(127);
}
