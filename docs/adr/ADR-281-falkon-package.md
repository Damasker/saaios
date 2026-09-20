# ADR-281: Falkon package recipe packs WebEngineProcess and resources

## Статус

Принято, 2026-09-21. APP-06 package tree. Not a panther launch.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd or
install via `saai-appd`.

## Нумерация

После ADR-280 следующий свободный номер — **281**. Не S33.

## Контекст

ADR-269 named Falkon on Alpine v3.20 aarch64 musl. The next
vertical is a packaged hello-frame on the proven Qt Wayland
path. PCManFM-Qt already has `build-pcmanfm-qt-package.sh`.
WebEngine is not just `usr/bin/falkon`: Chromium needs
`QtWebEngineProcess` and `icudtl.dat`. Nested Chromium sandbox
fights ADR-020's mount namespace.

## Decision

1. **Same recipe.** `os/targets/panther/build-falkon-package.sh`
   fetches v3.20 community aarch64 `falkon` + `qt6-qtwayland` +
   `qt6-qtwebengine` with apk-tools-static, like Kirigami/PCManFM.
2. **WebEngine files.** The package includes
   `libexec/QtWebEngineProcess` and `share/qt6/resources`
   (`qtwebengine_resources.pak`, `v8_context_snapshot.bin`). Alpine
   v3.20 WebEngine uses system ICU (`libicuuc.so.74`), not bundled
   `icudtl.dat`. `launch.c` sets `QTWEBENGINEPROCESS_PATH`,
   `QTWEBENGINE_RESOURCES_PATH`, `QTWEBENGINE_LOCALES_PATH`.
3. **No nested sandbox.** `QTWEBENGINE_DISABLE_SANDBOX=1`. appd
   sandbox stays. First frame does not invent a second jail.
4. **Empty capabilities.** Hello-frame is a window, not a
   network grant. `NetInternet` waits for a real browse.
5. **No panther install.** Do not `appd install`. Do not flash
   displayd. Launch is the next displayd-experiment week.

## Consequences

- `dist/panther/packages/org.saaios.demo.falkon` is a build
  artifact (`/dist/` is gitignored).
- EGL/WebEngine GPU is still ADR-024: software scanout or fail
  honestly on device.
- Rollback: drop `apps/falkon-demo` and the build script.

## Verification

Host: run `build-falkon-package.sh` on R620; `usr/bin/falkon`,
`QtWebEngineProcess`, `qtwebengine_resources.pak`,
`v8_context_snapshot.bin`, and `libicuuc.so.74` exist; `launch` is
a static aarch64 musl binary. No panther install.
