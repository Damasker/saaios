# Pixel 7 (panther) graphics acceleration

## Current state (SaaiOS)

The verified display path is Exynos DRM/KMS scanout at 1080x2400x60 with a
CPU-rendered XRGB8888 dumb buffer. `drm-splash` converts logical RGB to the
panel's verified BGRX byte layout. It remains the boot and recovery renderer.

No GPU kernel driver, render node, Mali CSF firmware, Mesa, GBM or EGL stack is
currently packaged by SaaiOS. Hardware acceleration is therefore **not
claimed**.

## Live stock GPU (2026-10-05)

Stock Android on the same panther **does** run Mali. Scanout and render are
**different devices**.

| Piece | Live |
|---|---|
| GPU | **Mali-G710**, **7 cores**, `r0p0`, id `0x0A080602` |
| Kernel identify | `GPU identified as 0x2 arch 10.8.6 r0p0 status 4` |
| DT | `/mali@28000000` `arm,malit6xx` → `28000000.mali` |
| Kbase | `mali_kbase` **r54p3-00eac0** (UK 1.38), `mali_pixel`, probed `mali0` |
| CSF firmware | loaded `Mali firmware 0x1050000`, git `690855d0…`, blobs `/vendor/firmware/mali_csffw-r54p0`…`r54p3.bin` (r54p3 **282624** B) plus `mali_csffw-legacy-r56p0.bin` |
| Userspace | `ro.hardware.egl=mali`, `ro.hardware.vulkan=mali`, GLES **3.2** (`ro.opengles.version=196610`) |
| GLES string | `ARM, Mali-G710, OpenGL ES 3.2 v1.r54p3-00eac0.1848e3b066182d5bb5a345ab256f13ee` |
| Blobs | `libGLES_mali.so` **52 543 840** B; `vulkan.mali.so` **133 576** B (ICD, not the full GLES blob) |
| Render nodes | `/dev/mali0` (10,88), `/dev/dri/renderD128` |
| Scanout | `/dev/dri/card0` **exynos-drm**, connector `card0-DSI-1` (+ Writeback). **Not** a Mali display block |
| Power domains | `18061e00.pd-g3d`, `18062000.pd-embedded_g3d` |
| DVFS | freqs **202 / 251 / 302 / 351 / 400 / 471 / 510 / 572 / 701 / 762 / 848** MHz; max initialized 848000 kHz; governors `basic quickstep quickstep_use_mcu capacity_use_mcu` |
| Cooling | `thermal-gpufreq-0` max 10; zone G3D; OCP IRQs `ocp_gpu` / `soft_ocp_gpu` |
| Helpers | `mali-mgm` memory group manager, `mali-pcm` priority (DT: none configured), `mali-pma` protected allocator |
| Protected | `vendor.mali.base_protected_max_core_count=4`, TLS max 64 MiB; RenderEngine protected context supported |
| QoS | `bts` + `exynos_pm_qos` consumers include **mali_kbase** (same tree as cpif) |
| Mem this capture | `total_gpu_mem` ~145 899 520; SurfaceFlinger (pid 540) ~105 MB |
| Userspace svcs | `gpuservice`, `surfaceflinger`, `hwc3-service.pixel` (Skia/Ganesh GLES) |

G710 Valhall CSF is **not** Panfrost. Upstream open driver is **Panthor**.
Stock does **not** use Panthor; it uses proprietary **kbase r54p3** + CSF
`mali_csffw-r54p3`. Copying those blobs into SaaiOS without the matching
signed `mali_kbase.ko` + DT/power domains will not bind.

Do **not** point Mesa at `/dev/dri/card0` expecting a Mali render node:
that card is Exynos DSI. Render is `renderD128` / `mali0`.

GXP (`25c00000.gxp`, Janeiro) is the **camera** coprocessor, not the
3D GPU. Do not load GXP for UI acceleration.

Pixel 7 uses Mali-G710 MP7. Its upstream open-driver path is Panthor;
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
