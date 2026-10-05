#!/usr/bin/env python3
"""Follow-up: STRH +0xBF6 writers + HotSwap insert vs CardPower vs STATUS wrap callers."""
from __future__ import annotations
import struct
from pathlib import Path

PATH = Path(__file__).resolve().parent / "fw" / "saaios-probe-b-modem.bin"
MAIN = 0x16C10
END = MAIN + 0x05917ACC
VA0 = 0x40010000
img = PATH.read_bytes()

SET_APP = 0x19916D2
STATUS = 0x14FB322
FN_A = 0x14F692C
GET_APP = 0x18EC8C0
STATUS_WRAP = 0x14C6626  # PUSH {r4,lr} entry


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


def cstr(v, lim=120):
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
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8A0:
                extra = f" ;STRH.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                extra = f" ;STR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


def nearby_strings(o, radius=0x80):
    names = []
    for p in range(max(MAIN, o - radius), min(END, o + 0x40), 2):
        r = movw(p)
        if not r:
            continue
        for q in range(max(MAIN, p - 16), min(END, p + 24), 2):
            t = movt(q)
            if not t or t[1] != r[1]:
                continue
            if not (0x4000 <= t[0] <= 0x4600):
                continue
            s = cstr((t[0] << 16) | r[0])
            if s and len(s) > 4:
                names.append(s[:100])
    return names


def find_xrefs_to_str(needle: bytes):
    hits = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        v = va(j)
        lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
        for o in range(MAIN, END - 8, 2):
            r = movw(o)
            if not r or r[0] != lo:
                continue
            for p in range(max(MAIN, o - 16), min(END, o + 20), 2):
                t = movt(p)
                if t and t[0] == hi and t[1] == r[1]:
                    hits.append((o, j, v))
                    break
        i = j + 1
    return hits


print("=== A) STRH/STR to +0xBF6 / +0xBF5 context (possible missed Present writers) ===")
sites = [
    0x18FE446,  # STRH #bf5 — also writes bf6!
    0x299E852, 0x299E87A,
    0x33CF682, 0x33CF69E, 0x33CFB3A, 0x33CFB5C,
    0x1816BF2, 0x19A539C, 0x1D8B92A, 0x1D8B968,
]
for s in sites:
    print(f"\n-- site {hex(s)} --")
    print(dump(s - 0x40, s + 0x30))
    ns = nearby_strings(s, 0xC0)
    if ns:
        print("  strings:", ns[:6])
    # BLs nearby to SET_APP/STATUS/FN_A/GET_APP
    for p in range(s - 0x60, s + 0x40, 2):
        bt = bl_target(p)
        if bt in (SET_APP, STATUS, FN_A, GET_APP, STATUS_WRAP):
            print(f"  nearby BL {hex(p)}->{hex(bt)}")

print("\n=== B) Is STRH#bf6 same SIM-status object? Look for MLA base 0x48C649B8 / stride ===")
# STATUS uses MOVW r8,#0x49b8 MOVT #0x48c6 then MLA
base_lo, base_hi = 0x49B8, 0x48C6
x = []
for o in range(MAIN, END - 8, 2):
    r = movw(o)
    if not r or r[0] != base_lo:
        continue
    for p in range(o, min(END, o + 16), 2):
        t = movt(p)
        if t and t[0] == base_hi and t[1] == r[1]:
            x.append(o)
            break
print(f"MLA base xrefs n={len(x)}: {list(map(hex, x[:20]))}")

# PresentObj singleton 0x48b87b7c
po_lo, po_hi = 0x7B7C, 0x48B8
x2 = []
for o in range(MAIN, END - 8, 2):
    r = movw(o)
    if not r or r[0] != po_lo:
        continue
    for p in range(o, min(END, o + 16), 2):
        t = movt(p)
        if t and t[0] == po_hi and t[1] == r[1]:
            x2.append(o)
            break
print(f"PresentObj ptr 0x48b87b7c xrefs n={len(x2)}: {list(map(hex, x2[:30]))}")

