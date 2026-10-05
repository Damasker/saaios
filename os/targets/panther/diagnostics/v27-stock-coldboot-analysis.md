# VERDICT 27 — Stock cold modem bring-up capture (panther, CP2A.260705.006)

Read-only lawful-interop observation of OUR OWN Pixel 7 running STOCK Android +
root (adb). Same SIM, same location, same modem firmware
(`g5300q-260317-260505-B-15346003`) as every SaaiOS VERDICT 1–26 run. No secrets
recorded here (no IMSI/ICCID/AID/PIN/NV/RF-cal/IMEI); PLMN/band/RAT/opcode names
and partition paths only. Companion ordered RIL trace:
`v27-stock-ril-sequence.txt`.

## PHASE 0 — DECISIVE ISOLATION: STOCK SUCCEEDS

`dumpsys telephony.registry` + `ip addr` + ping, stock, same SIM+location:

- Operator = **25503 (UA-KYIVSTAR)**, HOME, automatic selection.
- RAT = **LTE** (getRilVoiceRadioTechnology=14, getRilDataRadioTechnology=14),
  carrier aggregation active, bands observed **[1, 3, 7, 40]**, EARFCN 3025/499.
- **CS registrationState=HOME, PS registrationState=HOME, rejectCause=0**
  (both IN_SERVICE).
- **Data bearer up**: `rmnet1` IPv4 10.111.84.133/32 + IPv6; `ping 8.8.8.8` 0% loss
  (~86 ms). APN "internet" (25503) CONNECTED, fail cause NONE.

⇒ On the identical SIM, location, and modem firmware where every SaaiOS owner run
camps only on foreign 25501 WCDMA with CS DENIED, **STOCK reaches 25503 Kyivstar
LTE with a working data bearer.** This FALSIFIES the VERDICT 14–26 conclusion that
the blocker is a CP-internal / environmental / RF-cal wall. The blocker is a
**host-side bring-up gap**: our host stack is not provisioning the CP the way
stock does.

## PHASE 1 — STOCK COLD-BOOT CP SEQUENCE (dmesg `cpif`)

```
umts_boot0 opened by: vendor.google.r, shared_modem_pl, wfc-pkt-router, dmd,
                      rfsd, cbd, modem_logging_c, rild_exynos
IOCTL_HANDOVER_BLOCK_INFO (sku:1 rev:0)
IOCTL_POWER_ON
IOCTL_START_CP_BOOTLOADER  -> normal boot mode -> start_normal_boot
  change_modem_state: OFFLINE -> BOOTING
  link_start_normal_boot: PCIE magic == 0x0000BDBD
  set_cp_rom_boot_img: boot_img addr:0xEA410000 size:0x16800
  check_cp_status: boot_stage 0xFF -> 0x3FFF
IOCTL_COMPLETE_NORMAL_BOOTUP -> complete_normal_boot
cmd_init_start_handler:  INIT_START  <- s5300
cmd_phone_start_handler: PHONE_START <- s5300
  change_modem_state: BOOTING -> ONLINE
```

`GET_RADIO_CAPABILITY` -> `mRadioAccessFamily=906119` (0xDD387), modem advertises
full multi-RAT incl. LTE. Host desired allowed set
`AllowedNetworkTypeValue=588799` (0x8FBFF, LTE+NR+legacy).

## PHASE 1 — RFS PROVISIONING (the load-bearing difference)

Open file descriptors of the stock daemons (paths only):

- **`rfsd` (RFS daemon)** holds: `/dev/umts_rfs0`, `/dev/umts_boot0`,
  **`/mnt/vendor/efs/nv_normal.bin` (+ .tmp)** and
  **`/mnt/vendor/efs/nv_protected.bin` (+ .tmp)** — i.e. it serves the modem's RFS
  from BOTH NV images, read-write, with `.tmp` atomic-commit files, from the LIVE
  EFS (sda5, f2fs, mounted rw by stock).
- **`cbd` (CP boot daemon)** holds: `/dev/umts_boot0`,
  `/mnt/vendor/efs/nv_protected.bin`.

