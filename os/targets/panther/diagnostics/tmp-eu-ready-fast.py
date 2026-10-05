#!/usr/bin/env python3
"""Fast targeted: STATUS_WRAP arg, Thumb16#2 getobj IDs, HotSwap lo16 xrefs, FN_A imm3."""
from __future__ import annotations
import struct
from pathlib import Path

PATH = Path(__file__).resolve().parent / "fw" / "saaios-probe-b-modem.bin"
MAIN = 0x16C10
END = MAIN + 0x05917ACC
VA0 = 0x40010000
img = PATH.read_bytes()
GETOBJ = 0x20EA040
FN_A = 0x14F692C
SET_APP = 0x19916D2
STATUS_WRAP = 0x14C6626
SET1 = 0x146A99C


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA0 + (o - MAIN)


def off_va(v):
    return MAIN + (v - VA0)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def bl_target(o):
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


def cstr(v, lim=100):
    o = off_va(v)
    if o < 0 or o >= len(img):
        return None
    b = o
    while b < o + lim and 32 <= img[b] < 127:
        b += 1
    if b == o:
        return None
    return img[o:b].decode("ascii", "replace")


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                extra = f" ;BL->{hex(bt)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


print("=== 1) STATUS_WRAP @0x14b7ca8 — what is r0 / case label? ===")
# Walk back for switch case name loads
print(dump(0x14B7C40, 0x14B7CC0))
# Resolve MOVW/MOVT pairs nearby as strings
for o in range(0x14B7C40, 0x14B7CC0, 2):
    r = movw(o)
    if not r:
        continue
    for p in range(o, min(END, o + 12), 2):
        t = movt(p)
        if t and t[1] == r[1]:
            s = cstr((t[0] << 16) | r[0])
            if s:
                print(f"  str@{hex(o)}: {s[:90]}")

print("\n=== 2) STATUS_WRAP @0x1a3e6cc context ===")
print(dump(0x1A3E680, 0x1A3E700))
for o in range(0x1A3E680, 0x1A3E700, 2):
    r = movw(o)
    if not r:
        continue
    for p in range(o, min(END, o + 12), 2):
        t = movt(p)
        if t and t[1] == r[1]:
            s = cstr((t[0] << 16) | r[0])
            if s:
                print(f"  str@{hex(o)}: {s[:90]}")

print("\n=== 3) Thumb16 STRB#0=2 near getobj — which object id? ===")
sites = [
    0x19432AE, 0x19438E2, 0x194424C, 0x1944666, 0x1945DDE,
    0x1947FD4, 0x1948316, 0x146AC48,
]
for s in sites:
    ids = []
    for p in range(s - 0x80, s + 0x20, 2):
        r = movw(p)
        if r and r[1] == 1:  # r1 often object id
            # confirm BL getobj soon
            for q in range(p, min(END, p + 0x30), 2):
                if bl_target(q) == GETOBJ:
                    ids.append(r[0])
                    break
    print(f"  STRB@{hex(s)} getobj_ids={list(map(hex, ids))}")
    print(dump(s - 0x30, s + 0x10))

print("\n=== 4) HotSwap: lo16-only xref in USIM band for insert strings ===")
needles = [
    b"[HOTSWAP] HotSwapInsertTimer state",
    b"SIM insertion resulted into new diag trigger",
    b"SIM removal resulted into new diag trigger",
    b"sitTxSimSetCardPower",
    b"Warm Reset Failed, Perform Cold Reset",
]
for needle in needles:
    j = img.find(needle)
    if j < 0:
        print(f"MISS {needle[:40]!r}")
        continue
    v = va(j)
    lo = v & 0xFFFF
    print(f"\n{needle.decode()[:50]} VA={hex(v)} lo={hex(lo)}")
    hi = (v >> 16) & 0xFFFF
    bands = [
        (0x1900000, 0x1A00000),
        (0x1400000, 0x1500000),
        (0x1700000, 0x1800000),
        (0x1C00000, 0x1E00000),
        (0x4C00000, min(END, 0x4E00000)),
    ]
    hits = []
    for lo_b, hi_b in bands:
        lo_b = max(MAIN, lo_b)
        hi_b = min(END - 8, hi_b)
        for o in range(lo_b, hi_b, 2):
            r = movw(o)
            if not r or r[0] != lo:
                continue
            ok = False
            for p in range(max(MAIN, o - 20), min(END, o + 24), 2):
                t = movt(p)
                if not t or t[1] != r[1]:
                    continue
                if t[0] == hi or (0x40C0 <= t[0] <= 0x4500):
                    ok = True
                    break
            if ok:
                hits.append(o)
                if len(hits) >= 8:
                    break
        if len(hits) >= 8:
            break
    print(f"  hits={list(map(hex, hits))}")
    for h in hits[:3]:
        print(dump(h - 0x20, h + 0x60))
        for p in range(h - 0x200, h + 0x200, 2):
            bt = bl_target(p)
            if bt in (SET_APP, FN_A, SET1, STATUS_WRAP, 0x14FB322, GETOBJ):
                print(f"    BL {hex(p)}->{hex(bt)}")

print("\n=== 5) FN_A: any BL with MOVS r0,#3 just before? ===")
fna_callers = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == FN_A]
print(f"FN_A callers={list(map(hex, fna_callers))}")
for c in fna_callers:
    movs3 = False
    for b in range(2, 40, 2):
        h = u16(c - b)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 3:
            movs3 = True
            print(f"  {hex(c)} has MOVS#3 @{hex(c-b)}")
    print(dump(c - 0x30, c + 8))

print("\n=== 6) Does pin1 field CMP affect READY in STATUS? scan 0x14fb3b0..0x14fb5c6 for pin1 ===")
# Look for LDRB of pin-status style then CMP before SET#5
print(dump(0x14FB3B0, 0x14FB450))
print("---")
print(dump(0x14FB4C0, 0x14FB5D0))

print("\nDONE")
