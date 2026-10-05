#!/usr/bin/env python3
"""Disasm MOVS#2 STRB sites near STATUS; check if they set Present before 0x106a."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
VA_BASE = 0x40010000
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
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


def dump(start, length):
    o = start
    end = start + length
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = " <<" if r[0] in (0x106A, 0x18E, 0x7AB, 0x2C5E, 0xC6B) else ""
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP #{hw&0xff}"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


for site in (0x14F6A14, 0x14F9578, 0x1F02828):
    print(f"\n======== MOVS#2 site @{site:#x}/va{va(site):#x} ========")
    print(dump(site - 0x40, 0xA0))

# Also: does Pin1Verified path eventually call something that sets trio Present=2?
# Search BL from pin1v fn to addresses near 0x14f6a14 / STATUS
print("\n=== BLs from Pin1Verified fn (0x1f04400..0x1f04700) ===")
for o in range(0x1F04400, 0x1F04700, 2):
    b = bl_target(o)
    if b and (0x14F0000 <= b <= 0x1500000 or 0x18EC000 <= b <= 0x18F0000):
        print(f"  @{o:#x} BL->{b:#x}")

# Search who writes Present source: look for pattern STRB #0/#1/#2 consecutive (trio)
print("\n=== Consecutive Present/Pin1V/MePer trio stores (MOVS + STRB #0,#1,#2) ===")
# scan for MOVS; STRB [rN,#0] then soon MOVS; STRB [rN,#1]
for o in range(MAIN_OFF, 0x2000000, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000:
        continue
    # look for STRB [rN,#0] within 8 bytes
    for q in range(o + 2, o + 10, 2):
        h = u16(q)
        if (h & 0xF800) != 0x7000 or ((h >> 6) & 0x1F) != 0:
            continue
        rn = (h >> 3) & 7
        val0 = hw & 0xFF
        # look ahead for STRB same rn #1 and #2
        found1 = found2 = None
        for p in range(q + 2, q + 0x30, 2):
            hh = u16(p)
            if (hh & 0xFF00) == 0x2000:
                val = hh & 0xFF
                for s in range(p + 2, p + 10, 2):
                    hs = u16(s)
                    if (hs & 0xF800) == 0x7000 and ((hs >> 3) & 7) == rn:
                        off = (hs >> 6) & 0x1F
                        if off == 1:
                            found1 = val
                        if off == 2:
                            found2 = val
        if found1 is not None and found2 is not None:
            # only if near SIM STATUS (within 0x2000 of 0x14fb380) or has 0x106a
            if abs(o - 0x14FB380) < 0x3000:
                print(f"  trio @{o:#x}: [0]={val0} [1]={found1} [2]={found2}")
        break

# Check 0x14f6xxx region for 0x106a and Present building
print("\n=== Any 0x106a in 0x14f6000..0x14fc000 ===")
for o in range(0x14F6000, 0x14FC000, 2):
    r = movw(o)
    if r and r[0] == 0x106A:
        print(f"  0x106a @{o:#x}")
