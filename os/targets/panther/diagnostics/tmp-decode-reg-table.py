#!/usr/bin/env python3
"""Decode GetRegState jump table: raw SIT byte12 -> RIL."""
from pathlib import Path
import subprocess

so = Path("sit-stream.so")
# Find file offset of VA 0x2a4a0 via readelf -S .rodata / .text
out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-S", str(so)], text=True)
text_addr = text_off = None
for line in out.splitlines():
    if ".text" in line and "PROGBITS" in line:
        parts = line.split()
        # [14] .text PROGBITS addr off
        # format varies; next line has size
print(out)

# parse sections
secs = {}
lines = out.splitlines()
for i, line in enumerate(lines):
    if "PROGBITS" in line:
        parts = line.split()
        name = parts[1] if parts[0].startswith("[") else parts[0]
        # try: [Nr] Name Type Address Offset
        try:
            idx = parts.index("PROGBITS")
            name = parts[idx - 1]
            addr = int(parts[idx + 1], 16)
            off = int(parts[idx + 2], 16)
            secs[name] = (addr, off)
        except Exception:
            pass
print("SECS", {k: (hex(a), hex(o)) for k, (a, o) in secs.items()})

data = so.read_bytes()
va = 0x2A4A0
# find section containing va
file_off = None
for name, (addr, off) in secs.items():
    # size from next - skip
    pass

# brute: ELF PT_LOAD
out = subprocess.check_output(["aarch64-linux-gnu-readelf", "-l", str(so)], text=True)
print(out)
for line in out.splitlines():
    if "LOAD" in line and "R E" in line or (line.strip().startswith("LOAD") and "R E" in line):
        print("LOADLINE", line)
    parts = line.split()
    if parts and parts[0] == "LOAD":
        # LOAD offset virtaddr physaddr
        print(parts)
