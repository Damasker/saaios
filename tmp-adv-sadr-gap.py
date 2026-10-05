#!/usr/bin/env python3
"""RO: who posts SADR_GAP_MEASURE_PAUSE_REQ; conditions; AP levers."""
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

def va_to_off(va):
    return MAIN_OFF + (va - VA_BASE)

# 1) Find all SADR_GAP string sites
NEEDLES = [
    b"SADR_GAP_MEASURE_PAUSE_REQ",
    b"SADR_GAP_MEASURE_RESUME_IND",
    b"SADR_GAP_MEASURE",
    b"SADR_MEASURE",
]
print("=== SADR strings ===")
sites = {}
for n in NEEDLES:
    idx = 0
    hits = []
    while True:
        j = IMG.find(n, idx)
        if j < 0:
            break
        # back up to start of identifier
        s0 = j
        while s0 > 0 and (IMG[s0-1] in b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_"):
            s0 -= 1
        end = IMG.find(b"\x00", j)
        hits.append((s0, IMG[s0:end].decode("ascii", "replace")))
        idx = j + 1
    sites[n.decode()] = hits
    for s0, s in hits[:12]:
        print(f"  {hex(s0)}: {s}")

PAUSE = b"MMCIF_L1LC_DSL1C_SADR_GAP_MEASURE_PAUSE_REQ"
pause_offs = []
idx = 0
while True:
    j = IMG.find(PAUSE, idx)
    if j < 0:
        break
    pause_offs.append(j)
    idx = j + 1
print(f"\nPAUSE_REQ file offs: {[hex(x) for x in pause_offs]}")

# 2) Find code refs: ADR to local string, or MOVW/MOVT VA pointing at string
# Local ADR at 0x14b7cb2 already known -> 0x14b7d50
# Search for PC-relative and absolute refs to pause_offs

def find_adr_to(target, window_lo=SCAN_LO, window_hi=SCAN_HI):
    """Thumb ADR: 10100 Rd imm8 -> (pc+4)&~3 + imm8*4"""
    hits = []
    o = window_lo
    while o < window_hi - 2:
        hw = u16(o)
        if (hw & 0xF800) == 0xA000:
            rd = (hw >> 8) & 7
            imm = (hw & 0xFF) * 4
            base = (o + 4) & ~3
            if base + imm == target:
                hits.append(o)
        o += 2
    return hits

print("\n=== ADR refs to each PAUSE string ===")
for po in pause_offs:
    ads = find_adr_to(po)
    print(f"  target {hex(po)}: {[hex(a) for a in ads[:20]]}")

# VA refs: string at file off -> VA = VA_BASE + (off - MAIN_OFF)
print("\n=== MOVW/MOVT r? pairs pointing at PAUSE VA ===")
for po in pause_offs:
    va = VA_BASE + (po - MAIN_OFF)
    lo16, hi16 = va & 0xFFFF, (va >> 16) & 0xFFFF
    print(f"  VA {hex(va)} lo={hex(lo16)} hi={hex(hi16)}")
    o = SCAN_LO
    found = []
    while o < SCAN_HI - 8:
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            rd = (hw2 >> 8) & 0xF
            if imm == lo16:
                # look ahead for MOVT same rd
                for a in range(o + 4, min(o + 16, SCAN_HI - 4), 2):
                    h, h2 = u16(a), u16(a + 2)
                    if (h & 0xFBF0) == 0xF2C0 and not (h2 & 0x8000) and ((h2 >> 8) & 0xF) == rd:
                        i2 = (h >> 10) & 1
                        himm = (i2 << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
                        if himm == hi16:
                            found.append(o)
                        break
        o += 2
    print(f"    sites: {[hex(x) for x in found[:30]]}")

# 3) Who BLs the dispatcher that contains SET#6 case (find func start of 0x14b7xxx)
# Walk back for PUSH.W from 0x14b7cbc
print("\n=== Dispatcher func containing SET#6 ===")
o = 0x14B7CBC
while o > 0x14B7000:
    hw = u16(o)
    if hw == 0xE92D:
        print(f"  PUSH.W @{hex(o)} mask={u16(o+2):04x}")
        if o < 0x14B7A00:
            break
    o -= 2

# Find BLs to likely dispatcher entry - search common pattern near first case
# Dump from earliest PUSH
FUNC = None
o = 0x14B7CBC
while o > 0x14B6800:
    if u16(o) == 0xE92D and (u16(o+2) & 0x4000):  # lr saved
        FUNC = o
        break
    o -= 2
print(f"  candidate entry: {hex(FUNC) if FUNC else None}")

if FUNC:
    print(f"\n=== BL -> {hex(FUNC)} ===")
    o = SCAN_LO
    callers = []
    while o < SCAN_HI:
        if bl_target(o) == FUNC:
            callers.append(o)
        o += 2
    print(f"  {len(callers)} callers: {[hex(c) for c in callers[:40]]}")

# Also BL to mid-switch is unlikely; find switch on msg id - look for CMP before TBB
print("\n=== Dump dispatcher entry 0x80 ===")
start = FUNC or 0x14B7900
o = start
end = start + 0x100
while o < end:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        b = bl_target(o)
        if b: extra = f" BL->{hex(b)}"
        if hw == 0xE8DF: extra = f" TBB/H {hw2:04x}"
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

# 4) Search who *produces*/posts PAUSE_REQ - string "post" or send near SADR_GAP in producer
# Look for BL that load PAUSE string as arg0 to a post/send helper
print("\n=== Code near other SADR_GAP string sites (producer?) ===")
for key, hits in sites.items():
    if "PAUSE" not in key and key != "SADR_GAP_MEASURE":
        continue
    for s0, s in hits:
        if s0 in pause_offs and 0x14B7D00 <= s0 <= 0x14B7E00:
            continue  # consumer label pool
        # scan ±0x40 for code-looking? strings are in rodata; find xrefs via ADR in wider range
        ads = find_adr_to(s0, 0x100000, min(len(IMG), 0x5800000))
        if ads:
            print(f"  ADR->{hex(s0)} ({s[:50]}): {[hex(a) for a in ads[:15]]}")

# Literal pool: search for file-offset pointers or VA pointers to PAUSE in data
print("\n=== u32 VA literals == PAUSE VA ===")
for po in pause_offs:
    va = VA_BASE + (po - MAIN_OFF)
    # also raw file ptr unlikely
    count = 0
    o = 0
    while o < len(IMG) - 4 and count < 20:
        if u32(o) == va:
            print(f"  lit@{hex(o)} -> VA {hex(va)}")
            count += 1
        o += 4

print("DONE")
