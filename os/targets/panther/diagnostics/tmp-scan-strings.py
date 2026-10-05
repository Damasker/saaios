#!/usr/bin/env python3
from pathlib import Path
data = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so").read_bytes()
idx = 0
n = 0
while n < 40:
    i = data.find(b"band", idx)
    if i < 0:
        i = data.find(b"Band", idx)
    if i < 0:
        break
    start = data.rfind(b"\x00", max(0, i - 120), i) + 1
    end = data.find(b"\x00", i)
    s = data[start:end]
    if 8 < len(s) < 160 and all(32 <= c < 127 for c in s) and (b"scan" in s.lower() or b"Band" in s or b"EUTRAN" in s or b"specifier" in s):
        print(f"0x{start:x} {s.decode()}")
        n += 1
    idx = i + 4
print("count", n)
