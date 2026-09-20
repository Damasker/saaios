# ADR-269: First browser candidate is Falkon on QtWebEngine, not Epiphany

## Статус

Принято, 2026-09-21. APP-06 package spike. Not a panther launch.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd or shell.

## Нумерация

После ADR-268 следующий свободный номер — **269**. Не S33.

## Контекст

APP-COMPAT asked to check a Qt browser under ARM64 musl before
putting Epiphany/WebKitGTK first. ADR-095 proved Alpine aarch64
packages exist (edge versions). The live sysroot is Alpine **v3.20**
(ADR-021), the same index that already launched Kirigami2 and
PCManFM-Qt. This spike names a first candidate from that index.

## Evidence (Alpine v3.20 community aarch64)

| Package | Version | Engine | Toolkit |
|---|---|---|---|
| `qt6-qtwebengine` | 6.6.3-r6 | Chromium | Qt6 (~197 MiB installed) |
| `falkon` | 24.02.2-r0 | Qt6WebEngineWidgets | Qt6 Widgets + KF6 |
| `angelfish` | 24.02.2-r0 | Qt6WebEngineQuick | Kirigami / Plasma Mobile |
| `epiphany` | 46.0-r0 | `libwebkitgtk-6.0` | **GTK4** |
| `webkit2gtk-4.1` | 2.44.1-r1 | WebKit GTK3 | engine only |
| `midori` | — | — | **not in v3.20** |

`falkon` and `angelfish` link `libc.musl-aarch64.so.1`. Musl is not
the question. Epiphany is GTK4, so it stays behind APP-02. There is
no v3.20 browser binary on `webkit2gtk-4.1`.

## Decision

1. **First candidate: Falkon.** Widgets, same class as PCManFM-Qt
   (APP-05). Not Angelfish: extra Plasma/KF6 QML on top of the same
   WebEngine. Not Epiphany until a GTK4 frame exists.
2. **The risk is WebEngine, not musl.** Chromium zygote, sandbox,
   GPU, ~200 MiB. `saai-appd`'s private dbus (ADR-098) may cover
   session bus; it does not make WebEngine scanout honest.
3. **GTK3 WebKit is an escape hatch, not a browser.** The engine
   package exists. A product UI on it would be a new port, not an
   apk. Firefox/Chromium stay last.
4. **No launch this slice.** Do not install Falkon on panther. Do
   not flash displayd. Next APP-06 vertical is a packaged Falkon
   hello-frame against the already-proven Qt Wayland path, after a
   displayd experiment is allowed.

## Consequences

- APP-06 is no longer "is there a Qt browser?" — there is, on the
  same v3.20 aarch64 musl index.
- Rollback: none. This records a candidate, it does not add chrome.

## Verification

Package pages: falkon, angelfish, qt6-qtwebengine, epiphany,
webkit2gtk-4.1 on pkgs.alpinelinux.org `v3.20/community/aarch64`.
No panther flash. Leave Сейчас.
