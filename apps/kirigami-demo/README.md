# Kirigami demo package

S08 Change 7's first real Kirigami2/QtQuick application, installable and
launchable through the ordinary `saai-appd` lifecycle (S05) like any other
package -- physical acceptance (install→launch→switch→stop→remove) confirmed
on a real Pixel 7 (ADR-026/ADR-027).

Unlike `demo-surface`, this app's runtime is not built from this repository's
own Rust source: `manifest.toml` and `app.qml` are the only things this repo
actually owns. `os/targets/panther/build-kirigami-demo-package.sh` fetches
Alpine Linux's prebuilt aarch64 `kirigami2` package (ADR-021's decision --
Alpine musl packages over from-source) and assembles it, together with a tiny
static `launch.c` wrapper (compiled by the same script), into an installable
package directory. `saai-appd install` can be pointed at the result the same
way as `demo-surface`.

`launch.c` exists only because `AppManifest` (ADR-020, MANIFEST_SCHEMA_V1)
has a single `exec` path and no argv/env fields -- it sets the three env vars
`qmlscene-qt5` needs beyond what `AppSupervisor::spawn()` already provides
(`QML2_IMPORT_PATH`, `QT_PLUGIN_PATH`, `XKB_CONFIG_ROOT`) and execs the real
`qmlscene-qt5` binary with `app.qml` as its one argument -- no shell involved,
since the sandbox reveals no `/bin/sh` for apps to depend on.
