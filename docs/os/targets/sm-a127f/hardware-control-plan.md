# SM-A127F — hardware control (OS track)

Endpoint is **command of this board**, not “Android boots” and not Platform Track AI. Bring-up of a ramdisk ([bringup-plan.md](bringup-plan.md)) already happened. Touch resume patch detail: [kernel-touch.md](kernel-touch.md).

Worker: docs, kernel patch, pack `boot.img`. Human: Download mode + Odin AP. **Do not flash from `make`.** Do not unbind TSP. Do not race another worker cloning `kernel_samsung_a12` or packing boot-v012.

```text
our init / BusyBox  →  sysfs + evdev + fb0  →  board
```

---

## Goal

Userspace we own (PID 1 now; a small compositor later) can **command**:

1. **Display** — own `fb0` (DRM only if a custom kernel ever exposes it); brightness / backlight.
2. **Input** — real finger coordinates from TD4150 (`event3`), plus volume and power keys.
3. **Power** — suspend/resume, and a real power-off path **in the kernel**, not a ramdisk fake.
4. Then, **one subsystem at a time**: audio, Wi‑Fi / modem, USB roles.

## Non-goals

| Out | Why |
|-----|-----|
| GSI / Lineage / “Android boots again” | Wrong product |
| KernelSU / Magisk / root-as-product | Tree may ship it; we do not ship it |
| Flashing BL / TZ / EFS / sboot | Download mode lives in sboot; we keep it |
| Mainline 4.19 replacement as step 1 | Vendor kernel first |
| Inventing a TSP driver | Reuse Samsung `synaptics/td4150` |
| Platform Track (`crates/`, `services/`) | Separate track |

---

## Now (boot-v033 LIVE — real hardware poweroff confirmed; E4 CP BOOTING; TSP parked)

Stock **DTB + kernel** in `boot.img` (`4.19.111-27127798`). Our ramdisk only. Vbmeta patched Magisk-style (flags OR 3). Pack: `SEANDROIDENFORCE` + pad **44 MiB**. Odin **AP**, human-only.

TD4150 bring-up is **parked** ([kernel-touch.md](kernel-touch.md)). Do not flash sinceN opcode images for product work. Return to TSP only with a stock-Android dmesg or a new discriminator, not another 0x05/0x30.

| Piece | State |
|-------|--------|
| PID 1 | our `/init` + BusyBox |
| Display | `fb0` console: banner, battery, backlight, USB, mem, `aud` |
| Backlight | sysfs `/sys/class/backlight` or `leds` (panel/lcd preferred). Vol±. Never a second `FBIOBLANK` |
| USB | RNDIS `0525:A4A2`; telnet **23 / 2323**; dropbear **22**. v028 `/sbin/usb-host` / `/sbin/usb-device` — **never host at boot**. v029 does not change USB |
| Volume | `event1` `gpio_keys` — brightness |
| Power key | `event2` `sec-pmic-key` — tap `/sbin/beep`; 2s reboot (kernel `poweroff` still missing) |
| Audio | SMA1303 / ABOX **pcmC0D1p** → SIFS1 → UAIF1. `/sbin/play` WAV **LIVE 2026-08-29**. [audio.md](audio.md). Vendor RDMA3/SIFS0 is silent |
| Wi‑Fi | **v031 LIVE join.** Maxwell fw + `/sbin/wifi-join` by args. Wallbox WPA2; `wlan0` **192.168.168.8** (telnet-only — not at boot; after reboot run join again). No PSK in image |
| Modem | **E4 TOC + load LIVE.** Full 5 TOC records; `MAIN` is 37 MiB on RADIO; `NV` is `b_off=0` (EFS, not RADIO). Vendor `cbd` wget’d but cannot run (no `linker64`). Static `/tmp/radio-boot` loaded BOOT+MAIN+**real NV from userdata copy**; **`modem_state` OFFLINE→BOOTING**, not ONLINE. Complete ioctl timed out. EFS listed `ro,noload` then umounted; NV copied to userdata `saaios-efs-copy` (userdata **formatted** — Android data gone). Original efs never written. No `cbd`. No v032. [modem.md](modem.md) |
| Touch | parked. Node may exist; do not unbind; do not send TSP opcodes from init |
| Poweroff | ramdisk `/sys/power/state` = `freeze mem` only; long-press reboots |

