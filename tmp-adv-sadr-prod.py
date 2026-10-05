#!/usr/bin/env python3
"""Trace dispatcher caller 0x19d15e0; LTE SADR producers; other Present=2 candidates."""
import struct
from pathlib import Path
from collections import deque

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

def dump(start, nbytes=0xA0, lab=""):
    print(f"\n=== {lab} @ {hex(start)} ===")
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
            if (hw & 0xFFF0) == 0xF880: extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400: imm -= 0x800
                extra = f" B->{hex(o+4+imm*2)}"
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80: imm -= 0x100
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14:
                    extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

# Caller of dispatcher
dump(0x19D15A0, 0x80, "caller BL dispatcher 0x19d15e0")

# Find func containing 0x19d15e0
o = 0x19D15E0
FUNC = None
while o > 0x19D1000:
    if u16(o) == 0xE92D and (u16(o+2) & 0x4000):
        FUNC = o
        break
    o -= 2
print(f"\nfunc containing dispatcher BL: {hex(FUNC) if FUNC else '?'}")

if FUNC:
    print(f"=== BL -> {hex(FUNC)} ===")
    o = SCAN_LO
    cs = []
    while o < SCAN_HI:
        if bl_target(o) == FUNC:
            cs.append(o)
        o += 2
    print(f"  {[hex(c) for c in cs[:40]]}")

# How does switch select PAUSE_REQ? Look at msg id compare before case
# At 0x14b7126: LDRB [r11,#8]; CMP #6 — maybe subtype
# Need TBB / switch on message name hash or id from 0x18d9b34 logger
# Search for switch table near 0x14b7170+

dump(0x14B7170, 0x120, "dispatcher switch after CMP#6")

# Producer: ADR/refs to 0x261ad7d MMCIF_L1LC_SADR_GAP_MEASURE_PAUSE_REQ
PROD = 0x261AD7D
print(f"\n=== refs to alt PAUSE string {hex(PROD)} ===")
# ADR
o = SCAN_LO
adr_hits = []
while o < SCAN_HI - 2:
    hw = u16(o)
    if (hw & 0xF800) == 0xA000:
        imm = (hw & 0xFF) * 4
        base = (o + 4) & ~3
        if base + imm == PROD:
            adr_hits.append(o)
    o += 2
print(f"  ADR: {[hex(a) for a in adr_hits[:20]]}")

va = VA_BASE + (PROD - MAIN_OFF)
lo16, hi16 = va & 0xFFFF, (va >> 16) & 0xFFFF
print(f"  VA {hex(va)}")
o = SCAN_LO
mov_hits = []
while o < SCAN_HI - 8:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        rd = (hw2 >> 8) & 0xF
        if imm == lo16:
            for a in range(o + 4, min(o + 16, SCAN_HI - 4), 2):
                h, h2 = u16(a), u16(a + 2)
                if (h & 0xFBF0) == 0xF2C0 and not (h2 & 0x8000) and ((h2 >> 8) & 0xF) == rd:
                    i2 = (h >> 10) & 1
                    himm = (i2 << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
                    if himm == hi16:
                        mov_hits.append(o)
                    break
    o += 2
print(f"  MOVW/T: {[hex(m) for m in mov_hits[:30]]}")

# SADR_MEASURE_REQ producer strings
for label, off in [
    ("SADR_MEASURE_REQ", 0x100235D),
    ("PAUSE_DONE_CNF", 0x1000E20),
    ("SADR_MEASURE_RSP", 0x1076228),
]:
    print(f"\n--- ADR to {label} @{hex(off)} ---")
    o = SCAN_LO
    hits = []
    while o < min(SCAN_HI, 0x3000000) - 2:
        hw = u16(o)
        if (hw & 0xF800) == 0xA000:
            imm = (hw & 0xFF) * 4
            base = (o + 4) & ~3
            if base + imm == off:
                hits.append(o)
        o += 2
    print(f"  {[hex(h) for h in hits[:20]]}")
    for h in hits[:5]:
        dump(max(SCAN_LO, h - 0x20), 0x50, f"near ADR {hex(h)}")

# Present=2 candidate 0x14f900a context
dump(0x14F8FE0, 0x80, "MOVS#2 STRB @0x14f900a region")
dump(0x14F9560, 0x40, "FN_B Present=2 region")

# Check getobj sites for STRB #0 = 2 after get
print("\n=== After getobj #636c: nearby MOVS#2 STRB? ===")
for go in (0x1551CF4, 0x1551F8A, 0x1553416, 0x1A175EA, 0x1A347E4, 0x1A55204):
    found = False
    for a in range(go, go + 0x100, 2):
        h = u16(a)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
            for b in range(a + 2, a + 12, 2):
                h2 = u16(b)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0:
                    print(f"  {hex(go)}: MOVS#2@{hex(a)} STRB@{hex(b)}")
                    found = True
    if not found:
        # any STRB [r,#0] with MOVS 0/1/2/3
        vals = []
        for a in range(go, go + 0xC0, 2):
            h = u16(a)
            if (h & 0xFF00) == 0x2000 and (h & 0xFF) <= 3:
                rd = (h >> 8) & 7
                for b in range(a + 2, a + 10, 2):
                    h2 = u16(b)
                    if (h2 & 0xF800) == 0x7000 and (h2 & 7) == rd and ((h2 >> 6) & 0x1F) == 0:
                        vals.append((h & 0xFF, a))
        print(f"  {hex(go)}: Present-ish stores {vals[:8]}")

print("DONE")
