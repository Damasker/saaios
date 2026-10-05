#!/usr/bin/env python3
import subprocess

stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"

# Find BuildSimSetLogicalSlotMapping and PortMapping addresses
out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-sW", stream], text=True, errors="replace")
for line in out.splitlines():
    if "LogicalSlot" in line or "ModemsConfig" in line or "PreferredDataModem" in line or "GetSlotStatus" in line:
        print("stream", line)

out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-sW", lib], text=True, errors="replace")
for line in out.splitlines():
    if "SlotMapping" in line or "SetSlotMapping" in line or "LogicalSlot" in line or "SlotPort" in line or "PreferredDataModem" in line:
        if "UND" not in line or True:
            if any(x in line for x in ("SlotMapping", "SetSlot", "LogicalSlot", "SlotPort", "PreferredData", "CreateSlot")):
                print("lib", line)
