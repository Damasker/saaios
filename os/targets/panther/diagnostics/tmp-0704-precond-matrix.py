#!/usr/bin/env python3
"""Finalize 0x0704 precondition table vs live state + SIT-satiability."""
from __future__ import annotations

# From MAIN B START_NETWORK @0x18e8028 RE (tmp-re-0704-handler5.out)
# and SIT registration of opcode 0x0704 @0x1150b14 → SIT_REG 0x20d1afa.
# Live 0x0704 under PIN returned RCM_E_GENERIC_FAILURE (error_raw=2).
# Error 2 also documented for "already auto" (0x0703 mode==0).

PRECONDS = [
    {
        "id": "P1_radio_on",
        "where": "SIT NET path / live prerequisite",
        "check": "radio_state ON (0x0801==10)",
        "live": "OK (10)",
        "blocks_0704_err2": False,
        "sit_satiable": True,
        "sit_note": "0x0800 ON already proven; live already ON",
        "banned": False,
    },
    {
        "id": "P2_selection_not_already_auto",
        "where": "SIT SetNetworkSelectionAuto completer (host+prior live)",
        "check": "selection_mode!=0 else RCM_E_GENERIC_FAILURE(2) / skip",
        "live": "UNKNOWN this turn (no 0x0703 cmd deployed; prefer query-only)",
        "blocks_0704_err2": "possible",
        "sit_satiable": False,
        "sit_note": "If already auto, 0x0704 err=2 is expected no-op — not a SIM unlock",
        "banned": False,
    },
    {
        "id": "P3_get_app_start_gate",
        "where": "START_NETWORK 0x18e831a..0x18e8330",
        "check": "GET_APP ∈ {1=DETECTED, 4=PERSO, 5=READY}; else Ignored SIM not ready → camp skip",
        "live": "BLOCKED app=PIN(2)",
        "blocks_0704_err2": True,
        "sit_satiable": False,
        "sit_note": "READY needs Present=+0xBF6==2 (FN_A/CDMA only). DETECTED not holdable. No signed SIT invents GET_APP∈{1,4,5} under bans",
        "banned": "Present=2/FN_A/EFS/rild required for READY",
    },
    {
        "id": "P4_early_get_app_perso",
        "where": "START_NETWORK 0x18e8072 CMP #4",
        "check": "GET_APP==PERSO(4) takes alternate path",
        "live": "N/A (app=PIN)",
        "blocks_0704_err2": False,
        "sit_satiable": False,
        "sit_note": "No signed perso→READY path without Present=2",
        "banned": False,
    },
    {
        "id": "P5_pin_branch",
        "where": "START_NETWORK 0x18e81e0 GET_APP CMP #2 then LDRB +0xBF5 CMP #4",
        "check": "PIN(2) special branch (Pin1V/+0xBF5); does NOT accept PIN as start-ready",
        "live": "active (PIN) but does not open main gate P3",
        "blocks_0704_err2": "indirect",
        "sit_satiable": False,
        "sit_note": "VerifyPin already done (pin1=2); does not change GET_APP",
        "banned": False,
    },
    {
        "id": "P6_ldrb_c3",
        "where": "START_NETWORK 0x18e80c4 LDRB [r7,#0xc3] CMP #1",
        "check": "internal flag ==1",
        "live": "unknown (not on SIT wire)",
        "blocks_0704_err2": "unknown",
        "sit_satiable": False,
        "sit_note": "No identified signed SIT writer for this flag",
        "banned": False,
    },
    {
        "id": "P7_ldrb_554",
        "where": "START_NETWORK 0x18e813e LDRB +0x554 CMP #1/#13",
        "check": "internal state machine nibble",
        "live": "unknown",
        "blocks_0704_err2": "unknown",
        "sit_satiable": False,
        "sit_note": "No signed SIT mapping found",
        "banned": False,
    },
    {
        "id": "P8_present_bf6",
        "where": "SET_APP#5 only (NOT in START_NETWORK CMP list)",
        "check": "Present +0xBF6==2 for READY",
        "live": "notin_1_2_3 (inferred)",
        "blocks_0704_err2": False,
        "sit_satiable": False,
        "sit_note": "Irrelevant to 0x0704 accept; blocks later READY/bearer",
        "banned": "FN_A/CDMA only",
    },
    {
        "id": "P9_pin1_verified",
        "where": "VerifyPin path; not START_NETWORK allow-list",
        "check": "pin1==2",
        "live": "OK (2)",
        "blocks_0704_err2": False,
        "sit_satiable": True,
        "sit_note": "Already satisfied via 0x0201 A+AID",
        "banned": False,
    },
    {
        "id": "P10_card_present",
        "where": "SIM layer",
        "check": "card PRESENT",
        "live": "OK",
        "blocks_0704_err2": False,
        "sit_satiable": True,
        "sit_note": "Already PRESENT",
        "banned": False,
    },
    {
        "id": "P11_preferred_lte",
        "where": "SIT 0x070a (often paired before 0x0704)",
        "check": "preferred LTE_ONLY=11",
        "live": "OK (11)",
        "blocks_0704_err2": False,
        "sit_satiable": True,
        "sit_note": "Already set; err0",
        "banned": False,
    },
]

print("=== 0x0704 / START_NETWORK precondition matrix ===")
satiable_unmet = []
for p in PRECONDS:
    print(
        f"{p['id']}: live={p['live']} | sit_satiable={p['sit_satiable']} | "
        f"blocks_err2={p['blocks_0704_err2']} | {p['sit_note']}"
    )
    if p["sit_satiable"] and "OK" not in str(p["live"]) and p["live"] not in ("OK", "OK (10)", "OK (2)", "OK (11)"):
        satiable_unmet.append(p)

print("\n=== satiable-unmet (would justify ONE live SIT seq) ===")
if not satiable_unmet:
    print("NONE — all SIT-satiable preconditions already met OR unmet ones are NOT SIT-satiable under bans")
else:
    for p in satiable_unmet:
        print(p)

print(
    "\nLIVE TRY DECISION: NO — hard blocker is P3 GET_APP∈{1,4,5}; "
    "no remaining signed SIT under bans can flip PIN→{1,4,5}. "
    "Do not re-spam 0x0704 under PIN."
)
print("DONE")
