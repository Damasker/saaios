# Pixel 7 (panther) graphics

## Production renderer

`os/targets/panther/src/drm-splash.c` remains the boot/recovery UI and current
phone shell. It owns Exynos DRM/KMS, uses one XRGB8888 dumb buffer, renders
with the CPU and packs logical RGB into the panel's verified BGRX byte order.

The polished renderer adds:

- shared color, typography, radius and state tokens;
- one visual language across lock, root sections, system pages and keyboard;
- cached static background and rounded-corner coverage masks;
- event-driven invalidation instead of an unconditional one-second redraw;
- ordinary frame publication without repeated `SETCRTC`;
- mode-set only for initial activation and screen wake;
- render/publish timing counters in `/run/drm-splash.log`;
- deterministic host previews for 12 UI states.

## Host visual regression

```sh
bash os/targets/panther/tests/render-preview.sh
```

The test builds the production C source as a host executable, fixes wall time,
renders all states twice and compares PPM output byte-for-byte. It also catches
compiler warnings with `-Werror`.

During implementation on the WSL host, the 540x1200 twelve-screen run reported
roughly 4.1–5.7 ms average render time after warm-up. This is a regression
signal only: it is **not** a Pixel 7 latency or power measurement.

To inspect frames manually:

```sh
SAAIOS_FONT_DIR="$PWD/os/targets/panther/assets/fonts" \
  /tmp/drm-splash-preview --preview /tmp/saaios-ui
```

## Device acceptance

Cross-compile the renderer, install it in a test image with the previous image
available for rollback, then copy and run:

```sh
sh /data/saaios/graphics-device-check.sh
```

The automated script verifies DRM/input nodes, selected 1080x2400x60 mode,
process health and absence of known mode-set/input failures. A human must still
verify BGRX colors, clipping, touch targets, lock isolation, tearing and both
power-button and touch wake. Results are not recorded as complete until that
physical pass is performed.

## Presentation behavior

The main loop hashes relevant service files, battery, brightness, Wi-Fi address
and current minute. A static screen therefore does no redraw work. A changed
service state causes one full redraw; direct input actions render immediately.
The renderer logs aggregate `render_avg_us` and `publish_avg_us` every 60
published frames without logging prompt contents.

The current renderer intentionally does not claim page-flip, atomic modeset or
GPU acceleration. Those paths require device evidence and retain this dumb
buffer implementation as fallback.

## Next platform

Wayland separates `saai-displayd` from `saai-shell`. Its first mandatory buffer
transport is `wl_shm`; GPU and dmabuf policy are tracked in
[gpu.md](gpu.md). The recovery renderer does not become a Wayland compositor.
