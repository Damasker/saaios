#!/usr/bin/env python3
"""MAIN: MM/NAS attach vs SIM READY / Present; reject-cause 0."""
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
needles = [
    b"SIM not ready",
    b"SIM NOT READY",
    b"sim not ready",
    b"USIM not ready",
    b"APP_STATE",
    b"app_state",
    b"ATTACH_REQ",
    b"ATTACH REJECT",
    b"Attach Reject",
    b"MM_REG_REJECT",
    b"REG_DENIED",
    b"registration denied",
    b"CS REJECT",
    b"PS REJECT",
    b"NO SIM",
    b"SIM_ABSENT",
    b"WAIT_SIM",
    b"SIM_READY",
    b"START_STACK",
    b"Present",
    b"Not registered",
    b"CAUSE=0",
    b"reject cause",
    b"RejectCause",
]
print("=== MAIN strings ===")
for n in needles:
    c = img.count(n)
    j = img.find(n)
    extra = ""
    if j >= 0:
        end = img.find(b"\0", j)
        extra = img[j:min(j+80, end if end > j else j+80)]
        extra = extra.decode("latin1", "replace")
    print(f"  count={c:3d} {n.decode()!r} {extra!r}")

# nearby attach/sim ready combos
print("\n=== grep-ish snippets ===")
for n in [
    b"SIM is not ready",
    b"SIM isn't ready",
    b"Wait SIM Ready",
    b"waiting SIM",
    b"SIM READY CHECK",
    b"CheckSimReady",
    b"IsSimReady",
    b"GetSimStatus",
    b"MM_STATE",
    b"GMM_STATE",
    b"EMM_STATE",
    b"EMM_DEREGISTERED",
    b"MM_IDLE",
    b"IMSI UNKNOWN",
    b"ILLEGAL ME",
    b"#2 SIM",
]:
    j = img.find(n)
    print(f"  {n.decode()!r}: {'hit' if j>=0 else 'no'} count={img.count(n)}")
