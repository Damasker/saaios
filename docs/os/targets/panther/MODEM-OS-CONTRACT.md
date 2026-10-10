# Panther modem — OS contract (write the rest of SaaiOS)

**Date:** 2026-10-04. Evidence: MODEM-BLOCKER VERDICT 24–27.

Cellular is **not** a boot, shell, or application-platform dependency.
Wi‑Fi (`wlan0`) and USB-NCM remain the live network. Do not stall PID 1,
`saai-displayd`, `saai-shell`, `saai-appd`, or `saai-entityd` on `rmnet`,
SIT registration, or `saai-modemd`.

## What is proven (do not rediscover)

| Observation | Meaning |
|---|---|
| CP ONLINE, SIM READY(5), radio ON, stack GET `0x0810` enabled | Bring-up and SIT ownership work |
| Voice `REG_DENIED` `reject_cause=0`, PS not registered | Local MM/cell-selection gate, not a missing AllowData |
| Serving camp `25501` WCDMA only; mask UMTS-only | RF path for 3G works; LTE is not acquired |
| GET `0x0750` bitmap `0x403fe` (LTE+NR allowed on paper) | Host RAT “allow LTE” is not the missing bit |
| SET `0x074f` / `0x0705` → `error_raw=2` | Operator-control SETs refused CP-internally |
| `0x0734` ACK 0, `0x0736` status=2 len=13, 0 cells | Modern LTE scan accepted, completed empty |
| Same scan after radio OFF→ON, 48 ms COMPLETE | CP short-circuit; no EUTRAN sweep; scan axis closed |

Firmware and NV were not written. Bearer (`rmnet` IPv4 / rx+tx) was never
observed. **Do not mark cellular complete** without that evidence.

## What the OS must do now

1. **Boot without the modem.** PID 1 does not start diagnostic owners, does
   not open `/dev/umts_*`, does not mount EFS, does not wait for `modem_state`.
2. **Network UI = Wi‑Fi.** Status copy for no Wi‑Fi stays ADR-191 (`Нет сети`).
   Do not invent a cellular bars/operator row until `bearer_verified=yes`.
3. **Optional status only.** If `saai-modemd os-gate` is present, the shell
   may show a diagnostics line. Absence of the binary is success.
4. **Keep diagnostics opt-in.** `os/targets/panther/diagnostics/` stays out of
   `build-native-c-image.sh`.

## `saai-modemd os-gate` (host-safe, no modem I/O)

```sh
cargo run -p saai-modemd -- os-gate
```

Machine-readable lines (stable keys):

```
continue_os=yes
cellular_service=unavailable
network_authority=wlan0
reason=cp_rf_cell_selection_wall
scan_axis=closed
host_registration_levers=exhausted
bearer_verified=no
hardware_actions=none
```

`continue_os=yes` is the only signal PID 1 / shell need.

## Closed research axes (do not repeat as OS work)

- RFS read-divert of gate NV (CP never RfsReads the gate at boot)
- PIN / CardPower / OEM catalog `0x2f50`
- Legacy `0x0706` scans and repeating `0x0734` from radio-off or camped
- Android RIL/framework as the missing piece
- Treating `0x070a` LTE_ONLY ACK as LTE RF enablement
- SetupDataCall / DIAL / SMS before CS∈{1,5} and PS∈{1,5}

## What may resume later (not on the OS critical path)

Constraint-safe, operator-gated, still **not** PID 1:

- Productize a **read-only** SIT poller for diagnostics (SIM/radio/reg scalars
  only; no PLMN payload in logs).
- Quarantine-copy `nv_normal` capture remains a policy experiment, not an OS
  feature. Real EFS/NV write stays forbidden without explicit operator grant.
- `SetupDataCall` stays implemented-but-gated until registration 1/5.

## Shell / Spaces copy

Until `bearer_verified=yes`:

- Do not show operator name, bars, dBm, IMEI, or a “cellular available” chip.
- Wi‑Fi up = network. Wi‑Fi down = `Нет сети` (ADR-191). USB-NCM is bring-up,
  not user WAN.
- A future Spaces hardware row may say cellular is **unsupported on this
  build** (diagnostics only), never “searching…”.