Pack: `make -f os/Makefile boot-v031` (stock Image, no maze). Tar `$(MEDIA)/saaios-boot-v031.tar`. Does not overwrite v021–v030. `make flash` refuses. Rollback: `saaios-boot-v030.tar` or `saaios-boot-v029.tar`.

Already visible in sysfs (read-only until a phase owns it): `fb0`, gpio volume, PMIC power key, battery, `/sys/class/sec/tsp/cmd`, USB gadget.

Kernel tree to **reuse** (a12s / Exynos 850, not Helio): [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12). Do not invent a driver.

## Endpoint vs now

```text
now:      v033 LIVE — /sbin/poweroff confirmed real hardware shutdown (Power key still unchanged)
          v032 LIVE — Phase 4 dirty-rect console (no full-panel repaint flash)
          v031 LIVE — wifi-join by args; Wallbox 192.168.168.8
          v030 packed — /sbin/iw + /sbin/wifi-scan
          v029 LIVE — Maxwell + first Wallbox join (PSK only in /tmp)
          v028 LIVE device — RNDIS/telnet; usb-host opt-in (not run)
          v027 LIVE play — /sbin/play WAV on pcmC0D1p / SIFS1 / UAIF1 / SMA1303
          ↓  Phase E4 modem — TOC LIVE; CP BOOTING (not ONLINE); EFS/rild blocked
          ─  Phase A (TSP) parked; see kernel-touch.md
```

---

## Phases

Each phase is one ENGINEERING.md loop: Goal / Current / Change / Test / Acceptance / Rollback. Smallest verifiable step. Tagged `boot-vNNN.img` is the return point. Human flashes Odin AP; `make flash` **refuses**.

### Phase A — vendor Image + TSP `do_reset` — **PARKED**

Do not schedule. Last packed discriminator is v019 since76. Init no longer drives `/sys/kernel/saaios_touch`. Details: [kernel-touch.md](kernel-touch.md).

### Phase B — input into our userspace — **keys on v020 / v021**

Touch packets are not a dependency for the console. Volume and power are live. Finger cursor waits on Phase A.

### Phase C — display control — **v020 LIVE**

**Depends on:** none of Phase A.

| | |
|--|--|
| Goal | Command brightness / backlight; status overlay |
| Current | v011 painted `fb0`; backlight sysfs unused; v019 was a TSP lab |
| Change | Ramdisk console on **stock** Image. Probe `/sys/class/backlight` (prefer panel/lcd). Vol± writes `brightness` (min ~1/16 of max, never 0). Battery + mem + USB on `fb0`. Unblank **once**. No TSP opcodes |
| Test | Human Odin AP `saaios-boot-v020.tar`. Banner `SaaiOS v020`. Vol± visibly changes backlight. Battery % updates. Telnet still works |
| Acceptance | **LIVE 2026-08-29.** Banner `SaaiOS v020`. Vol± brightness. Battery on screen. Telnet/RNDIS. No maze Image. No second unblank |
| Rollback | `saaios-boot-stock-restore.tar` or last good v011 tar |

Do **not** couple a new blank path with a TSP experiment.

### Phase D — power policy — **v033 LIVE: reboot(RB_POWER_OFF) is a real hardware shutdown**

**Depends on:** keys (already true). Real `poweroff` needs a kernel change later; not bundled with TSP.

| | |
|--|--|
| Goal | Long-press power in init; real `poweroff`; then suspend/resume |
| Current | `/sys/power/state` = `freeze mem`; ramdisk cannot halt |
| Change | **Kernel:** enable a real power-off path (`poweroff` / `disk` or vendor equivalent — whatever this 4.19 actually implements; do not fake it in BusyBox). **Init:** long-press `KEY_POWER` → that path. Then `echo mem` suspend and a resume that still has touch (Phase A patch must survive) |
| Test | Long-press → device off. Power-on from cold. Later: suspend, wake on power key, `event3` still alive |
| Acceptance | Off is a kernel shutdown, not a hang. Resume does not re-deaf the TSP |
| Rollback | Previous Image; do not iterate power and TSP in one flash |

