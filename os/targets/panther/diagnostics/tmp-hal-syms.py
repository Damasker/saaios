#!/usr/bin/env python3
import subprocess
p = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
out = subprocess.check_output(
    ["aarch64-linux-gnu-readelf", "-sW", p],
    text=True,
    errors="replace",
)
needles = (
    "CheckAndAuto",
    "OnSimStatus",
    "BuildRilCard",
    "CardStatus",
    "AppState",
    "AutoVerify",
    "SimStatusChanged",
    "PinState",
    "perso",
    "DoAutoVerify",
    "GetApplicationState",
    "covertApp",
)
for line in out.splitlines():
    if any(n.lower() in line.lower() for n in needles):
        print(line)
