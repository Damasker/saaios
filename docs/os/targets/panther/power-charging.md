# Pixel 7 (panther) wireless charging and power path

Live **2026-10-05**, SKU **GVU6C**. No pad on the desk: `power_supply/wireless`
`online=0` `present=0`. Do not log `serial_number` from that psy.

## Wireless (Qi)

| Item | Live |
|---|---|
| Coil IC | **P9412** (IDT/Renesas family) |
| DT | `/hsi2c@10DA0000/p9412@3c` |
| Linux driver | `p9221.ko` — dmesg `selecting p9412`, I2C **`15-003c`** |
| Enable GPIO | 110, active-low (`WLC enable/disable pin:110`) |
| Psy | `/sys/class/power_supply/wireless` `type=Wireless` |
| HAL | `vendor.google.wireless_charger-default` (`wireless_charger_AIDL`) |
| Features in DT | `idt,has_rtx` (reverse TX), `idt,has_wlc_dc` (DC / Pixel Stand), EPP/HPP FOD tables, alignment current |

Sysfs on the i2c device includes `force_epp`, `operating_freq`, `fw_rev`,
`is_rtx_connected`, `has_wlc_dc`, `fan_level`, `alignment`. Not exercised
without a pad.

`hal_wlcservice` was **stopped**; AIDL wireless charger was **running**.

## Wired USB-C (same Google BMS tree)

WLC is not a separate battery. It dumps into **Google BMS**:

| Block | Live |
|---|---|
| Type-C / PD | **MAX77759** TCPC, `tcpci_max77759`, `/sys/class/typec/port0` |
| Main charger | `main-charger`, `google_charger` |
| Charge pumps / PPS | **PCA9468**, **HL7132**, **LN8411**, `google_cpm` (`gcpm`, `gcpm_pps`) |
| Fuel gauge | **MAX77779** FG (`maxfg`) + `google_battery` |
| USB psy | `usb` + `tcpm-source-psy-i2c-max77759tcpc` (this capture: 5 V partner, ADB) |
| Dock / stand | `google_dock`, `google_ccd` |

This capture: battery Li-ion ~100%, USB online, wireless offline, DC offline.

Modem BCL (`google_bcl`) sits on the same charger/FG modules — a WLC or
PPS bug can throttle the CP the same way a hot battery does.

## SaaiOS gap

SaaiOS already reads MAXFG percentage for STATUS. It does **not** run
`p9221`, wireless AIDL, CPM, or Pixel Stand. Putting the phone on a Qi
pad under SaaiOS will not charge until this path is brought up (or
until stock Android is booted).
