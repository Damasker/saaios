#!/usr/bin/env python3
import subprocess

stream = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"

ranges = [
    ("BuildSimSetLogicalSlotMapping", stream, 0x7D500, 0x7D5E0),
    ("BuildSimSetLogicalSlotPortMapping", stream, 0x7D5E0, 0x7D6D0),
    ("GetLogicalSlotId", stream, 0x63960, 0x63A40),
    ("GetLogicalSlotId_ii", stream, 0x639C0, 0x63A40),
    ("GetPortState", stream, 0x63D70, 0x63DF0),
    ("SetSlotMapping", lib, 0x12EE10, 0x12F420),
    ("SimSlotMappingHandler_OnRequest", lib, 0x1A2D00, 0x1A3040),
]
for name, path, start, stop in ranges:
    print("===", name)
    print(subprocess.check_output(
        ["aarch64-linux-gnu-objdump", "-d",
         f"--start-address={start:#x}", f"--stop-address={stop:#x}", path],
        text=True, errors="replace"))
