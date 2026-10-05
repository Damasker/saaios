#!/usr/bin/env python3
import subprocess
from pathlib import Path

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
stream = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so")

needles = [
    b"GetSimStatus", b"BuildGetSimStatus", b"phone_id", b"PhoneId",
    b"GetPhoneId", b"mPhoneId", b"slotId", b"SlotId", b"logicalModem",
    b"LogicalModem", b"subscription", b"RilContext", b"umts_ipc",
    b"ipc0", b"ipc1", b"GetChannel", b"SitChannel", b"ModemId",
]
for name, p in (("libsitril", lib), ("sit-stream", stream)):
    data = p.read_bytes()
    print("==", name)
    for n in needles:
        start = 0
        hits = 0
        while hits < 4:
            j = data.find(n, start)
            if j < 0:
                break
            end = data.find(b"\0", j)
            if end < 0 or end - j > 120:
                end = j + min(80, len(data) - j)
            s = "".join(chr(c) if 32 <= c < 127 else "." for c in data[j:end])
            print(f"  {j:#x} {s}")
            start = j + 1
            hits += 1

print("== sit-stream symbols")
out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-sW", str(stream)], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("GetSimStatus", "SimStatus", "PhoneId", "SlotId", "Channel", "InitRequestHeader", "SetPhone")):
        if "Build" in line or "GetSim" in line or "PhoneId" in line or "InitRequest" in line or "Adapter" in line:
            print(line)

print("== libsitril SimStatus / PhoneId")
out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-sW", str(lib)], text=True, errors="replace")
for line in out.splitlines():
    if any(k in line for k in ("SimStatus", "GetPhoneId", "PhoneId", "GetSlotId", "DoGetSimStatus", "SimService")):
        if "UND" in line and "Build" not in line and "GetPhone" not in line and "DoGet" not in line:
            continue
        if any(k in line for k in ("SimStatus", "PhoneId", "SlotId", "DoGetSim")):
            print(line)
