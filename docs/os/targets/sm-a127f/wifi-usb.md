# RTL8188EUS — USB dongle driver, not a12s wlan

User pointed at [aircrack-ng/rtl8188eus](https://github.com/aircrack-ng/rtl8188eus) the same way as the a12s kernel tree (“use this, don’t reinvent”). **Wrong bus for onboard Wi‑Fi.** Do not treat it as a replacement for Samsung `wlan0`, and do not mix it into [kernel-touch.md](kernel-touch.md) / TD4150 / synaptics.

## What the repo is

Out-of-tree **USB** 802.11n driver for Realtek **RTL8188EUS / RTL8188EU / RTL8188ETV** (and a pile of rebrands). aircrack-ng fork: monitor mode, frame injection, MESH. Default branch `v5.3.9`. README now says **these drivers are deprecated** — prefer mac80211 [lwfinger/rtw88](https://github.com/lwfinger/rtw88).

| | |
|--|--|
| Chip | RTL8188E USB (not SDIO, not Exynos combo) |
| Typical VID:PID | `0bda:8179` (8188EUS), `0bda:0179` (8188ETV); also TP-Link `07b8:8179`, `2357:010c`, D-Link `2001:330f` / `3310` / `3311` / `331b`, Edimax `7392:b811`, … |
| Kernel | README: Android 12/13, “up to v6.5+”. Tree is still a 4.x/5.x-era vendor driver |
| Build | `make && sudo make install`; blacklist `r8188eu` and `rtl8xxxu`; `modprobe 8188eu` |
| Monitor | `airmon-ng check kill`; `ip link set <if> down`; `iw dev <if> set type monitor`; injection test `aireplay-ng -9 <if>` |

This is a **host USB stick** driver. SM-A127F onboard radio is Samsung/Exynos SCSC Maxwell `wlan0` (`119c0000.scsc_wifibt`, `mx140.bin` in `/vendor/etc/wifi`). Phase E3 is that path — **v031 LIVE** (`/sbin/wifi-join` by args; Wallbox `192.168.168.8`). No PSK in the image. See [hardware-control-plan.md](hardware-control-plan.md). Not this chip. Cellular is E4 ([modem.md](modem.md)) — CP OFFLINE, no write.

## Where it belongs in SaaiOS

| Place | Verdict |
|-------|---------|
| **R620 / saaios-vm USB Wi‑Fi** | Right home **if** `lsusb` shows `0bda:8179` (or a rebrand above). Clone gitignored `os/third_party/rtl8188eus`, out-of-tree `make` against the **running host** kernel. Do not commit the tree. |
| **Phone (Phase E radios)** | Onboard Wi‑Fi = Maxwell / `wifi-up` / `wifi-join` (E3, v031 LIVE join). Cellular = CPIF / `cbd` (E4 mapped, CP OFFLINE). An 8188 stick needs USB **host**/OTG and drops RNDIS. Stock DXJ2 has `CONFIG_USB_DWC3_DUAL_ROLE`. v028 `/sbin/usb-host` is explicit; boot stays gadget. See [hardware-control-plan.md](hardware-control-plan.md). |
| **a12s vendor Image** | Do **not** turn `CONFIG_RTL8188EU` back on for TSP. Project-Xed already ships an in-tree copy; `os/Makefile` `kernel` disables it (`clang-9` `-Werror=implicit-fallthrough`). Not onboard `wlan`. |
| **TD4150 / touch** | Unrelated. Synaptics is SPI. This tree does not replace `synaptics/td4150`. |

## R620 check (2026-08-19)

Hostname `R620`, kernel `6.12.96+deb13-amd64`. `lsusb`: **no Realtek**. Present: Intel hubs, Dell hub, Avocent virtual HID, Kingston DataTraveler `0951:1666`. No clone, no `make` — 8188eus on 6.12 would likely fail anyway (README tops out ~6.5; Debian already has `rtl8xxxu` / rtw88 for when a stick appears).

Intended clone (when a stick is plugged, **not** done):

```text
os/third_party/rtl8188eus   # gitignored; do not commit
```
