#!/usr/bin/env python3
"""Find scan-status strings and GetScanStatus in TD1A libsitril.so."""
from pathlib import Path
SO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so")
data = SO.read_bytes()
needles = [b"PARTIAL", b"COMPLETE", b"scanStatus", b"ScanStatus", b"NETWORK_SCAN", b"incrementalResults"]
for n in needles:
    idx = 0
    hits = 0
    while hits < 8:
        i = data.find(n, idx)
        if i < 0:
            break
        start = data.rfind(b"\x00", 0, i) + 1
        end = data.find(b"\x00", i)
        s = data[start:end]
        if 0 < len(s) < 180 and all(32 <= c < 127 or c in (9,) for c in s):
            print(f"0x{i:x} {s.decode()}")
            hits += 1
        idx = i + 1
    if hits == 0:
        print(f"no {n!r}")
