#!/usr/bin/env python3
"""Name SET#6 switch message; gate 0x18c2acc; does VerifyPin reach SET#6?"""
import struct
from pathlib import Path
from collections import deque

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)

def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]

def u32(o):
    return struct.unpack_from("<I", IMG, o)[0]

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

# Find string for log id 0x8e3 (hdr style: low byte 0x44, id<<8)
print("=== log 0x8e3 / nearby strings ===")
for off in range(0x4000000, min(len(IMG)-8, 0x5200000), 4):
    w = u32(off)
    if (w & 0xFF) == 0x44 and ((w >> 8) & 0xFFFF) == 0x8E3:
        s = off + 8
        end = IMG.find(b"\x00", s)
        print(f"  hdr@{hex(off)}: {IMG[s:end][:120]!r}")

# Also search ascii
idx = 0
while True:
    j = IMG.find(b"0x8e3", idx)
    if j < 0:
        break
    print(f"  text@{hex(j)}: {IMG[j:j+40]!r}")
    idx = j + 1

# Dump more of switch at 0x14b7b00 to see case values
print("\n=== Dispatcher around 0x14b7cbc (cases) ===")
o = 0x14B7B00
while o < 0x14B7D00:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        b = bl_target(o)
        if b: extra = f" BL->{hex(b)}"
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
        # MOVW r0,#imm often case id
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

# BFS FirstPIN → SET#6 entry 0x14c643c?
print("\n=== FirstPIN BFS reach SET#6 / STATUS_WRAP? ===")
FIRSTPIN = 0x1F0452C
want = {0x14C643C, 0x14C6626, 0x14B7CBC, 0x14FB322, 0x18EC31C}
parent = {FIRSTPIN: None}
q = deque([FIRSTPIN])
found = {}
while q and len(parent) < 100000:
    pc = q.popleft()
    o = pc
    end = min(pc + 0x400, len(IMG) - 4)
    while o < end:
        b = bl_target(o)
        if b is not None:
            if b in want and b not in found:
                found[b] = (pc, o)
            if SCAN_LO <= b < SCAN_HI and b not in parent:
                parent[b] = (pc, o)
                q.append(b)
            o += 4
            continue
        hw = u16(o)
        if (hw & 0xFF00) == 0xBD00 or hw == 0x4770:
            break
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
        else:
            o += 2

for t in sorted(want):
    if t in found:
        # reconstruct
        chain = []
        cur = t
        seen = set()
        while cur is not None and cur not in seen:
            seen.add(cur)
            chain.append(cur)
            p = parent.get(cur)
            if p is None:
                break
            cur = p[0]
        print(f"  REACH {hex(t)} via {' -> '.join(hex(x) for x in reversed(chain))}")
    else:
        print(f"  MISS  {hex(t)}")

# 0x18c2acc full — what does it return for pin states?
print("\n=== Full 0x18c2acc (SET#6 gate) ===")
o = 0x18C2ACC
while o < 0x18C2B80:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        b = bl_target(o)
        if b: extra = f" BL->{hex(b)}"
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
        if (hw & 0xF000) == 0xD000:
            cond = (hw >> 8) & 0xF
            imm = hw & 0xFF
            if imm >= 0x80: imm -= 0x100
            names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
            if cond < 14:
                extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
        if (hw & 0xF800) == 0xE000:
            imm = hw & 0x7FF
            if imm >= 0x400: imm -= 0x800
            extra = f" B->{hex(o+4+imm*2)}"
        if (hw & 0xFF00) == 0xBD00 or hw == 0x4770:
            print(f"  {hex(o)}: {hw:04x}{extra} RET")
            break
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

# Case table for 0x18ec31c — which case index hits SET#7 path
# At 0x18ec360 CMP #25; TBB/TBH at 0x18ec366 e8df f010 = TBB [pc, r0]
print("\n=== 0x18ec31c TBB cases (byte offsets) ===")
# After BHI to default, e8df f010 is TBB [PC, R0]
base = 0x18EC36A  # PC value for TBB is align? For TBB, PC is address of instr+4 = 0x18ec36a
# Actually TBB: PC = addr_of_instr + 4, then PC + 2*table[index]
table = 0x18EC36A
for i in range(26):
    off = IMG[table + i]
    target = table + off * 2
    # interesting if target near SET7 path cases
    mark = ""
    if 0x18EC4DE <= target <= 0x18EC520:
        mark = " *** SET7-ish"
    if 0x18EC464 <= target <= 0x18EC470:
        mark = " *** has MOVS#6 path?"
    print(f"  case {i}: +{off}*2 -> {hex(target)}{mark}")

print("DONE")
