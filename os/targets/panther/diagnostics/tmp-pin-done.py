#!/usr/bin/env python3
"""Disassemble GetSimStatusDone / BuildSimStatus for post-PIN actions."""
import subprocess
lib = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
# Find addresses
out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",lib], text=True, errors="replace")
addrs = {}
for line in out.splitlines():
    for key in ("GetSimStatusDone", "BuildSimStatus", "IsEarlySimDetection", "DoGetSimStatus"):
        if key in line and "UND" not in line:
            parts = line.split()
            # format: Num: Value Size ... Name
            try:
                val = int(parts[1], 16)
                size = int(parts[2])
                addrs[key] = (val, size)
                print(line)
            except: pass

for key, (val, size) in addrs.items():
    stop = val + min(size, 0x200)
    print(f"=== {key} {val:#x}-{stop:#x} ===")
    dump = subprocess.check_output([
        "aarch64-linux-gnu-objdump","-d",
        f"--start-address={val:#x}", f"--stop-address={stop:#x}", lib
    ], text=True, errors="replace")
    # filter interesting: bl, cmp for pin states, mov of opcodes
    for line in dump.splitlines():
        if any(x in line for x in ("bl\t", "mov\tw", "cmp\tw", "strb", "0x20", "PIN", "Send")):
            if "bl\t" in line or "cmp\t" in line or "#0x" in line:
                print(line)
