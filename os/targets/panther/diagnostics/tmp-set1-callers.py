#!/usr/bin/env python3
"""RO: SET_APP#1 callers — what event reaches DETECTED stably?"""
import struct
from pathlib import Path

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
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

def dump(lo, hi, lab):
    print(f"\n=== {lab} {hex(lo)}..{hex(hi)} ===")
    o = lo
    while o < hi:
        b = bl(o)
        mw, mt = movw(o), movt(o)
        hw = u16(o)
        if b is not None:
            mark = ""
            if b == 0x19916D2:
                mark = " SET_APP"
            if b == 0x18EC8C0:
                mark = " GET_APP"
            if b == 0x146A99C:
                mark = " SET1FN"
            print(f"  {hex(o)}: BL->{hex(b)}{mark}")
            o += 4
            continue
        if mw:
            print(f"  {hex(o)}: MOVW r{mw[1]},#{hex(mw[0])}")
            o += 4
            continue
        if mt:
            print(f"  {hex(o)}: MOVT r{mt[1]},#{hex(mt[0])}")
            o += 4
            continue
        if (hw & 0xFF00) == 0x2000:
            print(f"  {hex(o)}: MOVS r{(hw>>8)&7},#{hw&0xff}")
            o += 2
            continue
        if (hw & 0xFF00) == 0x2800:
            print(f"  {hex(o)}: CMP r{(hw>>8)&7},#{hw&0xff}")
            o += 2
            continue
        if (hw & 0xF800) == 0x7800:
            print(f"  {hex(o)}: LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]")
            o += 2
            continue
        if (hw & 0xFFF0) == 0xF890:
            print(f"  {hex(o)}: LDRB.W r{(u16(o+2)>>12)&0xf},[r{hw&0xf},#{hex(u16(o+2)&0xfff)}]")
            o += 4
            continue
        if (hw & 0xF000) == 0xD000 and ((hw >> 8) & 0xF) != 0xF:
            imm = hw & 0xFF
            if imm & 0x80:
                imm -= 0x100
            print(f"  {hex(o)}: Bcond{(hw>>8)&0xf} ->{hex(o+4+imm*2)}")
            o += 2
            continue
        if (hw & 0xF800) == 0xE000:
            imm = hw & 0x7FF
            if imm & 0x400:
                imm -= 0x800
            print(f"  {hex(o)}: B ->{hex(o+4+imm*2)}")
            o += 2
            continue
        o += 2

# Find who BLs to callers of SET1FN
callers = [0x14538E4, 0x1A2ABF8]
for c in callers:
    # find prolog
    o = c
    while o > c - 0x400:
        if u16(o) == 0xE92D:
            print(f"prolog@{hex(o)} for caller BL@{hex(c)}")
            dump(o, c + 0x80, f"fn containing {hex(c)}")
            break
        o -= 2

# Who calls those functions?
for tgt in [0x14535A2, 0x1A2AAC8]:
    hits = []
    for o in range(0, len(img) - 4, 2):
        t = bl(o)
        if t == tgt:
            hits.append(o)
    print(f"\nBLs to {hex(tgt)}: n={len(hits)}", [hex(h) for h in hits[:25]])

# Strings near SET1 / PRESENT / INSERT
for s in [
    b"SIM_PRESENT_IND",
    b"SIM_ABSENT_IND",
    b"SIM detected",
    b"DETECTED",
    b"SIM_INIT",
    b"Card inserted",
    b"USIM inserted",
    b"SIM INSERT",
    b"sitSetApp",
]:
    i = 0
    n = 0
    while n < 3:
        j = img.find(s, i)
        if j < 0:
            break
        ctx = img[j : j + 60]
        print(hex(j), s, ctx[:50])
        i = j + 1
        n += 1

# Dump SET#1 function body briefly
dump(0x146A99C, 0x146AB20, "SET1FN body")
