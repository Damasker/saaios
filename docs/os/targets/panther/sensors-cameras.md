# Pixel 7 (panther) cameras, fingerprint, proximity, sensors

Live on stock **2026-10-05**, SKU **GVU6C**, while the phone was idle
(camera HAL ready, UDFPS HAL up). Names from sysfs / CHRE
`sensorservice`, not guessed part numbers. No serials.

Radio-adjacent silicon: [hardware-risks.md](hardware-risks.md).

SaaiOS today has touch, DRM panel, and CS40L26 haptics. **None** of
the camera / UDFPS / CHRE stack below is claimed.

## Fingerprint (under-display)

| Item | Live |
|---|---|
| Vendor | **Goodix** UDFPS |
| DT | `/odm/goodixfp` `goodix,fingerprint` |
| Driver | `goodix_fp` → `odm:goodixfp` |
| Nodes | `/dev/goodix_fp`, `/sys/class/goodix_fp`, input **`event1` `goodix_fingerprint`** |
| HAL | `android.hardware.biometrics.fingerprint-service.goodix` |
| TA | `ro.vendor.fingerprint.ta.name=g7.app` (Trusty) |
| Display | `persist.vendor.udfps.lhbm_controlled_in_hal_supported=true` |
| ALS assist | `persist.vendor.udfps.als_feed_forward_supported=true` (uses **TMD3719**) |

This boot logged `vendor.fingerprint.init.error=GF_ERROR_CALIBRATION_NOT_READY`.
After a stock reflash / userdata wipe the cal blob may be missing until
an enroll/cal path runs. Do not treat that as a SaaiOS kernel bug.

Face unlock is **not** a second FP: `android.hardware.biometrics.face-service.pixel`
+ front camera (`vendor.face-hal`). No dedicated IR/ToF face module.

## Two “proximity” devices (easy to mix up)

| Role | Chip | Where |
|---|---|---|
| Screen-off / pocket / SystemUI | **AMS TMD3719** | CHRE: `android.sensor.proximity` + `android.sensor.light`. Wake-up. |
| Camera AF assist / ToF | **ST VL53L1** | I2C `1-0029` `stmvl53l1`, input **`event3`**, module used by **LWIS** (`stmvl53l1 1 lwis`) |

Call screen-off must use **TMD3719**, not VL53. Camera PDAF/ToF is VL53.

TMD3719 also feeds auto-brightness (`com.google.sensor.auto_brightness`)
and UDFPS ALS feed-forward. Rear **VD6282** is a separate color/light
sensor for the camera (`com.google.sensor.rear_light`).

## Cameras (Google LWIS, not V4L sensors)

There is **no** `/dev/video*` for the imagers. `video6`–`video12` are
**MFC decode/encode** and **exynos-jpeg**, not CSI cameras.

Live LWIS nodes:

| Node | Role |
|---|---|
| `lwis-sensor-sandworm` | imager (one of three) |
| `lwis-sensor-dokkaebi` | imager |
| `lwis-sensor-nagual` | imager |
| `lwis-eeprom-smaug-sandworm` / `smaug-dokkaebi` / `gargoyle` | module EEPROM |
| `lwis-act-slenderman` | VCM actuator |
| `lwis-ois-gargoyle` | OIS |
| `lwis-flash-lm3644` | flash (TI **LM3644**), `vendor.camera.max_flash_current=1500` |
| `lwis-csi` + `samsung,exynos-csis` | CSI |
| `lwis-ipp` `itp` `mcsc` `pdp` `g3aa` `gdc0/1` `gtnr-*` `scsc` `votf` `slc` `dpm` `top` | Tensor ISP blocks |
| `gxp@25C00000` `google,gxp` + `janeiro` | GXP coprocessor (used by camera) |

Three platform `sensor@0` `@1` `@2`, each supplied by **SLG51002**
(`i2c 8-0075`) and **s2mpg13**. HAL:
`android.hardware.camera.provider@2.7-service-google`,
`vendor.camera.hal.build_id=15316753`.

Public Pixel 7 set is rear 50 MP + ultrawide 12 MP + front 12 MP.
Map those to sandworm/dokkaebi/nagual from Google camera trees when
implementing; do not invent Sony/Samsung part IDs here.

Risks: GXP/Janeiro, cam_supply rails, CSI PHYs, SYSMMU on every LWIS
block, Trusty for some cal, shared **BTS / PM QoS / SLC** with the
modem. A camera bring-up that clocks the wrong PD can fight Shannon.

## IMU / mag / baro (CHRE on AoC)

Not IIO (IIO is only PMIC **ODPM** `s2mpg12/13-odpm`). Sensors HAL:
`android.hardware.sensors-service.multihal`. AoC services
`com.google.chre` and `com.google.usf`.

| Physical | Vendor | Android type |
|---|---|---|
| **LSM6DSV** | ST | accel, gyro, uncal, gyro temp, motion/stationary detect |
| **MMC56X3X** | MEMSIC | mag + uncal |
| **ICP20100** | InvenSense | pressure + pressure temp |
| **TMD3719** | AMS | light + proximity |
| **VD6282** | ST | rear light (camera) |

Google fused: orientation, gravity, linear accel, rotvecs, steps,
tilt, pickup, device orientation, double twist, binned brightness,
proximity-gated tap/long-press, quick pickup, auto brightness,
camera vsync 0–3.

Haptics already on SaaiOS: **CS40L26** input `event5` (`i2c 8-0043`).
Keys: `gpio_keys` `event0`, `s2mpg12-power-keys` `event2`. Touch
`fts_ts` `event4`.

## What SaaiOS would need (order of pain)

1. **TMD3719 via CHRE/AoC** — proximity + ALS; without it, no pocket
   detect and weak UDFPS/brightness.
2. **LSM6DSV + MMC56X3X + ICP20100** on the same CHRE path — rotate,
   compass, baro.
3. **Goodix UDFPS** + Trusty `g7.app` + LHBM on the panel + TMD3719.
4. **LWIS + GXP + three sensors + LM3644** — full camera; largest
   surface, shares power/QoS with the modem.
5. **VL53L1** — camera ToF, not the phone proximity HAL.
6. **Pixel face HAL** — depends on a working front camera.

Do not bind camera sensors as generic V4L2. Do not use VL53 as
`android.sensor.proximity`.