Real EFS NV sizes: `nv_normal.bin` = **524288 B**, `nv_protected.bin` = **524288 B**
(a `nv_protected.bak` also present).

## PHASE 2 — DIFF vs our owner (`modem-rfs-full-quarantine-owner.c`)

Our owner, by design/verdict history:
- Boots the CP via the custom `probe-handover-rfs-camp-combined` +
  `saaios-probe-b-modem.bin` path (not the stock `cbd`
  `IOCTL_START_CP_BOOTLOADER` normal-boot handshake).
- Serves the modem's RFS from a **read-only quarantine copy of `nv_protected`
  only** (`/data/saaios/var/efs-copy/nv_protected.bin`, 524288 B).
- Provides **no working `nv_normal` backing store**: in the proven build the
  handle-1 (nv_normal) grant-requests are not answered at all, and even the
  VERDICT-24 `SAAIOS_RFS_NORMAL_CAPTURE` experiment only *captured the write* to a
  throwaway quarantine file — the modem was never given a valid `nv_normal` image
  to **read at boot**.

Handle map (consistent with VERDICTs 22–24): handle=3 = `nv_protected`
(RF-cal/IMEI/security), handle=1 = `nv_normal` (mutable settings incl.
allowed-RAT / band / selection).

### Why 0x074f fails for us but succeeds for stock
At cold boot stock issues `SET_ALLOWED_NETWORK_TYPES_BITMAP` (= our SIT `0x074f`)
and it returns **SUCCESS** (log `[0257]< SET_ALLOWED_NETWORK_TYPES_BITMAP`, 26 ms,
no error). Our owner's byte-identical `0x074f` returns `GENERIC_FAILURE`
(`error_raw=2`) in every verdict. The SIT bytes are not the problem (we matched
the stock builders and even got `0x0734` accepted). The difference is the CP's NV
state. Stock's `rfsd` maintains `nv_normal.bin` as an authoritative, host-backed,
read-write store (with `.tmp` atomic commit), so the modem's mutable
RAT/band/selection NV is anchored to a valid 524288 B image and the allowed-types
write persists. Our owner serves only `nv_protected` (read-only quarantine copy)
and provides **no persistent `nv_normal` store**: in the proven build the CP's
handle-1 (nv_normal) write-grant requests (`cmd=6`, observed in the owner's
`post_terminal_rfs_drain`) are never answered, and the VERDICT-24
`SAAIOS_RFS_NORMAL_CAPTURE` path only writes the CP's handle-1 output to a
throwaway `normal-candidate.bin` that is never fed back as the backing store. With
no valid host-backed `nv_normal`, the modem's mutable RAT/band NV stays
uninitialized -> it degrades to the strongest legacy cell (foreign 25501 WCDMA) and
refuses the allowed-types SET. VERDICT 24 did not fix it because capturing the CP's
handle-1 *write-out* once is not the same as giving the modem a persistent,
consistent `nv_normal` backing store across the whole boot/registration window the
way stock `rfsd` does.

## RECOMMENDATION (concrete, constraint-safe)

The missing host-side step is **complete RFS provisioning from BOTH NV images**,
mirroring stock `rfsd`:

1. Seed the quarantine with a **complete, current `nv_normal.bin` (524288 B)** copy
   in addition to `nv_protected.bin`.
2. Have the owner **serve handle-1 (nv_normal) reads from that copy at boot**, and
   serve handle-1 writes to the writable quarantine copy at runtime (the
   `SAAIOS_RFS_NORMAL_CAPTURE` write path already exists; it must be paired with a
   valid *read* backing image and kept persistent, not throwaway).
3. Keep handle-3 (`nv_protected`) served from the read-only quarantine copy as now.

This stays within AGENTS.md (the real EFS on sda5 is never written; both NV images
are served to the modem from quarantine copies only). Expected result: with a valid
`nv_normal` backing store the modem initializes LTE RAT/band config, accepts
`0x074f`, and can select 25503 LTE — the step stock performs that our owner omits.

Secondary (lower priority): align the CP boot with stock's `cbd`
`IOCTL_START_CP_BOOTLOADER` normal-boot handshake if the `nv_normal` fix alone is
insufficient.
