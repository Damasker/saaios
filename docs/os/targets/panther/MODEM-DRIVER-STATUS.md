# SaaiOS host-side modem driver — implementation-status checklist

Scope: lawful interoperability engineering per repo `AGENTS.md`. This is a static
source/doc audit of the **host (AP) side** RIL/SIT driver for the owner's own
Pixel 7 (panther) talking to the **stock, unmodified** modem firmware over the
documented SIT/RIL interface. No firmware is modified, no auth is bypassed, no
vulnerability is used. This file only catalogs which host-side RIL request
handlers our driver already implements versus which remain, so we can finish the
driver and get cellular data/voice/SMS on SaaiOS.

Primary owner source: `os/targets/panther/diagnostics/modem-rfs-full-quarantine-owner.c`
(plus the one-shot probes beside it). On-device evidence of record: the VERDICT
log in `docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md` and the one-pager
`docs/os/targets/panther/MODEM-BLOCKER.md` (VERDICT 11–19).

Status legend:
- **DONE+verified** — implemented and exercised live on-device (frame ACKed and/or
  response decoded in a VERDICT run).
- **impl-unverified** — implemented and self-tested byte-exact, but never exercised
  live because an upstream gate (CS/PS registration, PIN-enabled card) was never
  reached.
- **partial** — implemented and exercised, but the modem refused it or it had no
  effect; or only one half of a modern/legacy pair is present.
- **not-impl** — not yet implemented in the owner.

---

## 1. Status by RIL request category

### SIM / card readiness
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| SIM_GET_STATUS (card/app/PIN state) | `0x0200` | **DONE+verified** | VERDICT 16/18/19: `card_raw/apps/app_state_raw/pin1_raw` decoded live; empty-tray control confirmed reader works |
| SIM PIN1 verify | `0x0201` (len 38) | **impl-unverified** | VERDICT 16: recovered byte-exact from stock `libsitril.so`, self-test RC=0; never fired (cards report PIN disabled) |
| CardPower (card reseat) | `0x024c` | **partial** | referenced in runs; `AGENTS.md` flags CardPower as spam-risk — treated as a boundary, not a routine lever |

### Network registration & selection
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| Voice/CS reg state GET | `0x0700` | **DONE+verified** | VERDICT 12/13: full field decode (reg state, reject, RAT, LAC, CID, PSC) |
| Data/PS reg state GET | `0x0701` | **DONE+verified** | VERDICT 12/13: decoded; PS NOT_SEARCHING on 25501 |
| GET_OPERATOR (serving PLMN) | `0x0702` | **DONE+verified** | VERDICT 13: serving PLMN=25501 (foreign Vodafone-UA) |
| Selection-mode GET | `0x0703` | **DONE+verified** | exercised in bring-up GET set |
| Auto network-selection SET | `0x0704` | **DONE+verified** | ACKed error 0 in multiple runs |
| Radio-state GET | `0x0801` | **DONE+verified** | round-robin GET, radio=10/ON |
| RADIO_POWER OFF/ON | `0x0800` | **DONE+verified** | VERDICT 15: OFF/ON both ACK error 0, `radio_off_confirmed=1`, reversible |
| GetPsService | `0x0711` | **impl-unverified** | one-shot probe `get-ps-service.c`, empty GET |
| SET_NETWORK_SELECTION_MANUAL | `0x0705` | **partial** | VERDICT 14: recovered byte-exact, exercised → RIL_E_GENERIC_FAILURE |
| GET_AVAILABLE_NETWORKS (legacy scan, len 12 + len 16 scanType) | `0x0706` / cancel `0x0707` | **partial (deprecated)** | VERDICT 14–17: both frame forms, all scanType 0–5, all radio states → GENERIC_FAILURE |
| Modern scan: StartNetworkScan + SetSystemSelectionChannels | (to recover) | **not-impl** | builders seen in stock but RAS payload not recovered; this is the live-scan path a modern HAL uses |

### RAT / band selection
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| SetPreferredNetworkType (legacy) GET/SET | `0x070b` / `0x070a` | **partial** | VERDICT 13: SET LTE_ONLY `0x0b` ACKed error 0 but CP acquired **zero LTE** cells — accepted yet ineffective |
| SetAllowedNetworkTypeBitmap (modern) | `0x074f` | **not-impl** | **KEY GAP** (see §2): we send only the legacy preferred-type; the modern bitmap command is never sent |

