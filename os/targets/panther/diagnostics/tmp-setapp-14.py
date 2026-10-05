#!/usr/bin/env python3
"""Dump SET_APP #1 and #4 sites; disasm START_NETWORK GET_APP gate."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
SET_APP = 0x19916D2
GET_APP = 0x18EC8C0


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
        extra = ""
        r, t, b = movw(o), movt(o), bl(o)
        if r:
            extra = f" MOVW r{r[1]},#{hex(r[0])}"
        if t:
            extra = f" MOVT r{t[1]},#{hex(t[0])}"
        if b is not None:
            extra = f" BL->{hex(b)}"
        if (hw & 0xFF00) == 0x2000:
            extra += f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFFF0) == 0xF890:
            extra += f" LDRB.W r{(u16(o+2)>>12)&0xf},[r{hw&0xf},#{u16(o+2)&0xfff}]"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
            o += 4
        else:
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# SET_APP #1 and #4 from known table
dump(0x146AA80, 0x146AAE0, "SET_APP#1 @0x146aaba")
dump(0x14FB4E0, 0x14FB540, "SET_APP#4 @0x14fb526")
dump(0x14FB3A0, 0x14FB430, "SET_APP PUK/PIN cluster before #4")
dump(0x18E82C0, 0x18E83C0, "GET_APP START_NETWORK gate")

# Wider function containing 0x18e831a: find PUSH at start
print("\n=== function prologue search before 0x18e831a ===")
o = 0x18E831A
while o > 0x18E7000:
    hw = u16(o)
    if hw in (0xB570, 0xB5F0, 0xB5F8, 0xE92D):
        print(f"  possible prolog {hex(o)} {hw:04x}")
        if hw == 0xE92D:
            break
    o -= 2
    if 0x18E831A - o > 0x400:
        break

# Search log-id tables: string "START_NETWORK Ignored" nearby 32-bit ids
n = b"START_NETWORK Ignored: SIM is not ready"
j = img.find(n)
print(f"\nSTART_NETWORK Ignored off={hex(j)}")
# 16 bytes before string often DBT log header
print("  before", img[j - 32 : j].hex())

# Find 0x18e831a function name via nearby unique strings in 2k
win = img[0x18E7C00:0x18E8600]
for s in [
    b"START_NETWORK",
    b"SIM is not ready",
    b"SIT_0_NET",
    b"RadioPower",
    b"SET_RADIO",
]:
    k = win.find(s)
    print(f"  in window {s}: {k}")
