# Pixel 7 (panther) graphics acceleration

## Current state (SaaiOS)

The verified display path is Exynos DRM/KMS scanout at 1080x2400x60 with a
CPU-rendered XRGB8888 dumb buffer. `drm-splash` converts logical RGB to the
panel's verified BGRX byte layout. It remains the boot and recovery renderer.

No GPU kernel driver, render node, Mali CSF firmware, Mesa, GBM or EGL stack is
started by SaaiOS PID 1. Hardware acceleration is therefore **not claimed**.

A local stock kit is extracted under
`os/targets/panther/artifacts/gpu/` (gitignored). Hashes:
[gpu-kit.sha256](gpu-kit.sha256). Re-pull with
`os/targets/panther/scripts/collect-gpu-artifacts.sh`. Optional ramdisk
pack (modules + CSF only, still no PID 1 load):
`os/targets/panther/build-gpu-vendor-boot.sh`.

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

## Stock kit (collected 2026-10-05, not in PID 1)

Pulled from live panther `google/panther/panther:17/CP2A.260705.006/15641320`.
`vendor/lib/modules/mali_*.ko` matches `vendor_dlkm` (same SHA256).
`/vendor/etc/mali` is **absent** on this build; `fw_name` is the module
parameter `mali_csffw-r54p3.bin`.

| Path in kit | Role | Size |
|---|---|---|
| `modules/mali_kbase.ko` | CSF kbase r54p3 | 2 733 424 |
| `modules/mali_pixel.ko` | Pixel MGM/PCM/PMA | 46 616 |
| `modules/gpu_cooling.ko` | GPU DVFS cooling | 45 888 |
| `firmware/mali_csffw-r54p3.bin` | Live CSF image | 282 624 |
| `firmware/mali_csffw-r54p{0,1,2}.bin` + `legacy-r56p0.bin` | Alternate CSF | ~278–286 KiB |
| `egl/libGLES_mali.so` | Proprietary GLES 3.2 | 52 543 840 |
| `hw/vulkan.mali.so` | Vulkan ICD | 133 576 |
| `egl/libOpenCL.so` + `libOpenCL-pixel.so` | OpenCL | 82 808 + 14 440 |
| `hw/mapper.pixel.so` + allocator AIDL | Android gralloc | Android-only |
| `modules/deps/*.ko` | kbase `depends=` | SoC glue, already on vendor_dlkm |

`mali_kbase` `depends=`: systrace, google_bcl, exynos-pmu-if, exynos-pd,
itmon, cmupmucal, exynos_pm_qos, bts, gpu_cooling, mali_pixel, dss.

`mali_pixel` `depends=`: pixel_stat_sysfs, pixel_stat_mm, slc_pt.

`gpu_cooling` `depends=`: ect_parser, cmupmucal.

`google_bcl` itself pulls the charger/MFD tree. Do **not** insmod it from
this kit on a cold SaaiOS boot expecting GPU-only deps; it is already live
on stock vendor_dlkm with the power stack.

Manual load order (stock kernel, **not** PID 1), after those deps exist:

1. `gpu_cooling.ko`
2. `mali_pixel.ko`
3. `mali_kbase.ko` with firmware `mali_csffw-r54p3.bin` on the firmware
   search path
4. Confirm `/dev/mali0` and `/dev/dri/renderD128`

`libGLES_mali.so` / `vulkan.mali.so` / mapper / allocator are **Android
Bionic + HIDL/AIDL**. They will not `dlopen` on SaaiOS musl. Userspace on
SaaiOS still needs Mesa **Panthor** + GBM, or a non-Android EGL port — the
kit is the matching kernel/CSF side, not a drop-in renderer.

Kernel `uname` `…gbd23337e42e7-ab14791245` vs module vermagic
`…ge4470993d947-ab15260412` is what stock actually loads (modversions).
Do not mix these kos onto a different `ab*` kernel.

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
