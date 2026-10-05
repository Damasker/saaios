#!/usr/bin/env python3
"""Search factory HAL for app_state / READY / pin DISABLED handling."""
from pathlib import Path

paths = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"),
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"),
]
needles = [
    b"RIL_APPSTATE_",
    b"APPSTATE_PIN",
    b"APPSTATE_READY",
    b"PINSTATE_DISABLED",
    b"ENABLED_NOT_VERIFIED",
    b"CheckAndAutoVerifyPin",
    b"DoAutoVerifyPin",
    b"CheckDisabledIccid",
    b"OnSimStatusChanged",
    b"OnGetSimStatusDone",
    b"BuildRilCardStatus",
    b"covertAppState",
    b"pin disabled",
    b"PIN disabled",
    b"App state",
    b"app state",
    b"SIM_PIN",
    b"SIM READY",
]
for p in paths:
    data = p.read_bytes()
    print("==", p.name, "size", len(data))
    for n in needles:
        start = 0
        hits = 0
        while hits < 8:
            j = data.find(n, start)
            if j < 0:
                break
            end = data.find(b"\0", j)
            if end < 0 or end - j > 120:
                end = j + min(80, len(data) - j)
            s = data[j:end]
            printable = "".join(chr(c) if 32 <= c < 127 else "." for c in s)
            print(f"  {n.decode(errors='replace')}: {j:#x} {printable}")
            start = j + 1
            hits += 1
