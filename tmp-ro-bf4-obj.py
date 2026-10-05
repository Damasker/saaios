#!/usr/bin/env python3
"""RO: is +0xBF4 on PresentObj #636c or a different object?"""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

GET_APP = 0x18EC8C0
SET_APP = 0x19916D2
SET_APP_STRB = 0x1991734
STATUS = 0x14FB322
GETOBJ = 0x20EA040


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(o):
    if o + 4 > len(IMG):
        return None
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


def movw_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return rd, (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8


print("=== GET_APP body ===")
for o in range(GET_APP, GET_APP + 0x40, 2):
    t = bl_target(o)
    m = movw_imm(o)
    hw = u16(o)
    bits = []
    if t:
        print(f"  {hex(o)} BL->{hex(t)}")
        continue
    if m:
        bits.append(f"MOVW r{m[0]},#{hex(m[1])}")
    # LDRB.W Rt,[Rn,#imm12]
    if (hw & 0xF800) == 0xF800 or True:
        hw2 = u16(o + 2)
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0x0F00) == 0x0000:
            rn = hw & 0xF
            rt = (hw2 >> 12) & 0xF
            imm12 = hw2 & 0xFFF
            bits.append(f"LDRB.W r{rt},[r{rn},#{hex(imm12)}]")
    if bits:
        print(f"  {hex(o)} {' '.join(bits)}")

print("\n=== SET_APP STRB +0xBF4 neighborhood ===")
for o in range(SET_APP, SET_APP + 0x80, 2):
    t = bl_target(o)
    m = movw_imm(o)
    hw, hw2 = u16(o), u16(o + 2)
    bits = []
    if t:
        print(f"  {hex(o)} BL->{hex(t)}")
        continue
    if m:
        bits.append(f"MOVW r{m[0]},#{hex(m[1])}")
    if (hw & 0xFFF0) == 0xF880:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm12 = hw2 & 0xFFF
        bits.append(f"STRB.W r{rt},[r{rn},#{hex(imm12)}]")
    if (hw & 0xFFF0) == 0xF890:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm12 = hw2 & 0xFFF
        bits.append(f"LDRB.W r{rt},[r{rn},#{hex(imm12)}]")
    if bits:
        print(f"  {hex(o)} {' '.join(bits)}")

print("\n=== STATUS: PresentObj arg vs +0xBF4 base ===")
for o in range(STATUS, STATUS + 0x120, 2):
    t = bl_target(o)
    m = movw_imm(o)
    hw, hw2 = u16(o), u16(o + 2)
    bits = []
    if t:
        tag = ""
        if t == GETOBJ:
            tag = " GETOBJ"
        if t == SET_APP or abs(t - SET_APP) < 8:
            tag = " SET_APP"
        print(f"  {hex(o)} BL->{hex(t)}{tag}")
        continue
    if m:
        bits.append(f"MOVW r{m[0]},#{hex(m[1])}")
    if (hw & 0xFFF0) == 0xF890:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm12 = hw2 & 0xFFF
        bits.append(f"LDRB.W r{rt},[r{rn},#{hex(imm12)}]")
    if (hw & 0xFFF0) == 0xF880:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm12 = hw2 & 0xFFF
        bits.append(f"STRB.W r{rt},[r{rn},#{hex(imm12)}]")
    if bits:
        print(f"  {hex(o)} {' '.join(bits)}")

# Size check: 0xBF4 < 0x636c?
print(f"\n0xBF4 < 0x636c = {0xBF4 < 0x636c} (fits in PresentObj alloc)")
print(f"0xBF5, 0xBF6 offsets likewise on same object if STATUS base==PresentObj")
print("DONE")
