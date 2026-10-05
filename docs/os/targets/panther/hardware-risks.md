# Pixel 7 (panther) hardware that can break cellular

Live inventory **2026-10-05** on stock slot `_b` while LTE was **ONLINE**
(`modem_state=ONLINE`). Hardware revision **MP1.0**, SKU **GVU6C**,
SoC **GS201**, `ro.boot.hardware.radio.subtype=0`. No IMEI/MAC here.

Use with [modem-stock-reproduction.md](modem-stock-reproduction.md).
SaaiOS already has Wi-Fi/BT/audio bring-up; this list is the **other
silicon** next to Shannon that will fail camp, RF, SIM, or IMS if
missing, unclocked, or in the wrong power state.

## Highest risk (no radio without these)

| Piece | Live id | Why it breaks cellular |
|---|---|---|
| Tensor GS201 + CPIF | DT `samsung,exynos-cp`, modules `shm_ipc` `cpif_page` `cpif` `cp_thermal_zone` `google_modemctl`; `exynos_pcie_iommu` used by `cpif` | No `umts_*`, no `rmnet*`. Autoload does **not** happen; Android `modules.load` does. |
| Shannon S5300 EP | PCI `0000:01:00.0` `144d:a5a5` driver **`s51xx`**. Type string `s5100sit`. Baseband `g5300q-260317-260505-B-15346003` | Wrong firmware / empty `NV_NORM` → WCDMA `REG_DENIED`. Do not treat as SIPC A12. |
| Modem PCIe RC | DT `/pcie@11920000` `samsung,exynos-pcie-rc`, sysfs `11920000.pcie` → `pci0000:00`. Vendor `144d:ecec` (`pcieport`). Link **GEN3**. IRQ `exynos-pcie` + `msi0`–`msi4`. Cal module `pcie_exynos_gs201_rc_cal`. PMIC supplier **`i2c-s2mpg12`**. SYSMMU `11860000` `samsung,pcie-sysmmu` | If this RC is down, CP never enumerates. Wi-Fi uses a **different** RC (`14520000`) — bringing up BCM4389 does not power Shannon. |
| Shared memory | DT `cp_shmem`; reserved `cp_rmem` `cp_rmem_1` `cp_msi_rmem` `cp_aoc_rmem` (`exynos,modem_if`) | UDL/IPC/pktproc. Wrong map → `BOOTING` stall / no `rmnet`. |
| EFS + cal NV | `sda5` efs, `sda6` efs_backup, files `nv_normal` / `nv_protected` | Protected is factory cal + identity. First-boot rfsd restored protected from backup. |
| `modem` partition | slotselect `modem_b` `sda29` ~98 MiB `modem.bin` + `confpack` `cfgdb-whipro_r16-260505-B-15346003` | Image/NV version skew → invalid NV / no camp. |
| Dual-SIM detect | `/sys/devices/platform/cpif/sim/ds_detect=2`, `cbd -s 2`, DSDS | `ds_detect≠2` vs `-s 2` is a known stock mismatch class. |
| ACPM / clocks | `gs_acpm`, `cmupmucal` (cpif is a user), mboxes `18300000` `18320000` `18340000` (high IRQ count on first two) | CPIF needs CMU/PMU. Silent PCIe if rails/clocks off. |
| BTS / PM QoS | `bts` and `exynos_pm_qos` list **cpif** as consumer | Bandwidth/QoS not optional for pktproc. |

Stock modem EP `current_link_speed` in PCI sysfs is `Unknown`/width `0`
even while ONLINE; trust **RC** `link_speed=GEN3`, not the EP PCI
fields.

## High risk (radio up, then dies or never attaches)