print("\n=== C) STATUS_WRAP callers ===")
cs = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == STATUS_WRAP]
print(f"n={len(cs)}: {list(map(hex, cs[:40]))}")
for c in cs[:15]:
    print(f"\n-- caller {hex(c)} --")
    print(dump(c - 0x30, c + 0x10))
    print("  strings:", nearby_strings(c, 0xA0)[:5])

print("\n=== D) HotSwap insert string xrefs ===")
for needle in (
    b"SIM insertion resulted into new diag trigger",
    b"HotSwapInsertTimer state",
    b"HotSwapInsertTimer hotswapstate",
    b"SIM removal resulted into new diag trigger",
    b"Sending SIM_PRESENT_IND to PBM from USIM_CARD_PRESENT",
    b"Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ",
    b"sitTxSimSetCardPower",
    b"DS_TCS_GV_FCN_SIM_HOT_SWAP",
):
    print(f"\n## {needle.decode('ascii','replace')[:60]}")
    xrefs = find_xrefs_to_str(needle)
    print(f"xrefs n={len(xrefs)}")
    for xo, so, sv in xrefs[:6]:
        print(f"  code@{hex(xo)} strVA={hex(sv)}")
        print(dump(xo - 0x20, xo + 0x50))
        # any BL SET_APP / STATUS / FN_A in window
        for p in range(xo - 0x100, xo + 0x120, 2):
            bt = bl_target(p)
            if bt in (SET_APP, STATUS, FN_A, STATUS_WRAP, GET_APP):
                print(f"    BL {hex(p)}->{hex(bt)}")

print("\n=== E) Does HotSwapInsert BL SET#1 / STATUS / FN_A? Scan USIM hotswap band ===")
# Find HotSwapInsertTimer xref function bounds and scan BLs
xrefs = find_xrefs_to_str(b"HotSwapInsertTimer state")
for xo, _, _ in xrefs[:3]:
    # find fn start
    fn = None
    for p in range(xo, max(MAIN, xo - 0x800), -2):
        h = u16(p)
        if (h & 0xFFF0) == 0xE92D or h in (0xB5F0, 0xB570, 0xB5B0):
            fn = p
            break
    print(f"\nHotSwapInsert fn~{hex(fn) if fn else None} xref@{hex(xo)}")
    if not fn:
        continue
    # scan 0x400 bytes of BLs
    interesting = []
    for p in range(fn, min(END, fn + 0x600), 2):
        bt = bl_target(p)
        if bt is None:
            continue
        if bt in (SET_APP, STATUS, FN_A, STATUS_WRAP, GET_APP, 0x146A99C, 0x14F6A16):
            interesting.append((p, bt))
        # also note Present=2 style MOVS#2 STRB
    print(f"  interesting BLs: {[(hex(a), hex(b)) for a,b in interesting]}")
    # MOVS#2 + STRB.W #0 in this fn window
    for p in range(fn, min(END, fn + 0x600), 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
            for b in range(2, 24, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
                    print(f"  Present-ish STRB#0=2 @{hex(p)}")
                    break

print("\n=== F) pin1 ENABLED_VERIFIED(2): any CMP #2 on pin1 field -> SET_APP#5? ===")
# Scan STATUS fn for CMP related to pin1 before READY
print(dump(0x14FB3B0, 0x14FB580))

print("\n=== G) CardPower handler: does it touch HotSwap / ABSENT / SET#1? ===")
xrefs = find_xrefs_to_str(b"sitTxSimSetCardPower")
for xo, _, _ in xrefs[:4]:
    print(f"\n-- CardPower @{hex(xo)} --")
    print(dump(xo - 0x10, xo + 0x80))
    for p in range(xo - 0x40, xo + 0x200, 2):
        bt = bl_target(p)
        if bt in (SET_APP, STATUS, FN_A, STATUS_WRAP, 0x146A99C, GET_APP):
            print(f"  BL {hex(p)}->{hex(bt)}")

print("\nDONE")
