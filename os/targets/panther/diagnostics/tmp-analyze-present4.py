#!/usr/bin/env python3
"""Gate analysis for Present=2 writers at 0x14f6a14 and 0x14f9578; FCP link."""
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
                note = ""
                if r[0] in (0x1068, 0x1069, 0x106A, 0x18E, 0x7AB, 0xC6B, 0x2C5E, 0xBDA):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            elif (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF000) == 0xD000 and (hw & 0x0F00) < 0x0E00:
            imm8 = hw & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            lines.append(f"  {o:#x}: {hw:04x} ;Bcond->{o+4+(imm8<<1):#x}")
            o += 2
            continue
        elif (hw & 0xF800) == 0xE000:
            imm11 = hw & 0x7FF
            if imm11 >= 0x400:
                imm11 -= 0x800
            lines.append(f"  {o:#x}: {hw:04x} ;B->{o+4+(imm11<<1):#x}")
            o += 2
            continue
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


# Wider context for Present=2 @ 0x14f6a14 — find what CMP gates it
print("=== Present=2 writer A 0x14f6900..0x14f6a80 ===")
print(dump(0x14F6900, 0x14F6A80))

print("\n=== Present=2 writer B 0x14f9480..0x14f95a0 ===")
print(dump(0x14F9480, 0x14F95A0))

# Find ALL MOVS#2 STRB.W [rN,#0] in 0x14f0000..0x1500000 (SIM STATUS module)
print("\n=== All Present=#imm STRB.W [rN,#0] in SIM module 0x14f0000-0x1500000 ===")
for o in range(0x14F0000, 0x1500000, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000:
        continue
    val = hw & 0xFF
    if val > 5:
        continue
    rt = (hw >> 8) & 7
    for q in range(o + 2, o + 12, 2):
        h, h2 = u16(q), u16(q + 2)
        if (h & 0xFFF0) == 0xF880 and ((h2 >> 12) & 0xF) == rt and (h2 & 0xFFF) == 0:
            print(f"  Present={val} STRB.W [r{h&0xf},#0] @{o:#x}/va{va(o):#x}")
            break

# Log 0x1068 string
print("\n=== log 0x1068 string ===")
for off in range(0x4D4E000, 0x4D52000):
    w = struct.unpack_from("<I", img, off)[0]
    if (w & 0xFF) == 0x44 and ((w >> 8) & 0xFFFF) == 0x1068:
        s = off + 8
        if 32 <= img[s] < 127:
            end = img.find(b"\0", s)
            print(f"  @{s:#x}: {img[s:end].decode()[:160]}")
        break

# Does FCP path (0x1e0c2fe) ever BL into 0x14f6xxx / 0x14f9xxx?
print("\n=== FCP body BLs into 0x14f0000-0x1500000? ===")
for o in range(0x1E0C174, 0x1E0C600, 2):
    b = bl_target(o)
    if b and 0x14F0000 <= b <= 0x1500000:
        print(f"  FCP @{o:#x} BL->{b:#x}")

# Does VerifyPin success (after 0x18e) BL into Present=2 region?
print("\n=== Pin1Verified fn BLs into 0x14f0000-0x1500000? ===")
for o in range(0x1F04400, 0x1F04800, 2):
    b = bl_target(o)
    if b and 0x14F0000 <= b <= 0x1500000:
        print(f"  pin1v @{o:#x} BL->{b:#x}")

# Callers of Present=2 sites' containing functions
# Find PUSH before 0x14f6a14
print("\n=== Callers of fn containing Present=2 @0x14f6a14 ===")
fn_a = None
for p in range(0x14F6A14, 0x14F6800, -2):
    if (u16(p) & 0xFFF0) == 0xE92D or u16(p) in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0):
        fn_a = p
        print(f"  fn_a={p:#x}")
        break
if fn_a:
    cs = [o for o in range(MAIN_OFF, min(END, MAIN_OFF + 0x3000000), 2) if bl_target(o) == fn_a]
    print(f"  callers: {len(cs)} {[hex(c) for c in cs[:12]]}")