**First probe (v033, packed, not flashed):** `/sys/power/state=freeze mem` only limits *suspend* states; it says nothing about `reboot(2)` `RB_POWER_OFF`, a separate path (`kernel_power_off()` → board `pm_power_off` hook) that stock Samsung kernels normally wire for a real hardware shutdown. `do_reboot()` in `os/init/init.c` only ever called `LINUX_REBOOT_CMD_RESTART` — `RB_POWER_OFF` was never tried. Added static `/sbin/poweroff` (`sync()` then `reboot(RB_POWER_OFF)`, logs to stderr if it returns instead of powering off) as a **telnet-only** probe, same style as `touchlab`/`beep`/`play`. **Deliberately not wired to the Power key** — the existing 2s long-press → `do_reboot()` is unchanged, because [kernel-touch.md](kernel-touch.md)'s `state=dead` recovery flow depends on that exact gesture. Confirm the stock kernel actually powers off (not a hang, not a silent return) via telnet before binding any gesture to it. Pack: `make -f os/Makefile boot-v033`. Tar `$(MEDIA)/saaios-boot-v033.tar`. Does not overwrite v021–v032. Rollback: `saaios-boot-v032.tar`.

**Test when flashed:** telnet in, `/sbin/poweroff`. If the screen goes dark and the device stays off (needs the physical power button or a charger to wake), the path works and the next step is wiring a gesture. If `reboot()` returns, the stderr line names the `errno`; if it hangs, that's the discriminator this probe exists to find.

**LIVE 2026-09-02 (v033, human Odin AP):** telnet `/sbin/poweroff` — telnet session aborted mid-command (not a timeout), RNDIS USB adapter disappeared entirely from the Windows host (not just unreachable), device screen off. Human confirmed: **device powered off, then auto-powered back on when a charger was connected** (standard PMIC behavior, not ramdisk code). `reboot(2) RB_POWER_OFF` reaches a real `pm_power_off` on this stock kernel — confirms the discriminator this probe was built to answer. Long-press → `do_reboot()` (2s, `LINUX_REBOOT_CMD_RESTART`) is still unchanged and still owns that gesture. Next: decide a Power-key binding for poweroff that does not collide with the existing 2s-reboot recovery gesture (kernel-touch.md depends on it), then suspend/resume (`echo mem`) with a TSP-survives-resume check.

### Phase E1 — speaker beep + WAV play — **v027 LIVE**

**Depends on:** Phase C return point (v020 LIVE). Detail: [audio.md](audio.md).

| | |
|--|--|
| Goal | Hear a tone, then a short melody, from the loudspeaker from our ramdisk |
| Current | **LIVE 2026-08-29.** `/sbin/play` WAV on `pcmC0D1p` / SIFS1 / UAIF1 / SMA1303. User said **работает**. First `/tmp` on v026, then confirmed |
| Change | v027: `/sbin/play FILE.wav` on that same route. Packed `/usr/share/sounds/test.wav` (Ode to Joy). No auto-play. Power tap stays beep. Stock Image |
| Test | Human Odin AP `saaios-boot-v027.tar`. Banner `SaaiOS v027`. Telnet `/sbin/play /usr/share/sounds/test.wav` |
| Acceptance | **LIVE 2026-08-29.** Melody from the speaker. Replay `/sbin/play /usr/share/sounds/test.wav`. Do not use `pcmC0D3p` / SIFS0 |
| Rollback | `saaios-boot-v026.tar` |

Do **not** toggle `Codec Enable`, SMA1303 `I2C Reg Reset`, or `Force AMP Power Down`. Leave `HP`/`EP`/codec `SPK` off.

### Phase E2 — USB roles — **v028 LIVE device (host untested)**

**Depends on:** Phase E1 return point (v027 LIVE). Stock DXJ2 Image — no maze.

