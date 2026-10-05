#!/usr/bin/env python3
"""modem_a: confirm Present=2 site is CDMA FN_A; find SET_APP#5 gate."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-a-modem.bin").read_bytes()
MAIN = 0x16C10


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


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b = movw(o), bl(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
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
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# A Present=2 at 0x14f7e10
dump(0x14F7D3C, 0x14F7E40, "A FN_A-ish Present=2")
dump(0x14FA950, 0x14FA9A0, "A second Present=2 (FN_B?)")

# +0xBF6 STRB at 0x14fc75c — dump STATUS around it and find SET_APP#5
dump(0x14FC700, 0x14FC780, "A +0xBF6 STATUS copy")
# Find MOVS#5 + BL after LDRB #bf6 near 0x14fcxxx
print("\n=== A scan SET_APP#5 near STATUS ===")
for o in range(0x14FC500, 0x14FD000, 2):
    hw = u16(o)
    if (hw & 0xFFF0) == 0xF890 and (u16(o + 2) & 0xFFF) == 0xBF6:
        print(f"LDRB +0xBF6 @{hex(o)}")
        dump(o, o + 0x20, f"after LDRB@{hex(o)}")
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 5:
        b = bl(o + 2)
        if b:
            # check BF6 nearby
            for p in range(max(0x14FC500, o - 0x20), o, 2):
                if (u16(p) & 0xFFF0) == 0xF890 and (u16(p + 2) & 0xFFF) == 0xBF6:
                    print(f"SET_APP#5 candidate BL@{hex(o+2)}->{hex(b)}")
                    dump(o - 0x10, o + 0x10, "READY gate")

# Who BL to A FN_A entry (function containing 0x14f7e10)
# Find push before 0x14f7d3c
entry = 0x14F7C00
for o in range(0x14F7D3C, 0x14F7C00, -2):
    if u16(o) == 0xE92D:
        entry = o
        break
print(f"\nA FN_A entry ~{hex(entry)}")
cs = []
o = MAIN
while o < len(img) - 4:
    if bl(o) == entry:
        cs.append(o)
    o += 2
print(f"direct BL to entry: {len(cs)} {[hex(c) for c in cs[:10]]}")

# version string
j = img.find(b"g5300q-")
print("ver", img[j : j + 40])
