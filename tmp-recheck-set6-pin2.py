#!/usr/bin/env python3
"""Recheck SET#6 gate 0x18c2acc for pin1=2; SET#6 callers; START_NETWORK vs Pin1Verified."""
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

print("=== 0x18c2acc CMP/MOVS (SET#6 pin gate) ===")
o = 0x18C2ACC
while o < 0x18C2B80:
    hw = u16(o)
    extra = ""
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        b = bl_target(o)
        extra = f" BL->{hex(b)}" if b else ""
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
        o += 4
        continue
    if (hw & 0xFF00) == 0x2800:
        extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
    if (hw & 0xFF00) == 0x2000:
        extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
    if hw == 0x4770:
        print(f"  {hex(o)}: {hw:04x} RET")
        break
    print(f"  {hex(o)}: {hw:04x}{extra}")
    o += 2

print("\n=== BL -> SET#6 entry 0x14c643c ===")
bls = []
for o in range(0x100000, min(len(IMG) - 4, 0x5A00000), 2):
    t = bl_target(o)
    if t == 0x14C643C:
        bls.append(hex(o))
print("  count", len(bls), bls[:20])

print("\n=== START_NETWORK 0x18e8310.. CMP / LDRB.W +0xBF4/+0xBF5 ===")
# 0x18ec8c0 GET_APP; 0x18e831a region
for o in range(0x18E82F0, 0x18E8380):
    hw = u16(o)
    if (hw & 0xFF00) == 0x2800:
        print(f"  {hex(o)} CMP r{(hw>>8)&7},#{hw&0xff}")
    # LDRB.W [rn, #imm12] f89x
    if (hw & 0xFFF0) == 0xF890:
        hw2 = u16(o + 2)
        imm = hw2 & 0xFFF
        rn = hw & 0xF
        print(f"  {hex(o)} LDRB.W [r{rn},#{hex(imm)}]")

print("DONE")
