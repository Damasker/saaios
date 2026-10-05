#!/usr/bin/env python3
"""Decode STATUS CFG for Present=0/2; SET_APP #6/#7 callers; VerifyPin→post."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()

def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]

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

def b_w_target(o):
    """B.W / Bxx.W unconditional and conditional."""
    hw, hw2 = u16(o), u16(o + 2)
    # B.W T4 unconditional: 11110 S imm10 | 10 J1 1 J2 imm11
    if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x9000:
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
    # Bcc.W T3: 11110 S cond imm6 | 10 J1 0 J2 imm11
    if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x8000:
        s = (hw >> 10) & 1
        cond = (hw >> 6) & 0xF
        imm6 = hw & 0x3F
        j1 = (hw2 >> 13) & 1
        j2 = (hw2 >> 11) & 1
        imm11 = hw2 & 0x7FF
        i1 = ~(j1 ^ s) & 1
        i2 = ~(j2 ^ s) & 1
        imm32 = (s << 20) | (i1 << 19) | (i2 << 18) | (imm6 << 12) | (imm11 << 1)
        if s:
            imm32 |= ~((1 << 21) - 1) & 0xFFFFFFFF
            if imm32 >= 0x80000000:
                imm32 -= 0x100000000
        names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
        return (names[cond] if cond < 14 else f"?{cond}", o + 4 + imm32)
    return None

def cbz_target(o):
    hw = u16(o)
    # CBZ/CBNZ: 1011 op i 1 imm5 Rn
    if (hw & 0xF500) != 0xB100:
        return None
    op = (hw >> 11) & 1  # 0=CBZ 1=CBNZ
    i = (hw >> 9) & 1
    imm5 = (hw >> 3) & 0x1F
    rn = hw & 7
    imm = (i << 5 | imm5) << 1
    return ("CBNZ" if op else "CBZ", rn, o + 2 + imm)

print("=== key branches in STATUS ===")
for o in (0x14FB34E, 0x14FB3AC, 0x14FB3D8, 0x14FB4A4, 0x14FB4D8, 0x14FB522, 0x14FB5C2, 0x14FB5B6):
    hw = u16(o)
    print(f"  {hex(o)}: {hw:04x}", end="")
    c = cbz_target(o)
    if c:
        print(f" {c[0]} r{c[1]} -> {hex(c[2])}")
        continue
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        bw = b_w_target(o)
        bl = bl_target(o)
        hw2 = u16(o + 2)
        print(f" {hw2:04x}", end="")
        if bl:
            print(f" BL->{hex(bl)}")
        elif isinstance(bw, tuple):
            print(f" B{bw[0]}.W->{hex(bw[1])}")
        elif bw:
            print(f" B.W->{hex(bw)}")
        else:
            print()
    elif (hw & 0xF000) == 0xD000:
        cond = (hw >> 8) & 0xF
        imm = hw & 0xFF
        if imm >= 0x80: imm -= 0x100
        names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
        print(f" B{names[cond]}->{hex(o+4+imm*2)}")
    elif (hw & 0xF800) == 0xE000:
        imm = hw & 0x7FF
        if imm >= 0x400: imm -= 0x800
        print(f" B->{hex(o+4+imm*2)}")
    else:
        print()

# Dump SET_APP #1, #6, #7 sites context
SET_APP = 0x19916D2
print("\n=== SET_APP #1/#6/#7 context ===")
for site in (0x146AABA, 0x14C64A6, 0x18EC4FE):
    print(f"\n--- site {hex(site)} ---")
    start = max(0, site - 0x40)
    o = start
    while o < site + 0x20:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b: extra = f" BL->{hex(b)}"
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
            mark = " <<<" if o == site else ""
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}{mark}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            mark = " <<<" if o == site else ""
            print(f"  {hex(o)}: {hw:04x}{extra}{mark}")
            o += 2

# Who BL to SET_APP#6 / #7 function starts — find function containing site
# Search BL to functions containing those sites: scan for strings nearby

# Message post helpers after FirstPIN — look for known scheduler post patterns
# Common: MOVS r0,#imm; BL post. Scan FirstPIN callees for MOVS #6/#7 SET? already 0.

# After VerifyPin: does anything call SET_APP#1 (0x146aaba's function)?
# Find all BL to function containing 0x146aaba — need function start.
# Search xrefs: BL targets near 0x146aa00

# MAIN code only (skip TOC/padding) — file range covering Thumb MAIN
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)

print("\n=== BLs targeting near SET#1 function (0x146aa00..0x146ac00) ===")
o = SCAN_LO
count = 0
while o < SCAN_HI and count < 30:
    b = bl_target(o)
    if b and 0x146AA00 <= b <= 0x146AC00:
        print(f"  {hex(o)} -> {hex(b)}")
        count += 1
    o += 2

print("\n=== BLs targeting near SET#6 (0x14c6400..0x14c6600) ===")
o = SCAN_LO
count = 0
while o < SCAN_HI and count < 40:
    b = bl_target(o)
    if b and 0x14C6400 <= b <= 0x14C6600:
        print(f"  {hex(o)} -> {hex(b)}")
        count += 1
    o += 2

print("\n=== BLs targeting near SET#7 (0x18ec400..0x18ec600) ===")
o = SCAN_LO
count = 0
while o < SCAN_HI and count < 40:
    b = bl_target(o)
    if b and 0x18EC400 <= b <= 0x18EC600:
        print(f"  {hex(o)} -> {hex(b)}")
        count += 1
    o += 2

# Decode GET_APP return values used: STATUS checks 1,4 then skip path 6,7
# What sets app to 6 or 7 after PIN verify?

print("\n=== STATUS skip entry 0x14fb4ca detail ===")
o = 0x14FB4CA
while o < 0x14FB5D0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        b = bl_target(o)
        bw = b_w_target(o)
        if b: extra = f" BL->{hex(b)}"
        elif isinstance(bw, tuple): extra = f" B{bw[0]}.W->{hex(bw[1])}"
        elif bw: extra = f" B.W->{hex(bw)}"
        if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        c = cbz_target(o)
        if c: extra = f" {c[0]} r{c[1]}->{hex(c[2])}"
        if (hw & 0xF000) == 0xD000:
            cond = (hw >> 8) & 0xF
            imm = hw & 0xFF
            if imm >= 0x80: imm -= 0x100
            names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
            if cond < 14:
                extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
        if (hw & 0xF800) == 0xE000:
            imm = hw & 0x7FF
            if imm >= 0x400: imm -= 0x800
            extra = f" B->{hex(o+4+imm*2)}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

print("DONE")
