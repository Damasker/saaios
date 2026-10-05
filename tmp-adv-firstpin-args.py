#!/usr/bin/env python3
"""Decode FirstPIN → 0x1f1458c args; what is stored to [r4,#0]."""
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

def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    n = 0
    while o < end and n < 70:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b: extra = f" BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880: extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFFC0) == 0x4600:
                rd = (((hw >> 7) & 1) << 3) | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" MOV r{rd},r{rm}"
            if (hw & 0xF800) == 0x7000:
                extra = f" STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x7800:
                extra = f" LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            # ADDS Rd, Rn, #imm3 : 0001110 imm3 Rn Rd
            if (hw & 0xFE00) == 0x1C00:
                rd, rn, imm = hw & 7, (hw >> 3) & 7, (hw >> 6) & 7
                extra = f" ADDS r{rd},r{rn},#{imm}"
            if (hw & 0xF800) == 0x3000:
                extra = f" ADDS r{(hw>>8)&7},#{hw&0xff}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
        n += 1

# Full FirstPIN block through SimInfo and 0x1f1458c call
dump(0x1f0452c, 0x1f045f0, "FirstPIN full through post-SimInfo")

# What value goes into [r4,#0] at 0x1f145ae?
# 7820 LDRB r0,[r4,#0]
# 01c2 LSLS r2,r0,#7 ? 
# 7022 STRB r2,[r4,#0]
dump(0x1f145a0, 0x1f145d0, "bitop on [r4,#0]")

# Decode 01c2
hw = u16(0x1f145ac)
print(f"\n0x1f145ac={hw:04x}")
# LSLS immediate: 00000 imm5 Rm Rd
if (hw & 0xF800) == 0x0000:
    rd, rm, imm = hw & 7, (hw >> 3) & 7, (hw >> 6) & 0x1F
    print(f" LSLS r{rd},r{rm},#{imm}")

# 2280 = MOVS r2, #0x80?
hw = u16(0x1f145b0)
print(f"0x1f145b0={hw:04x} MOVS r{(hw>>8)&7},#{hw&0xff}" if (hw&0xFF00)==0x2000 else hex(hw))

print("DONE")
