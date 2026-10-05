#!/usr/bin/env python3
"""FN_B Present=2 gates; SIM-internal FN_A caller; when [r5,#8]==3."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
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


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = f" ;LOG" if r[0] in (0x7AB, 0x18E, 0x106A, 0x1068, 0xC6B, 0x2C5E) else ""
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS #{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP #{hw&0xff}"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xFF00) == 0x4600:
            rd = ((hw >> 7) & 1) << 3 | (hw & 7)
            rm = (hw >> 3) & 0xF
            extra = f" ;MOV r{rd},r{rm}"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


# FN_B Present=2 at 0x14f9578 — full gates from fn start
print("=== FN_B 0x14f9108 .. Present=2 ===")
print(dump(0x14F9108, 0x14F95A0))

# Caller of FN_B
print("\n=== FN_B caller 0x14c5fe6 context ===")
print(dump(0x14C5F80, 0x14C6010))

# Case name for FN_B caller path if in same switch
# 0x14c5fe6 was listed as FN_B caller - check if from switch
print("\n=== Is 0x14c5fe6 in switch? search refs ===")
# Find who BLs 0x14c5fe6 or if it's a handler entry from switch
# Switch had BL 0x14c6002 nearby - related
for o in range(0x14B7600, 0x14B7800, 2):
    b = bl_target(o)
    if b and 0x14C5F00 <= b <= 0x14C6100:
        # get r1 string
        for p in range(o - 0x20, o, 2):
            r = movw(p)
            if r and r[1] == 1:
                lo = r[0]
                # find MOVT r1
                for q in range(p, o, 2):
                    hw, hw2 = u16(q), u16(q + 2)
                    if (hw & 0xFBF0) == 0xF2C0 and ((hw2 >> 8) & 0xF) == 1 and (hw2 & 0x8000) == 0:
                        i = (hw >> 10) & 1
                        hi = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                        ptr = (hi << 16) | lo
                        off = MAIN_OFF + (ptr - VA_BASE)
                        if 0 <= off < len(img):
                            end = img.find(b"\0", off)
                            print(f"  BL->{b:#x} case str: {img[off:end].decode()[:80]}")
                        break

# For internal FN_A caller fn at 0x14f6d02: who calls it?
print("\n=== Callers of SIM-internal FN_A wrapper 0x14f6d02 ===")
fn = 0x14F6D02
cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == fn]
print(f"count={len(cs)}: {[hex(c) for c in cs[:15]]}")
for c in cs[:8]:
    print(f"\n---- @{c:#x} ----")
    print(dump(c - 0x40, c + 0x10))

# Summary strings: Pin1Status enum values?
print("\n=== Pin status / present enum strings ===")
for n in [
    b"Pin1Status",
    b"PIN1_STATUS",
    b"PIN_STATUS_",
    b"SimPinStatus",
    b"Present = 2",
    b"Present=2",
    b"PRESENT_VERIFIED",
    b"SIM_PRESENT",
    b"SIMSTART",
    b"SIM START IND",
]:
    j = img.find(n)
    if j >= 0:
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        print(f"  @{a:#x}: {img[a:b].decode()[:120]}")
