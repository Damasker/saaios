#!/usr/bin/env python3
"""Re-prove SET_APP#5 gate: +0xBF6 base provenance == PresentObj?"""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


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
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                extra = f" ;LDR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                extra = f" ;STR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            # MLA / UMULL style
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            # ADD rd, rn, rm
            if (hw & 0xFE00) == 0x1800:
                extra = f" ;ADDS r{hw & 7},r{(hw >> 3) & 7},r{(hw >> 6) & 7}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# STATUS function that copies Present -> +0xBF6, then SET_APP READY
# Known: 0x14fb322 STATUS entry-ish, 0x14fb380 STRB +0xBF6, 0x14fb5c6 SET_APP#5
dump(0x14FB300, 0x14FB450, "STATUS Present copy region")
dump(0x14FB4E0, 0x14FB650, "STATUS CFG / SET_APP#5 region")

# STATUS_WRAP 0x14c6626 — how PresentObj is passed
dump(0x14C6626, 0x14C6720, "STATUS_WRAP entry")

# Who calls STATUS at 0x14fb322?
STATUS = 0x14FB322
# try nearby function entries
for cand in (0x14FB300, 0x14FB322, 0x14FB2E0, 0x14FB280):
    cs = []
    o = MAIN
    while o < len(img) - 4:
        if bl(o) == cand:
            cs.append(o)
        o += 2
    if cs:
        print(f"BL->{hex(cand)}: {len(cs)} {[hex(x) for x in cs[:15]]}")

# Trace: at 0x14fb380, what is r0? Look back for r0 = base + slot*stride
# fb04 f100 is MLA; f44f 6063 MOV.W r0,#0x318?; f241 026a MOVW r2,#0x106a
print("\n=== decode MLA/base around +0xBF6 ===")
dump(0x14FB350, 0x14FB3A0, "pre-+0xBF6 arithmetic")

# Confirm PresentObj: STATUS wrap gets arg from getobj or global
# String 0x106a xref near READY
j = img.find(b"SIM STATUS update: Present")
print(f"\n0x106a string off={hex(j)} VA={hex(VA+(j-MAIN))}")

# Pin1Verified at +0xBF5 vs obj+20 — same STATUS source r5
dump(0x14FB400, 0x14FB480, "Pin1Verified +0xBF5 copy")

# Prove r5 at STATUS is PresentObj: look for LDRB [r5,#0] before copy / caller arg
# WRAP at 0x14c6626
print("\nDone gate provenance dump")
