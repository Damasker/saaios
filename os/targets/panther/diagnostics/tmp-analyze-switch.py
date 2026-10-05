#!/usr/bin/env python3
"""Identify WRAP_A switch case / message type; arg0=3 meaning; 0x1068 string."""
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


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


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


# 1) Find switch function containing 0x14b7766
print("=== Switch fn containing WRAP_A call ===")
fn = None
for p in range(0x14B7766, 0x14B7000, -2):
    hw = u16(p)
    if (hw & 0xFFF0) == 0xE92D:
        fn = p
        print(f"PUSH.W @{p:#x}")
        break
    if hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xE92D):
        fn = p
        print(f"PUSH @{p:#x}={hw:04x}")

# Dump switch discriminant setup at start
if fn:
    print(f"\n=== prologue {fn:#x} .. +0x80 ===")
    o = fn
    while o < fn + 0xA0:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
            elif (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS #{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP #{hw&0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f"  {o:#x}: {hw:04x}{extra}")
            o += 2

# 2) TBB/TBH switch table near WRAP_A — look for TBB pattern
print("\n=== Look for CMP/switch values leading to WRAP_A case ===")
# Walk backward from 0x14b7766 for BEQ targets / CMP immediates
for o in range(0x14B7600, 0x14B7766, 2):
    hw = u16(o)
    if (hw & 0xFF00) == 0x2800:
        print(f"  CMP r{(hw>>8)&7},#{hw&0xff} @{o:#x}")
    if (hw & 0xF800) == 0xF000 and (u16(o + 2) & 0xD000) == 0x8000:
        # B.W
        pass

# Dump more of switch: find all BL WRAP_A / WRAP_H / similar and preceding CMP
WRAP_A = 0x14C380E
WRAP_H = 0x14C37AC
print("\n=== All CMP immediates in switch fn before each handler BL ===")
# scan 0x14b7200..0x14b7a00 for CMP #N followed eventually by BL to known
handlers = {}
o = 0x14B7200
while o < 0x14B7C00:
    hw = u16(o)
    if (hw & 0xFF00) == 0x2800:
        imm = hw & 0xFF
        # look ahead 0x80 for BL to wrap
        for p in range(o + 2, o + 0x100, 2):
            b = bl_target(p)
            if b in (WRAP_A, WRAP_H, 0x14C5FE6, 0x14F9108, 0x14C4882, 0x14C5E4E, 0x14C5F28, 0x14C6002):
                handlers.setdefault(b, []).append((imm, o, p))
                break
            # stop at another CMP of same style
            if (u16(p) & 0xFF00) == 0x2800 and p > o + 4:
                break
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        o += 4
    else:
        o += 2
for h, lst in sorted(handlers.items()):
    print(f"  handler {h:#x}: CMPs {[x[0] for x in lst]} at {[hex(x[1]) for x in lst]}")

# 3) Resolve string IDs used in WRAP_A case: r1=0xec0b with ctx 0x4106?
# MOVW r1,#0xec0b; MOVT r1,#0x4106 → VA 0x4106EC0B — might be string ptr in MAIN
print("\n=== Case string ptrs (r1 MOVW/MOVT before WRAP_A) ===")
# At 0x14b774e: MOVW r0,#0x8e3; MOVW r1,#0xec0b; MOVT r0,#0x4108; MOVT r1,#0x4106
# String at VA = (MOVT<<16)|MOVW for r1 if it's a pointer
for label, lo, hi in [
    ("WRAP_A_case", 0xEC0B, 0x4106),
    ("prev1", 0xEF1B, 0x4106),
    ("prev2", 0xEEE0, 0x4106),
    ("prev3", 0xEEFB, 0x4106),
]:
    ptr = (hi << 16) | lo
    # Convert VA to file off if in MAIN range
    if VA_BASE <= ptr < VA_BASE + (END - MAIN_OFF):
        off = MAIN_OFF + (ptr - VA_BASE)
        # read string
        if off < len(img) and 32 <= img[off] < 127:
            end = img.find(b"\0", off)
            print(f"  {label} VA {ptr:#x} off {off:#x}: {img[off:end].decode()[:100]}")
        else:
            # maybe it's an offset into string table; try as imm log id style
            print(f"  {label} VA {ptr:#x} not ascii at off")
    else:
        print(f"  {label} VA {ptr:#x} outside MAIN")

# Try r1 values as shannon log string offsets in file (absolute)
for lo in (0xEC0B, 0xEF1B, 0xEEE0, 0xEEFB, 0xF937):
    for base in (0x4D00000, 0x4C00000, 0x400000, 0):
        off = base + lo
        if off + 20 < len(img) and 32 <= img[off] < 127:
            end = img.find(b"\0", off)
            s = img[off:end]
            if b" " in s or b"_" in s or b"SIM" in s or b"Pin" in s or b"USIM" in s:
                print(f"  lo={lo:#x} base={base:#x}: {s[:120]}")

# 4) Log 0x1068 — try (id<<8)|0x44 and also |0x00 variants; search text
print("\n=== Find 0x1068 format string ===")
# Known 0x106a at ~0x4d4f64b — scan nearby hdrs
for off in range(0x4D4F000, 0x4D50000):
    w = u32(off)
    if (w & 0xFF) in (0x44, 0x00, 0x41) and ((w >> 8) & 0xFFFF) == 0x1068:
        s = off + 8
        end = img.find(b"\0", s)
        print(f"  hdr@{off:#x} w={w:#x}: {img[s:end][:140]}")

# search ascii containing both Present and update near builder logs
idx = 0
while True:
    j = img.find(b"Present", idx)
    if j < 0 or j > 0x5000000:
        break
    s0 = max(0, j - 40)
    chunk = img[s0 : j + 80]
    if b"1068" in chunk or b"SIM" in chunk or b"status" in chunk.lower() or b"Status" in chunk:
        # printable run
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        if b - a > 20:
            print(f"  @{a:#x}: {img[a:b].decode()[:140]}")
    idx = j + 1
    if idx > 0x4E00000:
        break

# 5) Callers of switch fn
if fn:
    cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == fn]
    print(f"\n=== Callers of switch {fn:#x}: {len(cs)} ===")
    for c in cs[:15]:
        print(f"  @{c:#x}")
        for b in range(2, 32, 2):
            hw = u16(c - b)
            if (hw & 0xFF00) == 0x2000:
                print(f"    MOVS #{hw&0xff}")
            r = movw(c - b)
            if r and r[0] < 0x100:
                print(f"    MOVW r{r[1]},#{r[0]:#x}")
