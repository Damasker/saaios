#!/usr/bin/env python3
import subprocess
from pathlib import Path

needles = [
    b"SlotMap", b"slotmap", b"SimSlot", b"LogicalSlot", b"PhysicalSlot",
    b"setSimSlotMapping", b"SetSimSlot", b"SwitchSlot", b"PortInfo",
    b"ModemsConfig", b"DSDS", b"dsds", b"ActiveSlot", b"PrimarySlot",
    b"BuildSetModems", b"BuildGetSlot", b"BuildSetSlot", b"SlotStatus",
    b"setSignalStrength", b"SwitchTo", b"PhoneId", b"mapping",
    b"LOGICAL_TO_PHYSICAL", b"SetPreferredData", b"PreferredDataModem",
]
for name in ("libsitril.so", "sit-stream.so"):
    p = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics") / name
    data = p.read_bytes()
    print("==", name)
    for n in needles:
        start = 0
        hits = 0
        while hits < 6:
            j = data.find(n, start)
            if j < 0:
                break
            end = data.find(b"\0", j)
            if end < 0 or end - j > 140:
                end = j + min(100, len(data) - j)
            s = "".join(chr(c) if 32 <= c < 127 else "." for c in data[j:end])
            print(f"  {j:#x} {s}")
            start = j + 1
            hits += 1

print("== readelf libsitril")
out = subprocess.check_output(
    ["aarch64-linux-gnu-readelf", "-sW",
     "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"],
    text=True, errors="replace")
keys = ("Slot", "Port", "Modem", "DSDS", "Mapping", "Logical", "Physical", "Switch", "Config")
for line in out.splitlines():
    if any(k in line for k in keys) and ("Build" in line or "DoSet" in line or "DoGet" in line or "Handler" in line or "Mapping" in line or "Modems" in line):
        print(line)

print("== readelf sit-stream")
out = subprocess.check_output(
    ["aarch64-linux-gnu-readelf", "-sW",
     "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"],
    text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("Slot", "Port", "Modem", "Mapping", "Logical", "Physical", "Config", "DSDS")):
        if "Build" in line or "Adapter" in line or "fillSlot" in line or "SlotStatus" in line:
            print(line)