### Packet-data / bearer (PDN → rmnet)
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| SET_INITIAL_ATTACH_APN | `0x0603` (len 250) | **DONE+verified** | VERDICT 11: recovered + replayed, ACK error 0 (not the gate) |
| AllowData | `0x0710` (len 13) | **DONE+verified** | ACK error 0 in multiple runs |
| SetupDataCall | `0x0600` (len 246) | **impl-unverified** | `setup-data-call.c` recovered; never reached (no PS registration) |
| GetDataCallList | `0x0602` | **impl-unverified** | `get-data-call-list.c` recovered |
| rmnet bearer up (rx/tx, IPv4) | — (end goal) | **not-impl** | no bearer ever established; blocked upstream at registration |

### Voice call control
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| DIAL | `0x0001` (len 104) | **impl-unverified** | VERDICT 16: recovered byte-exact, self-test RC=0; gated on CS reg ∈ {home,roaming} which never occurred |
| GET_CALL_LIST | `0x0000` (len 12) | **impl-unverified** | VERDICT 16: recovered, self-tested |
| HANGUP | `0x0008` (len 20) | **impl-unverified** | VERDICT 16: recovered, self-tested |

### SMS
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| Send/receive SMS (SMS_INIT, submit, deliver, ack) | `0x04xx` (to recover) | **not-impl** | no SMS opcode recovered or implemented in the owner |

### Signal / network info
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| GetSignalStrength | `0x0900` (len 12) | **DONE+verified** | decoded live; also unsol `0x0900` observed |

### Device / config
| Request | SIT opcode | Status | Evidence |
|---|---|---|---|
| Camp bring-up trio (seq-config, SGC, radio power) | `0x093f` / `0x0404` / `0x0800` | **DONE+verified** | stage-1 camp sequence ACKed each run (CP OFFLINE→ONLINE) |
| OPX service toggles (stack/voice/intps/devsvc/dual) | `0x0810/0x080f`, `0x091b/0x091a`, `0x0933`, `0x0957/0x0956`, `0x072b` | **impl-unverified** | recovered from proven wire IDs; experimental, not a verified bearer lever |

---

## Status counts

| Status | Count | Items |
|---|---|---|
| **DONE+verified** | 12 | `0x0200`, `0x0700`, `0x0701`, `0x0702`, `0x0703`, `0x0704`, `0x0801`, `0x0800`, `0x0603`, `0x0710`, `0x0900`, camp trio `0x093f/0x0404` |
| **impl-unverified** | 8 | `0x0201`, `0x0711`, `0x0600`, `0x0602`, `0x0001`, `0x0000`, `0x0008`, OPX toggles |
| **partial** | 4 | `0x024c`, `0x0705`, `0x0706/0x0707`, `0x070a/0x070b` |
| **not-impl** | 4 | `0x074f`, modern StartNetworkScan + SetSystemSelectionChannels, SMS `0x04xx`, rmnet bearer end-goal |

The CS/PS GET+status plumbing is solid and verified; the whole voice/SMS/data
**action** layer is either implemented-but-gated behind registration, or not yet
implemented. Nothing has produced a bearer, because registration itself never
succeeds here.

---

## 2. Near-term gaps that block connectivity

The VERDICT log concluded "terminal environmental/firmware boundary," but VERDICT 19
falsified the *environmental* reading: an iPhone at the same spot sees all three
UA home operators strongly, while our Pixel camps only on foreign Vodafone-UA 25501
3G, acquires no LTE, and refuses every scan. That points the finger back at **our
host-side bring-up**, and the parallel RF finding names the concrete defect:

- **RAT selection uses the legacy command only.** We send `SetPreferredNetworkType`
  (`0x070a`) and *not* the modern `SetAllowedNetworkTypeBitmap` (`0x074f`). On a
  modern modem the allowed-network-type **bitmap** is the authoritative RAT
  enablement; the legacy preferred-type is advisory/deprecated. This matches the
  observed symptom exactly — `0x070a` set LTE_ONLY was ACKed (error 0) yet the CP
  acquired zero LTE cells (VERDICT 13). We may be ACKing a no-op.
- **Scan uses the deprecated command only.** The legacy available-networks scan
  `0x0706` is deprecated and was refused in every form/state (VERDICT 14–17). The
  modern path is `StartNetworkScan` + `SetSystemSelectionChannels`; neither is
  implemented, and the `StartNetworkScan` RAS payload has not been recovered.

Net: before declaring any boundary terminal, the driver must drive RAT and scan
through the **modern** opcodes. Both missing commands are `not-impl` today and are
the top of Tier 1.

