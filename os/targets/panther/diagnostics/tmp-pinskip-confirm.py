#!/usr/bin/env python3
"""Confirm PinSkip eSIM gate 0x26061d6; InvalidSimState path; SIT byte17 = +0xBF4."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()


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


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Full PinSkip fail cluster
dump(0x172AF80, 0x172B180, "PINSKIP_CLUSTER")

# eSIM check function
dump(0x26061D6, 0x2606300, "IS_ESIM_CHECK")

# Invalid SimState: MOVW r2,#0x11ce
print("\n=== MOVW r2,#0x11ce ===")
o = 0x1000000
hits = []
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == 0x11CE and r[1] == 2:
        hits.append(o)
    o += 2
print([hex(h) for h in hits])
for h in hits[:4]:
    dump(h - 0x80, h + 0x30, f"invstate@{hex(h)}")

# Who calls PinSkip cluster? Find function start and callers of ~0x172b000
# Find BL → addresses in 0x172af00-0x172b180
print("\n=== callers into PinSkip cluster ===")
callers = []
o = 0x1000000
while o < 0x3C00000 - 4:
    t = bl(o)
    if t is not None and 0x172AF00 <= t < 0x172B180:
        callers.append((o, t))
    o += 2
print(f"n={len(callers)}")
for c, t in callers[:20]:
    print(f"  {hex(c)} -> {hex(t)}")

# Count STRB +0xBF4 writers (should be only SET_APP)
print("\n=== ALL STRB.W #0xBF4 ===")
o = 0x1000000
w = []
while o < 0x3C00000 - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF4:
        w.append(o)
    o += 2
print([hex(x) for x in w])

# Count SET_APP callers with imm #5 and #2
SET = 0x19916D2
print("\n=== SET_APP callers ===")
o = 0x1000000
sc = []
while o < 0x3C00000 - 4:
    if bl(o) == SET:
        # look back for MOVS r0,#imm
        imm = None
        for p in range(max(0, o - 0x10), o, 2):
            hw = u16(p)
            if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) == 0:
                imm = hw & 0xFF
        sc.append((o, imm))
    o += 2
print(f"n={len(sc)}")
from collections import Counter
print("imm counts", Counter(i for _, i in sc))
for o, imm in sc:
    print(f"  {hex(o)} imm={imm}")
