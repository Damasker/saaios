#!/usr/bin/env python3
"""Dump SR_IF PAUSE_REQ poster @0x261acfe; check GET_APP/camp gates."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()

def u16(o): return struct.unpack_from("<H", IMG, o)[0]

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

GET_APP, SET_APP = 0x18EC8C0, 0x19916D2

# find func start before 0x261acfe
o = 0x261ACFE
while o > 0x261A800:
    if u16(o) == 0xE92D and (u16(o+2) & 0x4000):
        print(f"func @{hex(o)}")
        start = o
        break
    o -= 2
else:
    start = 0x261AC80
    print("no PUSH, dump from", hex(start))

print("\n=== poster dump ===")
o = start
end = start + 0x120
while o < end:
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
        mark = " <<<ADR" if o == 0x261ACFE else ""
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}{mark}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

# BLs to this func
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG)-4, 0x5A00000)
print(f"\n=== BL -> {hex(start)} ===")
callers = []
o = SCAN_LO
while o < SCAN_HI:
    if bl_target(o) == start:
        callers.append(o)
    o += 2
print([hex(c) for c in callers[:30]])

# In poster + callees 1 level: any GET_APP?
print("\n=== GET_APP in poster±0x200 and direct callees ===")
cals = set()
o = start
while o < start + 0x200:
    b = bl_target(o)
    if b:
        cals.add(b)
        if b == GET_APP:
            print(f"  GET_APP at {hex(o)}")
    o += 2
for c in list(cals)[:20]:
    if not (0x100000 <= c < len(IMG)):
        continue
    a, n = c, 0
    while a < c + 0x100 and n < 80:
        b = bl_target(a)
        if b == GET_APP:
            print(f"  callee {hex(c)} BL GET_APP @{hex(a)}")
        if (u16(a) & 0xF800) in (0xE800, 0xF000, 0xF800):
            a += 4
        else:
            a += 2
        n += 1

print("DONE")