(The RF analysis itself is owned by the parallel finding and is not re-derived here;
only its conclusion is folded in.)

---

## 3. Prioritized implementation plan

All items are host-side, read-mostly, one-shot, self-tested, and must respect the
`AGENTS.md` hard constraints — **no** NV / EFS-RW / `nv_protected` / `sda5` / SIM-EF
writes, **no** RF-cal/firmware writes, **no** `IOCTL_POWER_OFF` / `do_cp_crash`, **no**
invented opcodes/bodies (recover byte-exact from stock), and no CardPower / EngMode /
`0x0704` / reboot spam. These are boundaries, not tasks. Never log IMSI/ICCID/PIN/AID/
IMEI/keys.

### Tier 1 — connectivity (unblock registration → bearer)
1. **SetAllowedNetworkTypeBitmap `0x074f`** *(not-impl → highest priority)*
   - Recover: body layout + bitmap field from stock `sit-stream.so` /
     `libsitril.so` `BuildSetAllowedNetworkTypeBitmap` (and its GET pair if present).
   - Implement: owner sends the modern bitmap (e.g. LTE+NR+WCDMA) in place of /
     alongside the legacy `0x070a`, guarded + self-tested byte-exact.
   - Verify: ACK error 0, then watch `0x0700/0x0701` for a RAT change and any LTE(14)
     sample in the settle window — the thing `0x070a` never produced.
2. **Modern StartNetworkScan + SetSystemSelectionChannels** *(not-impl)*
   - Recover: the `StartNetworkScan` RAS/scan-request payload and
     `SetSystemSelectionChannels` body from stock (the RAS payload is the piece
     currently missing; `0x0706` empty-safe form is deprecated/refused).
   - Implement: reuse the reviewed one-shot/cancel state model in
     `sit-network-scan-host.*` (already fail-closed with `0x0707` cancel) behind the
     existing explicit-opt-in RF gate.
   - Verify: a bounded available-networks count returned (no PLMN/identifier logged),
     proving the modem will scan when driven via the modern path.
3. **SetupDataCall `0x0600` end-to-end** *(impl-unverified → verify once registered)*
   - Recover: already recovered (`setup-data-call.c`), re-confirm 246-byte body.
   - Implement: wire `0x0603` attach-APN → `0x0710` AllowData → `0x0600` SetupDataCall →
     `0x0602` GetDataCallList as one ordered sequence in the owner.
   - Verify (bearer gate per `AGENTS.md`): rmnet rx/tx nonzero and/or IPv4 on rmnet —
     do **not** mark connectivity done without this evidence.

### Tier 2 — voice / SMS
4. **Voice call (DIAL `0x0001` / GET_CALL_LIST `0x0000` / HANGUP `0x0008`)** *(impl-unverified)*
   - Recover: already recovered byte-exact (VERDICT 16).
   - Implement: already in owner, gated on voice reg ∈ {1 home, 5 roaming}.
   - Verify: once Tier 1 yields CS registration, place one owner-initiated call and
     confirm via `0x0000` call list, then `0x0008` hangup.
5. **SMS (`0x04xx`)** *(not-impl)*
   - Recover: `SMS_INIT_REQ`, submit/deliver, and deliver-ack opcodes + bodies from
     stock `libsitril.so` / `sit-stream.so` builders (none recovered yet).
   - Implement: minimal submit + deliver-ack handlers, one-shot, self-tested.
   - Verify: after PS/CS registration, send one SMS and confirm a deliver-ack frame.

### Tier 3 — info / management
6. **Signal/info polling (`0x0900`, reg GETs)** *(DONE+verified → productize)*
   - Implement: fold the existing verified GETs into a steady single-outstanding
     round-robin poller for UI signal/registration display. No new opcodes.
   - Verify: stable periodic decode without GET spam.
7. **PS-service / data-call-list GETs (`0x0711`, `0x0602`)** *(impl-unverified)*
   - Verify: exercise live once registered to confirm response decode.
8. **OPX service toggles (`0x0810/0x080f`, `0x091b/0x091a`, `0x0933`, `0x0957/0x0956`, `0x072b`)** *(impl-unverified)*
   - Treat as diagnostic only until Tier 1 proves them relevant; keep behind
     explicit opt-in. Not on the connectivity critical path.

**Critical path:** item 1 (`0x074f`) then item 2 (modern scan) are the two changes
most likely to move the modem off the foreign 3G camp and let registration — and
therefore every gated action above — proceed.