| | |
|--|--|
| Goal | Explicit host vs device on the Type-C DWC3 without losing the RNDIS console at boot |
| Current | **LIVE 2026-08-29 probe on v027.** UDC `13600000.dwc3` **configured** / high-speed / `is_otg=1` / `state=b_peripheral` / `id=1` `b_sess=1`. configfs `g1` bound (`rndis.usb0` + `acm.gs0`). `android_usb` CONFIGURED. **No** `/sys/class/usb_role` (`CONFIG_USB_ROLE_SWITCH` is not set). `/sys/bus/usb/devices` **empty**; `xhci-hcd` driver present but unbound. Type-C `port0` (SM5714 `i2c-0/0-0033`): `data_role=host [device]`, `port_type=[dual]`, `power_role=source [sink]`, files **rw**. `host_notify/usb_otg/mode=PERIPHER`. Stock `/proc/config.gz`: `CONFIG_USB=y` `CONFIG_USB_DWC3_DUAL_ROLE=y` `CONFIG_USB_XHCI_HCD=y` `CONFIG_USB_GADGET=y` `CONFIG_USB_CONFIGFS_RNDIS=y` `CONFIG_TYPEC=y` `CONFIG_PHY_EXYNOS_USBDRD=y`. `# CONFIG_USB_OTG` `# CONFIG_USB_DWC2` `# CONFIG_USB_ROLE_SWITCH`. **Stock kernel can host.** No new Image. |
| Change | Ramdisk v028: `/sbin/usb-host` unbinds RNDIS then writes typec `host` + dwc3 `id=0`; `/sbin/usb-device` reverses. Flag `/tmp/usb-role-host` stops init `retry_gadget`. Boot still forces device (`typec data_role=device`, dwc3 `id=1`) then binds gadget. No host key combo. Power 2s reboot = device. Stock Image. Same D1/SIFS1 play. Tar SHA256 `673c9b0ad6dd5be76ec4b4f5f9edc0f7218270b3389e5117b9612cc476fe28c8`. |
| Test | Human Odin AP `saaios-boot-v028.tar`. Banner `SaaiOS v028`. Telnet **before** any host switch. Confirm `usb-device` is a no-op while still gadget. Then, with Power 2s in mind: `/sbin/usb-host` — expect RNDIS drop. Reboot. Telnet back. Optional later: OTG adapter + stick, then `lsusb` / `/sys/bus/usb/devices`. |
| Acceptance | **LIVE device + switch 2026-08-29.** Banner `SaaiOS v028`. `usb-host` drops RNDIS. Power 2s reboot returns device (photo: `rndis0`, telnet, play). Host/stick not confirmed. |
| Rollback | `saaios-boot-v027.tar` |

Do **not** run `usb-host` from init or `rcS`. Do **not** write typec/`id` from a live v027 debug session (this probe did not).

### Phase E3 — onboard Maxwell Wi‑Fi — **LIVE join (v031)**

**Depends on:** Phase E2 return point (v028 LIVE device). Stock DXJ2 Image — no maze. Not RTL8188EUS.

| | |
|--|--|
| Goal | Start the SCSC Maxwell radio so `wlan0` has a real MAC (firmware load + `slsi_start`), scan 2.4 GHz SSIDs, then join a WPA2 AP from telnet |
| Current | **LIVE v029 2026-08-29.** Banner `SaaiOS v029`. `/sbin/wifi-up` → `wlan0` MAC `00:00:0f:08:0e:af`. `mx140.bin` + hcf. 2.4 GHz Y, 5 GHz N. MIB `NACHO_S612_A127F`. **LIVE scan:** static `iw` 6.9 at `/tmp/iw` (12 BSS / 11 named SSIDs). **LIVE join** the same day: static `wpa_supplicant` 2.11 + `wpa_cli` on `/tmp` only; **Wallbox** WPA2-PSK; `wlan0` **192.168.168.8/24**. RNDIS kept. PSK never left `/tmp`. |
| Change | v029: vendor `/etc/wifi` production files in ramdisk `/vendor/etc/wifi`. `/sbin/wifi-up`. Tar SHA256 `a9e743caa02da38f6dcc5bc7adbf4ddd81dcb31b6d06eccc8deb49fe01bb2cf7`. v030: `/sbin/iw` + `/sbin/wifi-scan`. Tar SHA256 `8eff38986a11fdc348fe1f6941f3aec3d8f2c9b7f108c2d3a70a9c13ab396bc3`. v031: same stock Image + static `/sbin/wpa_supplicant` 2.11 (nl80211, internal TLS, libnl-3.11) + `/sbin/wpa_cli` + `/sbin/wifi-join SSID PSK` (writes `/tmp/wpa_supplicant.conf` only, `-B -D nl80211`, `udhcpc` + `/usr/share/udhcpc/default.script`). **No PSK in image.** **Not at boot.** No `wpa_passphrase`. Tar SHA256 `dfd2fcafc898dced5ac4b90ea3e3ea6d14ebca63dd929bea277a67aa59dc819e`. |
| Test | Human flashed v031. Banner `SaaiOS v031`. Telnet `/sbin/wifi-join 'SSID' 'PSK'`. Do not run `usb-host`. |
| Acceptance | **LIVE join 2026-08-29.** v029 `/tmp` wpa then **v031** `/sbin/wifi-join`. Wallbox; `192.168.168.8`. E3 closed in the product ramdisk. |
| Rollback | `saaios-boot-v030.tar` or `saaios-boot-v029.tar` |

