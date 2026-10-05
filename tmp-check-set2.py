#!/usr/bin/env python3
from pathlib import Path

cands = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    Path("/mnt/c/Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]
MAIN_FILE_OFF = 0x16C10
MAIN_VA = 0x40010000
off = 0x14FB404
for p in cands:
    if not p.exists():
        print("missing", p)
        continue
    d = p.read_bytes()
    print(p.name, "size", len(d))
    b = d[off : off + 8]
    print(" @file", hex(off), b.hex(), list(b))
    va = MAIN_VA + (off - MAIN_FILE_OFF)
    print(" VA", hex(va))
    for alt in [0x414FA7F4, 0x414F47F4, 0x414FB404]:
        fo = alt - MAIN_VA + MAIN_FILE_OFF
        if 0 <= fo < len(d):
            print(" altVA", hex(alt), "file", hex(fo), d[fo : fo + 4].hex())
