#!/usr/bin/env python3
"""Caller 0x19d15e0 MOVS#3; switch discriminant; confirm WRAP_A case id."""
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
                if r[0] in (0x7AB, 0x18E, 0x106A, 0xC6B, 0x2C5E):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                t = bl_target(o)
                tag = " <<SWITCH" if t == 0x14B7074 else ""
                extra = f" ;BL->{t:#x}{tag}"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x4600:
            rd = ((hw >> 7) & 1) << 3 | (hw & 7)
            rm = (hw >> 3) & 0xF
            extra = f" ;MOV r{rd},r{rm}"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


print("=== Caller of switch @0x19d15e0 ===")
print(dump(0x19D1580, 0x19D1620))

# Find fn of caller and ITS callers
print("\n=== Who calls 0x19d15xx fn ===")
fn = None
for p in range(0x19D15E0, 0x19D1000, -2):
    if (u16(p) & 0xFFF0) == 0xE92D or u16(p) in (0xB5F0, 0xB570, 0xB5F8, 0xB5B0):
        fn = p
        print(f"fn={p:#x}")
        break
if fn:
    cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == fn]
    print(f"callers of {fn:#x}: {len(cs)}")
    for c in cs[:12]:
        print(f"  @{c:#x}")
        print(dump(c - 0x30, c + 8))

# Switch: how is WRAP_A case selected? Look for LDRB of msg id and CMP near 0x14b774e
print("\n=== Discriminant near WRAP_A case 0x14b7400..0x14b7770 ===")
print(dump(0x14B7400, 0x14B7770))

# Read exact strings at the VA pointers used in cases around WRAP_A
print("\n=== Exact case name strings ===")
cases = [
    (0x8E3, 0x4108, 0xEC0B, 0x4106),  # WRAP_A
    (0x8E3, 0x4108, 0xEF1B, 0x4106),
    (0x8E3, 0x4108, 0xEEE0, 0x4106),
    (0x8E3, 0x4108, 0xEEFB, 0x4106),
    (0x8E3, 0x4108, 0xF937, 0x4106),
]
# Actually re-read MOVT for r0 - is it 0x4108 or something else?
for o in [0x14B7756, 0x14B7738, 0x14B771A, 0x14B76FC, 0x14B7774]:
    hw, hw2 = u16(o), u16(o + 2)
    print(f"  @{o:#x}: {hw:04x} {hw2:04x}")

# Decode: F2C4 1008 = MOVT r0, #0x4108?
# MOVT encoding: F2C0 | i:imm4, then imm3:rd:imm8
# F2C4 = 1111 0010 1100 0100 → i=0, imm4=4? 
# Actually F2C4 1008: rd from hw2 bits 11-8 = 0, imm8=0x08, imm3=1 → 
# Let me just compute like movw helper for movt

def movt_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

for o in range(0x14B774E, 0x14B7762, 2):
    r = movw(o)
    t = movt_imm(o)
    if r:
        print(f"  MOVW r{r[1]},#{r[0]:#x}")
    if t:
        print(f"  MOVT r{t[1]},#{t[0]:#x}")

# Full VA for r1 and read string
r1 = 0x41060000 | 0xEC0B
off = MAIN_OFF + (r1 - VA_BASE)
print(f"\nr1 VA {r1:#x} off {off:#x}")
# show 32 bytes before too in case mid-string
print("  context:", img[off - 16 : off + 48])

# Search for SIM-related case names near that string table
base = off - 0x200
chunk = img[base : base + 0x400]
idx = 0
while True:
    j = chunk.find(b"SIM", idx)
    if j < 0:
        break
    a = j
    while a > 0 and 32 <= chunk[a - 1] < 127:
        a -= 1
    b = j
    while b < len(chunk) and 32 <= chunk[b] < 127:
        b += 1
    print(f"  SIM str @{base+a:#x}: {chunk[a:b].decode()[:80]}")
    idx = j + 1
