#!/usr/bin/env python3
"""GET_APP callers with CMP #5 (READY) nearby — attach/start-network gates."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
GET_APP = 0x18EC8C0
SET_APP = 0x19916D2


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def bl(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32


cs = []
o = 0x1000000
while o < 0x3C00000 - 4:
    if bl(o) == GET_APP:
        cs.append(o)
    o += 2

print(f"GET_APP n={len(cs)}")
ready_cmp = []
for c in cs:
    cmp5 = cmp2 = bf6 = False
    for p in range(max(0, c - 0x60), min(c + 0x80, len(img) - 2), 2):
        hw = u16(p)
        hw2 = u16(p + 2) if p + 2 < len(img) else 0
        if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) == 5:
            cmp5 = True
        if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) == 2:
            cmp2 = True
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF6:
            bf6 = True
        # CMP.W r0,#5  F1B0 0F05
        if hw == 0xF1B0 and hw2 == 0x0F05:
            cmp5 = True
    if cmp5:
        ready_cmp.append((c, cmp2, bf6))
        print(f"  {hex(c)} GET_APP then/near CMP#5 cmp2={cmp2} bf6={bf6}")

print(f"GET_APP+CMP#5 n={len(ready_cmp)}")

# BL SET_APP already known; search START_NETWORK log nearby GET_APP
# Unique ASCII around GET_APP sites
for c, _, _ in ready_cmp:
    window = img[c : c + 0x200]
    for n in [b"START_NETWORK", b"SIM is not", b"not ready", b"DENIED"]:
        if n in window:
            print(f"  ascii {n} after {hex(c)}")
    windowb = img[max(0, c - 0x200) : c]
    for n in [b"START_NETWORK", b"SIM is not", b"not ready"]:
        if n in windowb:
            print(f"  ascii {n} before {hex(c)}")
