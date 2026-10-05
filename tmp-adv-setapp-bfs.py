#!/usr/bin/env python3
"""Adversarial: SET_APP callers; +0xBF4 writers; STATUS after VerifyPin; Pin1Verified effect."""
import struct
from pathlib import Path
from collections import defaultdict

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

SET_APP = 0x19916D2
STATUS = 0x14FB322
FN_A = 0x14F692C

# All BL SET_APP with preceding MOVS r0,#imm
print("=== ALL BL SET_APP (imm r0 lookback) ===")
hits = defaultdict(list)
o = 0
while o < len(IMG) - 4:
    b = bl_target(o)
    if b == SET_APP:
        imm = None
        for a in range(max(0, o - 16), o, 2):
            h = u16(a)
            if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == 0:
                imm = h & 0xFF
            # MOVW r0,#imm16
            if (h & 0xFBF0) == 0xF240:
                h2 = u16(a + 2)
                if not (h2 & 0x8000) and ((h2 >> 8) & 0xF) == 0:
                    i = (h >> 10) & 1
                    imm = (i << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
        hits[imm].append(o)
    o += 2
for imm in sorted(hits.keys(), key=lambda x: (x is None, x or 0)):
    sites = hits[imm]
    print(f"  imm={imm}: {len(sites)} sites")
    for s in sites[:12]:
        print(f"    {hex(s)}")
    if len(sites) > 12:
        print(f"    ... +{len(sites)-12}")

# STRB.W [r*, #0xBF4] and #0xBF5 #0xBF6
print("\n=== STRB.W to +0xBF4/+0xBF5/+0xBF6 ===")
for off_name, off in (("BF4", 0xBF4), ("BF5", 0xBF5), ("BF6", 0xBF6)):
    found = []
    o = 0
    while o < len(IMG) - 4:
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == off:
            found.append((o, hw & 0xF))
        o += 2
    print(f"  +0x{off_name}: {len(found)} stores")
    for a, rn in found:
        print(f"    {hex(a)} STRB.W [r{rn},#0x{off_name}]")

# BFS from FirstPIN 0x1f0452c: reach STATUS / SET_APP / FN_A / SCHEDULER-like posts?
# Follow BLs depth-limited; record stores of #2
print("\n=== BFS from FirstPIN for STATUS/SET_APP/FN_A ===")
start = 0x1F0452C
queue = [(start, 0)]
seen = {start}
interesting = []
while queue:
    pc, depth = queue.pop(0)
    if depth > 6:
        continue
    # scan ~0x200 bytes of function body approx
    o = pc
    end = min(pc + 0x280, len(IMG) - 4)
    while o < end:
        hw = u16(o)
        b = bl_target(o)
        if b is not None:
            if b in (STATUS, SET_APP, FN_A, 0x14C6626, 0x14FB5C6):
                interesting.append((pc, depth, o, b))
            if b not in seen and depth < 6 and 0x100000 <= b < len(IMG):
                seen.add(b)
                queue.append((b, depth + 1))
            o += 4
            continue
        # POP / BX LR end heuristics
        if hw in (0xBD10, 0xBD30, 0xBD70, 0xBDF0) or (hw & 0xFF00) == 0x4700:
            break
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
        else:
            o += 2

print(f"  visited {len(seen)} funcs")
print(f"  interesting hits: {len(interesting)}")
for item in interesting[:40]:
    print(f"    from={hex(item[0])} d={item[1]} at={hex(item[2])} -> {hex(item[3])}")

# Does Pin1Verified gate appear in STATUS between Present copy and SET_APP#5?
print("\n=== STATUS 0x14fb380 .. 0x14fb5e0 (READY gate) ===")
o = 0x14FB380
while o < 0x14FB5E0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        b = bl_target(o)
        if b: extra = f" BL->{hex(b)}"
        if (hw & 0xFFF0) == 0xF880: extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
        if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
        if (hw & 0xFF00) == 0x2000: pass
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xF800) == 0x7800:
            extra = f" LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        if (hw & 0xF800) == 0x7000:
            extra = f" STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        b = bl_target(o)  # noop for 16
        if (hw & 0xF000) == 0xD000:
            cond = (hw >> 8) & 0xF
            imm = hw & 0xFF
            if imm >= 0x80: imm -= 0x100
            names = "EQ NE CS CC MI PL VS VC HI LS GE LT LE".split()
            if cond < 14:
                extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
        if (hw & 0xF800) == 0xE000:
            imm = hw & 0x7FF
            if imm >= 0x400: imm -= 0x800
            extra = f" B->{hex(o+4+imm*2)}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

print("DONE")
