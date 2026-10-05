#!/usr/bin/env python3
from pathlib import Path
import struct

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()


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


print("=== ignore path 0x19a8484 ===")
o = 0x19A8484
end = 0x19A8500
while o < end:
    hw = u16(o)
    extra = ""
    r, t, b = movw(o), movt(o), bl(o)
    if r:
        extra = f" MOVW r{r[1]},#{hex(r[0])}"
    if t:
        extra = f" MOVT r{t[1]},#{hex(t[0])}"
    if b is not None:
        extra = f" BL->{hex(b)}"
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
        o += 4
    else:
        print(f" {hex(o)}:{hw:04x}{extra}")
        o += 2
