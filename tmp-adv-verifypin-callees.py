#!/usr/bin/env python3
"""Inspect MOVS#2+STRB hits on VerifyPin BFS path; STATUS after SimInfo."""
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
    while o < end and n < 50:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b is not None:
                extra = f" BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            # MOVW
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                rt, rn, imm = hw & 7, (hw >> 3) & 7, (hw >> 6) & 0x1F
                extra = f" STRB r{rt},[r{rn},#{imm}]"
            if (hw & 0xFF00) == 0x2800:
                extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
        n += 1

# Suspect MOVS#2 sites from BFS
for s in (0x1d76e60, 0x1d76f00, 0x1d76f58, 0x18dce70, 0x1f90700, 0x1f2e540, 0x1dcb020):
    dump(s, s + 0x40, f"site@{hex(s)}")

# SimInfo callee 0x1dcb028 — does it post STATUS / scheduler msg?
dump(0x1dcb028, 0x1dcb028 + 0x100, "sitSendNsSimInfo-ish 0x1dcb028")

# After FirstPIN STRB Pin1V, BL 0x1f1458c
dump(0x1f1458c, 0x1f1458c + 0x80, "BL target 0x1f1458c")

# Check if any of MOVS#2 STRB use getobj 636c in same function (±0x200)
def has_636c(around):
    for o in range(max(MAIN := 0x16c10, around - 0x200), around + 0x200, 2):
        if o + 4 > len(IMG):
            break
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                if imm == 0x636C:
                    return hex(o)
            # skip
    return None

print("\n=== 636c near MOVS#2 suspects ===")
for s in (0x1d76e6c, 0x1d76f0a, 0x1d76f66, 0x18dce80, 0x1f90716, 0x1f2e556):
    print(hex(s), "636c@", has_636c(s))

# Corrected Pin1V STRB encoding check at FirstPIN
print("\n=== FirstPIN Pin1V store decode ===")
for o in (0x1f04576, 0x1f046a4):
    hw = u16(o)
    print(hex(o), f"{hw:04x}", "MOVS")
    hw = u16(o + 2)
    # STRB T1
    rt, rn, imm = hw & 7, (hw >> 3) & 7, (hw >> 6) & 0x1F
    print(hex(o + 2), f"{hw:04x}", f"STRB r{rt},[r{rn},#{imm}]")

print("DONE")
