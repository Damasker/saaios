#!/usr/bin/env python3
"""Analyze SET#1 getobj#0x10 Present=2 candidate + helper 0x1991838."""
from __future__ import annotations
import struct
from pathlib import Path

PATH = Path(__file__).resolve().parent / "fw" / "saaios-probe-b-modem.bin"
MAIN = 0x16C10
END = MAIN + 0x05917ACC
VA0 = 0x40010000
img = PATH.read_bytes()
GETOBJ = 0x20EA040
SET_APP = 0x19916D2
FN_A = 0x14F692C
STATUS = 0x14FB322
HELPER = 0x1991838
SET1 = 0x146A99C


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA0 + (o - MAIN)


def off_va(v):
    return MAIN + (v - VA0)


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


def cstr(v, lim=100):
    o = off_va(v)
    if o < 0 or o >= len(img):
        return None
    b = o
    while b < o + lim and 32 <= img[b] < 127:
        b += 1
    if b == o:
        return None
    return img[o:b].decode("ascii", "replace")


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                extra = f" ;BL->{hex(bt)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


print("=== SET#1 full fn 0x146a99c .. past getobj#0x10 store ===")
print(dump(0x146A99C, 0x146AC80))

print("\n=== Helper 0x1991838 (near SET_APP) ===")
print(dump(0x19916D2, 0x19918C0))

print("\n=== Callers of helper 0x1991838 ===")
cs = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == HELPER]
print(f"n={len(cs)}: {list(map(hex, cs[:30]))}")

print("\n=== getobj(#0x10) sites that STRB #2 to [obj,#0] ===")
# find MOVW/MOVS r1,#0x10 near getobj then later MOVS#2 STRB
for o in range(MAIN, END - 8, 2):
    # Thumb16 MOVS r1,#0x10 = 0x2110
    if u16(o) != 0x2110:
        continue
    # BL getobj within 16B
    got = None
    for p in range(o, min(END, o + 0x20), 2):
        if bl_target(p) == GETOBJ:
            got = p
            break
    if not got:
        continue
    # MOVS#2 STRB [*,#0] within 0x40 after
    store = None
    for p in range(got, min(END, got + 0x50), 2):
        h = u16(p)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
            # next STRB?
            for q in range(p + 2, min(END, p + 8), 2):
                hh = u16(q)
                if (hh & 0xF800) == 0x7000 and ((hh >> 6) & 0x1F) == 0:
                    store = (p, q)
                    break
        if store:
            break
    if store:
        print(f"  getobj@ {hex(got)} MOVS#2@{hex(store[0])} STRB@{hex(store[1])}")
        print(dump(o - 8, store[1] + 0x20))

print("\n=== String at 0x4107042c (debug tag near getobj#0x10) ===")
print(repr(cstr(0x4107042C)))
print(repr(cstr(0x410740B7)))

print("\n=== Does helper 0x1991838 or SET#1 touch PresentObj #636c / FN_A / STATUS? ===")
for window in ((0x146A99C, 0x146AD00), (0x19916D2, 0x1991900)):
    print(f"\n-- window {hex(window[0])} --")
    for p in range(window[0], window[1], 2):
        bt = bl_target(p)
        if bt in (FN_A, STATUS, 0x14C6626, GETOBJ, SET_APP, HELPER):
            print(f"  BL {hex(p)}->{hex(bt)}")
        r = movw(p)
        if r and r[0] == 0x636C:
            print(f"  MOVW #636c @{hex(p)}")

print("\n=== STATUS_WRAP case before SADR_MEASURE_RSP: exact case name for STATUS ===")
# At 0x14b7c94 SADR_MEASURE_RSP then STATUS_WRAP - confirm prior case
for o in range(0x14B7C90, 0x14B7CB0, 2):
    r = movw(o)
    if not r:
        continue
    for p in range(o, o + 12, 2):
        t = movt(p)
        if t and t[1] == r[1]:
            s = cstr((t[0] << 16) | r[0])
            if s:
                print(f"  {hex(o)}: {s}")

# What does ADD r0,r11,#8 produce? Need fn prolog for r11
print("\n=== Dispatcher fn containing 0x14b7ca8 — r11 setup ===")
fn = None
for p in range(0x14B7CA8, 0x14B7CA8 - 0x800, -2):
    h = u16(p)
    if (h & 0xFFF0) == 0xE92D:
        fn = p
        break
print(f"fn={hex(fn) if fn else None}")
if fn:
    print(dump(fn, fn + 0x80))

print("\nDONE")
