#!/usr/bin/env python3
"""Who posts PAUSE_REQ; callers of 0x19d1548; msg id; AP levers."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)
VA_BASE, MAIN_OFF = 0x40010000, 0x16C10

def u16(o): return struct.unpack_from("<H", IMG, o)[0]
def u32(o): return struct.unpack_from("<I", IMG, o)[0]

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

def dump(start, nbytes=0x80, lab=""):
    print(f"\n=== {lab} ===")
    o = start
    end = start + nbytes
    while o < end:
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
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

# Decode TBB to find which case index is PAUSE_REQ (ADR at 0x14b7cb2)
# TBB at 0x14b7274: e8df f010; table at 0x14b7278
# case index from: MOVW r0,#0x9040; SUB r0,r9,r0; CMP #49
print("=== TBB cases 0..49 (interesting) ===")
table = 0x14B7278
for i in range(50):
    off = IMG[table + i]
    tgt = table + off * 2
    # check if within 0x20 of known case labels
    mark = ""
    if abs(tgt - 0x14B7CB2) < 8 or abs(tgt - 0x14B7CBC) < 0x20:
        mark = " *** SET#6/PAUSE"
    if abs(tgt - 0x14B7CA8) < 8 or abs(tgt - 0x14B7C94) < 0x20:
        mark = " *** MEASURE_RSP/STATUS?"
    if mark or i < 5 or tgt in range(0x14B7C80, 0x14B7CE0):
        print(f"  case {i}: -> {hex(tgt)}{mark}")

# Exact: which case lands on 0x14b7cae (PAUSE case start)?
print("\n=== case targeting PAUSE region ===")
for i in range(50):
    off = IMG[table + i]
    tgt = table + off * 2
    if 0x14B7CA0 <= tgt <= 0x14B7CD0:
        msg_id = 0x9040 + i  # from SUB r0, r9, #0x9040
        print(f"  case {i} msg_id≈{hex(msg_id)} -> {hex(tgt)}")

# Callers of 0x19d1548 context
for site in (0x14AE31C, 0x194FF56, 0x1A2C700):
    dump(site - 0x30, 0x60, f"caller site {hex(site)}")

# Find strings near 0x19d1548 / 0x14ae31c for naming
print("\n=== MOVW log near 0x19d1548 ===")
o = 0x19D1548
while o < 0x19D15E0:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        if imm < 0x3000:
            print(f"  {hex(o)} MOVW r{(hw2>>8)&0xf},#{hex(imm)}")
    o += 2

# Search producer: "SADR_GAP_MEASURE_PAUSE" in code as send - look for string at 0x261ad7d
# Maybe it's in a message table (array of string ptrs). Search u32 file-offset or VA.
PROD = 0x261AD7D
va = VA_BASE + (PROD - MAIN_OFF)
print(f"\n=== literals to alt PAUSE VA {hex(va)} / file {hex(PROD)} ===")
count = 0
o = 0
while o < len(IMG) - 4 and count < 30:
    w = u32(o)
    if w == va or w == PROD:
        print(f"  @{hex(o)} = {hex(w)}")
        count += 1
    o += 4

# Consumer string at 0x14b7d50 - already only ADR from case
# Search PAUSE_DONE / MEASURE_REQ with MOVW/MOVT to their VAs
for name, fo in [
    ("PAUSE_DONE_CNF", 0x1000E20),
    ("SADR_MEASURE_REQ", 0x100235D),
    ("SADR_MEASURE_RSP_alt", 0x261AE59),
    ("PAUSE_alt", 0x261AD7D),
]:
    va = VA_BASE + (fo - MAIN_OFF)
    lo, hi = va & 0xFFFF, (va >> 16) & 0xFFFF
    hits = []
    o = SCAN_LO
    while o < SCAN_HI - 8:
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            rd = (hw2 >> 8) & 0xF
            if imm == lo:
                for a in range(o + 4, min(o + 12, SCAN_HI - 4), 2):
                    h, h2 = u16(a), u16(a + 2)
                    if (h & 0xFBF0) == 0xF2C0 and not (h2 & 0x8000) and ((h2 >> 8) & 0xF) == rd:
                        i2 = (h >> 10) & 1
                        himm = (i2 << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
                        if himm == hi:
                            hits.append(o)
                        break
        o += 2
    print(f"  {name} VA {hex(va)}: {[hex(h) for h in hits[:15]]}")

# Preferred/band / SIT strings near SADR
print("\n=== SADR-related log strings (sample) ===")
for n in [b"SADR_GAP", b"Meas_Paused", b"already Meas", b"SADR measure", b"GapMeasure"]:
    idx = 0
    c = 0
    while c < 8:
        j = IMG.find(n, idx)
        if j < 0:
            break
        s0 = j
        while s0 > 0 and 32 <= IMG[s0-1] < 127:
            s0 -= 1
        end = IMG.find(b"\x00", j)
        print(f"  {hex(s0)}: {IMG[s0:min(end,s0+90)]!r}")
        idx = j + 1
        c += 1

# Who calls 0x14ae31c's function - radio on path?
print("\n=== funcs for dispatcher-parent callers ===")
for site in (0x14AE31C, 0x194FF56, 0x1A2C700):
    o = site
    while o > site - 0x200:
        if u16(o) == 0xE92D and (u16(o+2) & 0x4000):
            print(f"  {hex(site)} in func {hex(o)}")
            # BL to this func
            entry = o
            callers = []
            a = SCAN_LO
            while a < SCAN_HI and len(callers) < 15:
                if bl_target(a) == entry:
                    callers.append(a)
                a += 2
            print(f"    callers: {[hex(c) for c in callers]}")
            break
        o -= 2

print("DONE")
