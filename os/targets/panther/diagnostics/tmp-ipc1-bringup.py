#!/usr/bin/env python3
import subprocess
from pathlib import Path

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
stream = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so")

needles = [
    b"EnableUicc", b"UiccAppsEnable", b"setUiccSubscription", b"UiccSubscription",
    b"SetDataAllowed", b"DataAllowed", b"SetRadioPower", b"RadioPower",
    b"CardPower", b"SetSimCardPower", b"BuildSimSet", b"BuildSetRadio",
    b"Subscription", b"ActivateUicc", b"EnableSim", b"PowerUp",
    b"SIM_POWER", b"CardPowerState", b"BuildSimPower",
]
for name, p in (("libsitril", lib), ("sit-stream", stream)):
    data = p.read_bytes()
    print("==", name)
    for n in needles:
        start = 0
        hits = 0
        while hits < 5:
            j = data.find(n, start)
            if j < 0:
                break
            end = data.find(b"\0", j)
            if end < 0 or end - j > 100:
                end = j + min(80, len(data) - j)
            s = "".join(chr(c) if 32 <= c < 127 else "." for c in data[j:end])
            print(f"  {j:#x} {s}")
            start = j + 1
            hits += 1

print("== stream builders")
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(stream)], text=True, errors="replace")
for line in out.splitlines():
    if "Build" in line and any(k in line for k in (
        "RadioPower", "Uicc", "CardPower", "DataAllowed", "Subscription",
        "SimPower", "Enable", "PowerUp", "PowerDown")):
        print(line)

print("== lib handlers")
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(lib)], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("RadioPower", "Uicc", "CardPower", "DataAllowed", "Subscription", "SimPower")):
        if "Handler" in line or "DoSet" in line or "DoGet" in line or "Build" in line:
            print(line)
