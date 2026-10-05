#!/usr/bin/env python3
import struct
from pathlib import Path

PATH = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin")
img = PATH.read_bytes()

def u16(o):
    return struct.unpack_from("<H", img, o)[0]

def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

def bl(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1, j2, imm11 = (hw2 >> 13) & 1, (hw2 >> 11) & 1, hw2 & 0x7FF
    i1, i2 = ~(j1 ^ s) & 1, ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32

def dump(o0, o1):
    o = o0
    while o < o1:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, b = movw(o), movt(o), bl(o)
            if r:
                extra = f" MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" BL->{hex(b)}"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

INTEREST = {0x19916D2, 0x14FB322, 0x14F692C, 0x14C6626, 0x18EC8C0, 0x146A99C, 0x1991838}
sites = [
    ("SIM insertion lo", 0x1936F98),
    ("SIM insertion lo2", 0x1AEA7CE),
    ("PRESENT_IND", 0x16AF3BA),
    ("PRESENT_IND2", 0x16AF550),
    ("PRESENT_IND3", 0x16AFA1A),
    ("WAIT_INIT", 0x171A404),
    ("ABSENT_WAIT", 0x165C6D6),
    ("STATUS Present update", 0x1B0F938),
    ("NoCDMA Supp lo", 0x14A59A0),
    ("Supp hex lo", 0x159162C),
]
for name, o in sites:
    print(f"\n=== {name} @{hex(o)} ===")
    r = movw(o)
    print(f"MOVW={r}")
    for p in range(o - 32, o + 40, 2):
        t = movt(p)
        if t and r and t[1] == r[1]:
            print(f"  paired MOVT@{hex(p)} => VA={hex((t[0]<<16)|r[0])}")
    dump(o - 0x20, o + 0x40)
    for p in range(o - 0x100, o + 0x200, 2):
        b = bl(p)
        if b in INTEREST:
            print(f"  NEAR BL {hex(p)}->{hex(b)}")
