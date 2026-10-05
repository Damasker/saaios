#!/usr/bin/env python3
"""Xref SIM NOT READY / SIM is not ready; SIT 0x0700 DENIED builder."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000


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


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def sva(j):
    return VA + (j - MAIN)


def dump_str(n, lim=8):
    j = 0
    hits = []
    while True:
        i = img.find(n, j)
        if i < 0:
            break
        hits.append(i)
        j = i + 1
        if len(hits) >= lim:
            break
    return hits


print("=== string VAs ===")
for n in [
    b"SIM NOT READY (usim state : %s)",
    b"SIM is not ready",
    b"SIM_READY_IND will be Ignored",
    b"Attach Reject Cause = %s",
    b"Not registered; status: %s.",
]:
    for j in dump_str(n, 4):
        print(f"  {n[:40]!r} off={hex(j)} VA={hex(sva(j))}")


def find_movw_va(target_va, scan_lo=0x1000000, scan_hi=0x3C00000):
    lo = target_va & 0xFFFF
    hi = (target_va >> 16) & 0xFFFF
    sites = []
    o = scan_lo
    while o < scan_hi - 8:
        r = movw(o)
        if r and r[0] == lo:
            t = movt(o + 4)
            t2 = movt(o - 4) if o >= 4 else None
            got_hi = None
            if t and t[1] == r[1]:
                got_hi = t[0]
            elif t2 and t2[1] == r[1]:
                got_hi = t2[0]
            if got_hi == hi:
                sites.append(o)
        o += 2
        if len(sites) > 20:
            break
    return sites


print("\n=== MOVW/MOVT to those strings (first hits) ===")
for n in [b"SIM NOT READY (usim state : %s)", b"SIM is not ready"]:
    j = img.find(n)
    va = sva(j)
    sites = find_movw_va(va)
    print(f"{n.decode()}: VA={hex(va)} sites={len(sites)} {[hex(s) for s in sites[:8]]}")
    for s in sites[:3]:
        # dump nearby CMP / LDRB BF4 / BF6 / SET_APP-ish
        print(f"  around {hex(s)}")
        for p in range(s - 0x40, s + 0x20, 2):
            hw = u16(p)
            extra = ""
            r, t, b = movw(p), movt(p), bl(p)
            if r:
                extra = f" MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" BL->{hex(b)}"
            if (hw & 0xFF00) == 0x2800:
                extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
            if extra:
                print(f"    {hex(p)}:{hw:04x}{extra}")

# GET_APP 0x18ec8c0 used near MM?
GET_APP = 0x18EC8C0
print("\n=== BL GET_APP (count + first/last) ===")
cs = []
o = 0x1000000
while o < 0x3C00000 - 4:
    if bl(o) == GET_APP:
        cs.append(o)
    o += 2
print(f"n={len(cs)}")
if cs:
    print("first", [hex(x) for x in cs[:8]], "last", [hex(x) for x in cs[-4:]])