Do **not** auto-run `wifi-up`, `wifi-scan`, or `wifi-join` from init or `rcS`. Do **not** write EFS/RADIO. Do **not** start `wpa_supplicant` at boot. Do **not** embed any PSK.

### Phase E4 — modem map (read-only; CP OFFLINE)

**Depends on:** Phase E3 return point (v031 LIVE join). Stock DXJ2 Image — no maze. Detail: [modem.md](modem.md).

| | |
|--|--|
| Goal | Map CPIF / RADIO / how Android boots the Shannon CP, without writing EFS or RADIO |
| Current | **LIVE 2026-08-29 on v031.** Full TOC (TOC/BOOT/MAIN/VSS/NV). Vendor `cbd` on `/tmp` cannot execute. Static `/tmp/radio-boot` loaded BOOT+MAIN; CP **BOOTING**, `COMPLETE` timed out. Zero-NV retry did not reach ONLINE. **2026-08-31:** efs `mmcblk0p1` **ro,noload** list; `nv_data.bin` 1 MiB copied to userdata `saaios-efs-copy` (userdata **formatted**); `loadnv` with that copy still **BOOTING**; original efs never written; no `cbd`. No v032. |
| Change | Docs + `/tmp` helper only. No pack. No `rild`. No EFS mount |
| Test | Telnet: `hexdump -C -n 160` p22; wget `cbd` (expect no linker); `/tmp/radio-boot status` then `load`. Watch `modem_state`. Keep RNDIS |
| Acceptance | **TOC + BOOTING 2026-08-29.** All 5 names documented. CP left OFFLINE without EFS write. ONLINE blocked on real NV / vendor `cbd`+linker. |
| Rollback | n/a (no image change) |

Do **not** start `cbd`, `rild`, or `secril_config_svc`. Do **not** remount EFS RW or feed `cbd`. Do **not** `dd` to RADIO. EFS writes stay off-limits.

Phase A (TSP) is parked.

**RTL8188EUS** ([aircrack-ng/rtl8188eus](https://github.com/aircrack-ng/rtl8188eus)) is a **USB** 802.11n dongle driver (monitor/injection). It is **not** onboard `wlan0` and **not** the TD4150/synaptics tree. An 8188 stick on this phone needs USB **host**/OTG; wait until E2 host is LIVE with a stick, then see [wifi-usb.md](wifi-usb.md). R620 (2026-08-19) has no `0bda:8179` — no host-side clone.

---

## Risks

| Risk | What to do |
|------|------------|
| **Project-Xed** `LOCALVERSION` vs stock **DXJ2** | Image may boot and still be the wrong personality. If modules, firmware paths, or TSP probe diverge, rebuild from Samsung **DXJ6** zip (binary D) + the same patch |
| Stock **DTB** + new **Image** | Keep stock DTB/DTBO until a boot proves otherwise. Do not ship a new DTB in the same experiment as the TSP patch |
| **AVB** | Patched stock vbmeta (flags OR 3) with every AP tar. Rollback = stock `boot` **and** stock `vbmeta` |
| **Unbind TSP** | Never. Rebind already killed `event3` until reboot |
| Second `FBIOBLANK` | Already broke resume once (v010). Unblank once until C is a dedicated boot |
| Factory `cmd` `0x2a` | `check_connection` / `sensitivity_mode` time out. Do not retry as a “wake” |
| `make flash` | **Refuses.** Human Odin AP only. Never BL/TZ/EFS |

---

## Flash rule (explicit)

```text
make flash          → refuse
make *-flash        → refuse
Odin AP             → human, BOOT + VBMETA only
Odin BL (full zip)  → never (sboot / TZ)
```

Pack stays: `--seandroid --pad-to 46137344`, Magisk-style vbmeta. Restore tar stays next to every experiment.
