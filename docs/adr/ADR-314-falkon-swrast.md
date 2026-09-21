# ADR-314: Falkon WebEngine needs swrast; packing it does not paint yet

## Статус

Принято, 2026-09-21. APP-06 software GL. Not a browse. Not Visual
v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-313 следующий свободный номер — **314**. Не S33.

## Контекст

ADR-312 hashed a white `wl_shm` hello-frame. ADR-313 brought up
loopback so Chromium's network service can load `file://`. The
screencap stayed `#dddddd`. `--use-gl=disabled` was suspected of
leaving QtWebEngine with nothing to rasterize. Dropping it still
committed hashed shm (not `(no buffer)`) and still painted white.
The package had `libEGL`/`libGLESv2` but no DRI driver: Mesa
cannot form a software GL. QtWebEngine's view is a GL widget;
QBackingStore still blits an empty chrome surface.

## Decision

1. **Pack `mesa-dri-gallium` swrast + `libLLVM-17.so` + `libelf`.**
   `build-falkon-package.sh` `apk add mesa-dri-gallium`. Copy
   `libgallium_dri.so` into `lib/dri/` with `swrast_dri.so` /
   `kms_swrast_dri.so` symlinks. Do not copy panfrost as a scanout
   path (ADR-024). LLVM gzip is 54 MiB; PUT stays under file-recv
   128 MiB.
2. **`LIBGL_DRIVERS_PATH=$cwd/lib/dri`.** Also
   `GALLIUM_DRIVER=llvmpipe`, `MESA_LOADER_DRIVER_OVERRIDE=swrast`,
   `QT_OPENGL=software`. Chromium flags stay
   `--disable-gpu --disable-gpu-compositing` so the Wayland window
   keeps shm (EGL plugins still omitted).
3. **Not a painted page.** After install, renderers die as
   `[QtWebEngineProc]` with no `--type=renderer`. `--use-gl=egl`
   plus llvmpipe still hashed shm and still white. `hello.html`
   is in the mount ns. Visible Falkon chrome is a later slice.

## Consequences

- Next full package rebuild ships LLVM (~142 MiB unpacked).
- Rollback: omit `mesa-dri-gallium` from the recipe; delete
  `lib/dri` and `libLLVM-17.so` from the installed app.
- Do not flash displayd for this.

## Verification

Host: no new rust test; widgets hello-frame recipe unchanged.
Panther: `lib/dri/swrast_dri.so` → `libgallium_dri.so`,
`LIBGL_DRIVERS_PATH` in falkon environ, screencap white.
dest-no-lock kept. Stop returns Дом · Сейчас. Leave Сейчас.
