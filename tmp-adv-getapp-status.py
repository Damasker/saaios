#!/usr/bin/env python3
"""GET_APP vs Pin1Verified; can STATUS re-enter after VerifyPin?"""
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

def dump(start, end, lab, maxn=90):
    print(f"\n=== {lab} ===")
    o = start
    n = 0
    while o < end and n < maxn:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b: extra = f" BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880: extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0: extra = f" LDR.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
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
        n += 1

# GET_APP function
dump(0x18EC8C0, 0x18EC8C0 + 0x120, "GET_APP 0x18ec8c0")

# STATUS prolog gates on GET_APP
dump(0x14FB322, 0x14FB3C0, "STATUS prolog GET_APP gate")

# After PIN verified: does anything SET_APP #1/#4/#5 outside STATUS?
# Search BL SET_APP in 0x1f04000-0x1f05000 (FirstPIN region) and 0x1dcb000-0x1dcc000
SET_APP = 0x19916D2
print("\n=== BL SET_APP in VerifyPin neighborhoods ===")
for lo, hi in ((0x1F04000, 0x1F05000), (0x1DCB000, 0x1DCC000), (0x1F14500, 0x1F14700), (0x1D76000, 0x1D78000)):
    o = lo
    while o < hi - 4:
        b = bl_target(o)
        if b == SET_APP:
            # look back for MOVS r0,#imm
            imm = None
            for a in range(max(lo, o - 8), o, 2):
                h = u16(a)
                if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == 0:
                    imm = h & 0xFF
            print(f"  {hex(o)} BL SET_APP imm_r0={imm}")
        o += 2

print("DONE")
