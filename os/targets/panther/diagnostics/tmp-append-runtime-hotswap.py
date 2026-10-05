#!/usr/bin/env python3
from pathlib import Path

p = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md"
)
text = p.read_text(encoding="utf-8")
marker = "## 2026-09-30: present_infer validation + HotSwap hard blocker"
if marker in text:
    print("already present")
else:
    add = """

## 2026-09-30: present_infer validation + HotSwap hard blocker

**Live one-liner:** ONLINE (~uptime 12.7ks); soft-lock app=PIN(2) pin1=2
`present_infer=notin_1_2_3`; reg=0; rmnet* rx=0 tx residual; **no IPv4**.
`tray-bearer-chase` PERSIST alive batch=1 round~8/12 (observability only).

### present_infer vs GET_APP / MAIN +0xBF6 — **valid**

| Check | Result |
| --- | --- |
| `0x0200` offsets | card@12 apps@14 type@15 **app@17** pin1@72 remain@74 (factory HAL / `note_sim`) |
| GET_APP `0x18ec8c0` | sole `LDRB.W [obj,#0xBF4]` @`0x18ec8ec` — byte17 mirrors **app_state**, not Present |
| +0xBF6 on wire? | **never** — Present only via STATUS STRB @`0x14fb380` then SET_APP |
| Inference table | Present 0→PIN(#2)=`notin_1_2_3`; 1→PUK=`was_1`; 2→READY=`was_2`; 3→PERSO=`was_3` |
| SET#5 | still sole @`0x14fb5c6` after `LDRB +0xBF6` **CMP #2** |
| STRB +0xBF6 | still sole @`0x14fb380` |
| STATUS_WRAP callers | still exactly 2 (`0x14b7ca8` SADR_MEASURE_RSP, `0x1a3e6cc` L1TUNNEL) |
| Tool bug? | **No** — HotSwap landed PIN with Present=0 decision; CP does not secretly hold Present=2 while publishing PIN under EU No-CDMA |

Script: `os/targets/panther/diagnostics/tmp-validate-present-infer.py`.

Caveat (documented, not a parse bug): while GET_APP==PIN, STATUS head
CMP#1/#4 skips Present re-eval. Infer = last published STATUS→SET decision.
EU FN_A never writes Present=2, so sticky-PIN cannot hide Present=2 under bans.

### Bypass hunt — none under bans

| Candidate | Verdict |
| --- | --- |
| SET#5 skip Present==2 | **no** — sole site gated CMP #2 |
| Alternate STATUS entry / wrong object | **no** — sole STRB +0xBF6; getobj#0x10 = L1LC not Present |
| L1 SADR_MEASURE_RSP inject | **no signed SIT**; STATUS_WRAP only from L1; needs camp (denied while PIN) |
| HotSwap / SET#1 → Present=2 | RO DBT-only; **live falsified** |
| FN_A without CDMA | EU RatMap No-CDMA; EFS TCS banned |

**Live try this turn:** **none** (no strong signed evidence after HotSwap falsifier).

### Hard blocker

EU No-CDMA + Present=2 FN_A-only + HotSwap insufficient.
Remaining options only: **EFS TCS** (banned), **unsigned MAIN** (rejected),
**stock rild** (banned). Post-READY CPIF (SetupDataCall/GetDataCallList/APN)
already on device — unreachable until READY.

**Bearer verified?** no. Goal incomplete. See MODEM-BLOCKER.
"""
    p.write_text(text.rstrip() + add, encoding="utf-8")
    print("appended", len(add), "chars")
