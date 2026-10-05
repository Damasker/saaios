#!/usr/bin/env python3
import subprocess
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"

# SimSlotMappingHandler OnRequest - how it chooses Mapping vs PortMapping
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x1a2d00","--stop-address=0x1a3040", lib
], text=True, errors="replace"))

# Preferred data modem builder
print("=== PreferredDataModem ===")
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x7b010","--stop-address=0x7b0a0", stream
], text=True, errors="replace"))

# SetPreferredDataModemHandler OnRequest
print("=== SetPreferredDataModem OnRequest ===")
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x19c3d0","--stop-address=0x19c5b0", lib
], text=True, errors="replace"))

# strings around slotmap empty / invalid
data = open(lib,"rb").read()
for n in (b"slotmap.config is empty", b"slotmap.config invalid", b"SetSlotMapping", b"LogicalSlotMapping", b"SlotPortMapping"):
    j = data.find(n)
    if j>=0:
        print(hex(j), data[j:j+80])
