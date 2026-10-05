#!/usr/bin/env python3
"""Factory HAL: extract Build* symbols and Eng/Virtual/Oem related IDs."""
import re
import subprocess
from pathlib import Path

STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
LIB = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)
BASE = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-base.so"
)

pat = re.compile(
    r"(Eng|Virtual|Oem|Hook|Camp|Fake|Lab|TestMode|ImsTest|Desense|RadioNode|Force|Attach)",
    re.I,
)

for path in (STREAM, LIB, BASE):
    print(f"\n==== nm {path.name} ====")
    try:
        out = subprocess.check_output(
            ["aarch64-linux-gnu-nm", "-C", str(path)], text=True, errors="replace"
        )
    except Exception as e:
        print("nm fail", e)
        # fallback strings
        data = path.read_bytes()
        for m in re.finditer(rb"[_A-Za-z][_A-Za-z0-9]{4,80}", data):
            s = m.group().decode()
            if pat.search(s) and ("Build" in s or "SIT_" in s or "Protocol" in s):
                print(" ", s)
        continue
    for line in out.splitlines():
        if pat.search(line) and (
            "Build" in line or "Protocol" in line or "SIT_" in line or "Virtual" in line
        ):
            print(line)

# Also dump ProtocolMiscBuilder / Network all Build*
print("\n==== all interesting Build from stream ====")
try:
    out = subprocess.check_output(
        ["aarch64-linux-gnu-nm", "-C", str(STREAM)], text=True, errors="replace"
    )
    for line in out.splitlines():
        if "Build" in line and any(
            k in line
            for k in (
                "Eng",
                "Virtual",
                "Oem",
                "Hook",
                "Camp",
                "Fake",
                "Lab",
                "Test",
                "Desense",
                "RadioNode",
                "Force",
                "Emergency",
                "Emc",
                "AllowData",
                "SetupData",
                "Network",
            )
        ):
            print(line)
except Exception as e:
    print(e)
