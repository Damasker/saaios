# Pixel 7 (panther) graphics acceleration

## Current state

The verified display path is Exynos DRM/KMS scanout at 1080x2400x60 with a
CPU-rendered XRGB8888 dumb buffer. `drm-splash` converts logical RGB to the
panel's verified BGRX byte layout. It remains the boot and recovery renderer.

No GPU kernel driver, render node, Mali CSF firmware, Mesa, GBM or EGL stack is
currently packaged by SaaiOS. Hardware acceleration is therefore **not
claimed**. Pixel 7 uses Mali-G710 MP7. Its upstream open-driver path is Panthor;
Panfrost is not a substitute.

G710 enablement was still a Panthor patch series in 2025 and adds firmware IDs
for `arm/mali/arch10.10/mali_csffw.bin` and `arch10.12`. The stock Pixel kernel
branch used by this target is the Android `pantah-6.1` family and is not
assumed to contain those later changes. A custom, tested kernel may be required;
loading a module built for another kernel is not an option.

## Read-only probe

`os/targets/panther/tools/gpu-probe.c` inventories the running system without
loading modules, becoming DRM master or writing power/sysfs state:

```sh
zig cc -target aarch64-linux-musl -static -Os -s \
  os/targets/panther/tools/gpu-probe.c -o /tmp/saaios-gpu-probe -ldl
/tmp/saaios-gpu-probe
```

Expected progression:

1. `NO_KERNEL_GPU` — no GPU driver or render node.
2. `KERNEL_VISIBLE` — a candidate driver/device is visible.
3. `RENDER_NODE_READY` — `/dev/dri/renderD*` exists.
4. `EGL_LOADER_READY` — render node plus an EGL loader; this still does not
   prove rendering.

The probe returning status 2 means that no render node is available. This is
an expected diagnostic result, not a reason to load an unverified module.

## Acceptance sequence

1. Inventory matching `CP2A.260705.006` images for Panthor/Mali modules,
   dependencies and `mali_csffw.bin`; record hashes and licenses.
2. Confirm the live DT node and deferred-probe state read-only.
3. On one bounded boot, manually load only the confirmed signed/upstream
   driver and verify a render node. Do not add it to PID 1.
4. Put Mesa/GBM/EGL under `/data/saaios`, run an off-screen smoke test, and
   record renderer/version, frame time, temperature and idle power.
5. Integrate `linux-dmabuf` into `saai-displayd` only after the `wl_shm`
   compositor is stable. Unsupported format/modifier, failed import or missing
   explicit sync must select `wl_shm`; it must not terminate the compositor.
6. Add GPU startup to PID 1 only after cold-boot, screen-off/wake and fallback
   acceptance. Slot B remains the rollback.

## Buffer contract

`saai-displayd` treats buffer transport as a negotiated capability:

- `wl_shm` is mandatory and always available;
- dmabuf is advertised only when the render node and backend initialize;
- each imported buffer records fourcc, modifier, dimensions and synchronization
  state;
- import failure affects only the client buffer and falls back to a software
  surface where possible;
- final Pixel scanout preserves the tested BGRX transform;
- GPU loss returns to software composition, then to `drm-splash` if the
  compositor itself enters a crash loop.

GPU acceleration is accepted only when measured frame time, CPU use and power
improve without weakening lock/input isolation or recovery.

## Upstream references

- [Panthor G710/G510/G310 v9 patch](https://lore-kernel.gnuweeb.org/dri-devel/20250807162633.3666310-4-karunika.choo@arm.com/)
- [Panthor dma-buf and cache synchronization series](https://lore-kernel.gnuweeb.org/dri-devel/38811d77-53b3-405d-8424-438ccfcc7fc1@arm.com/t/)
- [Arm Mali-G710 architecture overview](https://developer.arm.com/community/arm-community-blogs/b/mobile-graphics-and-gaming-blog/posts/new-suite-of-arm-mali-gpus)
