#!/usr/bin/env python3
"""SET_APP#5 +0xBF6 base provenance vs PresentObj."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
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
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Core READY path
dump(0x14FB360, 0x14FB3F0, "STATUS +0xBF6 copy from Present")
dump(0x14FB5A0, 0x14FB5E0, "SET_APP#5 gate LDRB +0xBF6")
dump(0x14C6626, 0x14C66C0, "STATUS_WRAP PresentObj arg")

# At 0x14fb380: STRB.W [r0,#0xBF6] where r0 = slot_base; value from?
# Prior: copies PresentObj[0]. Trace r7 at STRB — often the Present byte in r7
# Look at 0x14fb374 MOVW r8 and fb16 8000 MLA

# Confirm sole STRB +0xBF6 and sole SET_APP#5
bf6 = []
set5 = []
o = MAIN
while o < len(img) - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
        bf6.append(o)
    b = bl(o)
    if b == 0x19916D2:
        # check MOVS #5 in prior 12 bytes
        for p in range(max(MAIN, o - 12), o, 2):
            h = u16(p)
            if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 5:
                set5.append(o)
                break
    o += 2
print(f"\nSTRB +0xBF6 sites: {[hex(x) for x in bf6]}")
print(f"SET_APP#5 sites: {[hex(x) for x in set5]}")

# PresentObj identity: FN_A getobj #636c result used as [r8,#0]=2
# STATUS_WRAP: does it load same object?
dump(0x14F6A00, 0x14F6A30, "FN_A PresentObj[0]=2")

# Log string Present in STATUS uses LDRB [r5,#0] — r5 is PresentObj pointer arg
dump(0x14FB430, 0x14FB4A0, "log 0x106a Present from [r5,#0]")
print("DONE")
