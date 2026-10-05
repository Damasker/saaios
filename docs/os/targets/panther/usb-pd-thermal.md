# Pixel 7 (panther) USB-C PD and thermals

Live **2026-10-05** with a USB **host PC** attached (ADB). Wireless pad
absent. Complements [power-charging.md](power-charging.md).

## Type-C port

`/sys/class/typec/port0` — TCPC **MAX77759**.

| Field | This capture |
|---|---|
| `port_type` | dual (source and sink) |
| `preferred_role` | sink |
| `data_role` | **host** [device] (phone is ADB gadget-host toward PC? live: host selected, device available) |
| `power_role` | **source** [sink] |
| `power_operation_mode` | `usb_power_delivery` |
| USB PD rev | **3.0** |
| Type-C rev | **1.2** |

Two PD instances: `pd0` (phone as source while data-host) and `pd1`
(phone as sink / charger).

### Phone as source (`pd0`)

One PDO: **5 V, 900 mA**, dual-role data+power, USB comm capable,
USB suspend supported.

### Phone as sink (`pd1`) — what a charger may offer

| PDO | Voltage | Current |
|---|---|---|
| 1 fixed | 5 V | 900 mA (also dual-role) |
| 2 fixed | 9 V | 1000 mA |
| 3 fixed | 12 V | 1000 mA |
| 4 fixed | 14.8 V | 1000 mA |
| 5 fixed | 15 V | 1000 mA |
| 6 variable | 19–20 V | 1000 mA |

Hardware behind this: MAX77759 + PCA9468/HL7132/LN8411 CPM (see
power-charging). Contaminant detection modules
`max77759_contaminant` / `max77779_contaminant` / `max777x9_contaminant`.
Cooling device `usbc-port`.

SaaiOS CDC-NCM/ADB does not implement this PD policy. A PD charger
may still negotiate in the kernel TCPC; userspace Google charger HAL
is what actually programs FCC/DC_ICL.

## Thermal zones (this idle capture)

Temps are millidegree C unless noted. Empty rf zones were unread.

| Zone | Temp | Notes |
|---|---|---|
| BIG / LITTLE | 40000 | CPU |
| MID | 38000 | |
| G3D | 37000 | GPU |
| ISP / TPU / AUR | 37000 | camera / TPU / always-on island |
| cp_on_chip_0..6 | 30–37 °C | modem; 6 empty this read |
| disp_therm | 31038 | panel |
| maxfg / battery / batt_vs | 30600 | gauge |
| qi_therm | 33219 | coil (no pad) |
| usb_pwr_therm / therm2 | ~31 °C | USB |
| gnss_tcxo_therm | 31950 | GNSS TCXO |
| quiet_therm / neutral_therm | ~31 °C | chassis |
| pca94xx-mains | 3300 | charge-pump (scale differs) |
| usbc-therm-adc | 960 | ADC counts, not °C |
| batt_therm_sidekey | −26428 | unused/open? |
| soc | 1 | flag, not temperature |

kthreads: `thermal_BIG/MID/LITTLE/G3D/ISP/TPU/AUR`.

## Cooling devices (cur=0 idle)

CPU `thermal-cpufreq-0/1/2`, GPU `thermal-gpufreq-0`, `tpu_cooling`,
`gxp-cooling` (camera GXP), `DISP` `CAM` `TNR` `MFC` `BO` `INT` `INTCAM`
`MIF`, charger `chg_mdis` `fcc` `dc_icl` **`wlc_fcc`**, `usbc-port`.

Modem BCL and `wlc_fcc` mean a hot coil or PPS session can starve
Shannon the same way speaker BCL can.

`rf1_therm` / `rf2_therm` exist as zone names; this dump did not get
a numeric temp (empty sysfs). Treat as RF thermistors next to the PA.