| Piece | Live id | Failure mode |
|---|---|---|
| CP thermal | platform `cp-tm1` (`gs101-cp-thermal-zone`), zones `cp_on_chip_0`–`6` (~30–37 °C this capture), module `cp_thermal_zone` | Missing chown/sysfs → throttling or `cbd` unhappy. Do not write `do_cp_crash`. |
| Samsung PMIC pair | **s2mpg12** feeds modem PCIe RC; **s2mpg13** feeds Wi-Fi PCIe RC (`14520000.pcie`). Thermal `s2mpg13_spmic_thermal` | Wrong rail → one of two PCIe ports dead. Independent of `wlan0`. |
| Pixel BCL / `google_bcl` | `google_modemctl` used by `google_bcl` **and** `cpif` | Battery-current limit can mute PA/CP under load. |
| BCM4389 Wi-Fi PCIe | `0001:01:00.0` `14e4:4441` subsys `4389` driver `pcieh` **GEN2**. Host wake GPIO `dhdpcie_host_wake`. Same `pcie_exynos_gs` as CP | Coex / ASPM / shared Exynos PCIe wrapper. Two RCs (`1192` vs `1452`). |
| Broadcom GNSS | SPI `spi5.0` `bcm4775` driver `brcm gps spi`; `gpsd` + `lhd` + `android.hardware.gnss@2.1-service-brcm`; `persist.sys.gps.lpp=2`; node `/dev/sscd_gnss` | LPP/AGPS talks to the modem. Not required for LTE data, required for emergency/location. DT node is **not** named `/gnss`. |
| GSA + Trusty | `gsa.ko` `gsa_gsc.ko`, `17c90000.gsa-ns`, S2MPU `17c60000.s2mpu_gsa`, reserved `gsa@90200000`, `/dev/gsa0`, KeyMint/Gatekeeper on Trusty. OTA partition **`gsa`** | IMEI/NV protect, keystore. Broken GSA → SIM/NV/attestation failures, not a SIT opcode bug. |
| Unused CP SPI | SPI `spi9.0` `exynos-cp-spi` **no driver bound**. DT leftover `/spi@10D20000/cpboot_spi@0` | Do **not** bind this for panther PCIE boot. |

## Voice / IMS (data can work without them)

| Piece | Live id | Notes |
|---|---|---|
| Google AoC | `/dev/aoc`, `aocd`/`aocxd`, FW `14022723-polygon`; reserved `cp_aoc_rmem` | VoLTE path shares AoC with speakers. CS LTE + `rmnet1` worked with AoC running; IMS packages are separate. |
| Shannon IMS/RCS | `com.shannon.imsservice`, `com.shannon.rcsservice` | After bearer. Not the camp gate. |
| Speaker amps | SPI `spi7.0`/`spi7.1` **CS35L41** | Call audio, not attach. |
| Wi-Fi calling nodes | `umts_wfc0` `umts_wfc1` | IWLAN; stock showed IWLAN + LTE together. |

## SIM / eSE / NFC (adjacent, easy to confuse with modem)

| Piece | Live id | Notes |
|---|---|---|
| Physical SIM | phone0 `LOADED`, phone1 `NOT_READY` | One pSIM. Second RIL socket still exists (`umts_ipc1`). |
| eSE | SPI `spi17.0` **st33spi**; HALs `secure_element@1.2-service-gto` and `-gto-ese2` | Wallet/eSE, not the baseband. |
| NFC+eSE bridge | SPI `spi10.0` **st54spi**; `android.hardware.nfc-service.st` | ST54. Do not treat as UICC. |
| UICC SE HAL | `secure_element@1.2-uicc-service` | SIM applet channel; RIL also does `SIM_IO`. |

## Other SoC pieces that fight the same buses

Not cellular, but they share PCIe/SPI/IOMMU and show up in the same
`lsmod` / IRQ table:

- Touch **FocalTech** `spi0.0` `fts_ts` (already brought up on SaaiOS).
- Display **Exynos DSIM** (IRQ noise next to PCIe in `/proc/interrupts`).
- UFS `ufs_exynos_gs` (EFS lives here).
- Mali / G710 — GPU, see [gpu.md](gpu.md); not on the camp path.
- USB DWC3 / `phy_exynos_usbdrd_super` — console vs gadget; does not
  replace `rmnet`.

Cameras, UDFPS, proximity, IMU: [sensors-cameras.md](sensors-cameras.md).

## Kernel modules that must stay with CPIF

Observed `lsmod` edges (users of `cpif` / PCIe):

```text
pcie_exynos_gs          ← bcmdhd4389, cpif
exynos_pcie_iommu       ← cpif, pcie_exynos_gs  [permanent]
google_modemctl         ← google_bcl, aoc_alsa_dev_util, cpif
shm_ipc                 ← cpif
bts                     ← …, cpif, …
exynos_pm_qos           ← …, cpif, pcie_exynos_gs, …
cmupmucal               ← …, cpif, …
ect_parser              ← cpif, …
```

Load order still: `shm_ipc` → `cpif_page` → `cpif` → `cp_thermal_zone`,
**after** `pcie-exynos-gs` and `google_modemctl`.

## What this phone is (so firmware matches)

- Device `panther` (Pixel 7, not `cheetah`).
- Board rev `MP1.0`, color `WHT`, SKU `GVU6C`.
- Radio subtype `0`.
- Two Exynos PCIe RCs: **11920000 = Shannon**, **14520000 = BCM4389**.

Mismatched `modem.bin` / confpack / NV from another SKU or Pixel 7 Pro
is a likely silent camp failure. Keep NV on-device; do not commit it.
