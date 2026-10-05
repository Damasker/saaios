#!/usr/bin/env python3
"""How PAUSE case is reached; SR_IF poster; LTE-only gates."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)

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

def b_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    # B.W T4
    if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x9000:
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
    return None

def bcc_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0x8000:
        return None
    s = (hw >> 10) & 1
    cond = (hw >> 6) & 0xF
    imm6 = hw & 0x3F
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    imm32 = (s << 20) | (j2 << 19) | (j1 << 18) | (imm6 << 12) | (imm11 << 1)
    if s:
        imm32 -= 1 << 21
    return o + 4 + imm32

PAUSE_CASE = 0x14B7CAE  # ADR + BL SET#6 region start ~0x14b7cae
TARGETS = set(range(0x14B7CA0, 0x14B7CD8, 2))

print("=== Branches into PAUSE/SET#6 case ===")
o = 0x14B7074
end = 0x14B7CAE
hits = []
while o < end:
    hw = u16(o)
    # short B
    if (hw & 0xF800) == 0xE000:
        imm = hw & 0x7FF
        if imm >= 0x400: imm -= 0x800
        t = o + 4 + imm * 2
        if t in TARGETS:
            hits.append((o, f"B->{hex(t)}"))
    if (hw & 0xF000) == 0xD000:
        imm = hw & 0xFF
        if imm >= 0x80: imm -= 0x100
        t = o + 4 + imm * 2
        if t in TARGETS:
            hits.append((o, f"Bcc->{hex(t)}"))
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        bw = b_w(o)
        bc = bcc_w(o)
        if bw in TARGETS:
            hits.append((o, f"B.W->{hex(bw)}"))
        if bc in TARGETS:
            hits.append((o, f"Bcc.W->{hex(bc)}"))
        o += 4
    else:
        o += 2
print(f"  {len(hits)} hits")
for h in hits[:40]:
    print(f"  {hex(h[0])}: {h[1]}")

# Message id compare: search CMP with imm near PAUSE - or hash table
# 0x18d9b34 is called with string - maybe it's strcmp switch via registered handlers
# Look at registration of PAUSE handler

# Xref BL 0x14c643c already sole from 0x14b7cbc
# Find msg id constant used when posting PAUSE - search string decode tables

print("\n=== SR_IF PAUSE log site xrefs (0x261ad68) ===")
# find nearby code that references this via ADR in 0x2600000-0x2700000? string in rodata
# Search MOVW to nearby - use find of unique prefix in code as LDR literal pools
sva = 0x40010000 + (0x261AD68 - 0x16C10)
print(f"  VA {hex(sva)}")

# Search "GapMeasurePause" / GetIsGapMeasurePause code
needle = b"GetIsGapMeasurePause should use only LTE"
j = IMG.find(needle)
print(f"\n=== GetIsGapMeasurePause string @{hex(j) if j>=0 else 0} ===")
# find code referencing - hard. Search function by string xref via MOVW of nearby log id

# Search symbol-like: GapMeasurePause as ascii in code region comments already have RSM
for n in [b"GapMeasurePause", b"IsGapMeasurePause", b"SADR_GAP_MEASURE_PAUSE", b"PostSadr", b"SendSadr"]:
    idx = 0
    c = 0
    while c < 6:
        k = IMG.find(n, idx)
        if k < 0:
            break
        s0 = max(0, k - 20)
        # show if looks like code log
        frag = IMG[k:k+60]
        if all(32 <= b < 127 or b == 0 for b in frag[:40]):
            end = IMG.find(b"\x00", k)
            print(f"  {hex(k)}: {IMG[k:min(end,k+80)]!r}")
            c += 1
        idx = k + 1

# Preferred/SIT: any HAL/SIT string mentioning SADR or GapMeasure?
print("\n=== sit/HAL-ish near SADR (in known sit ranges?) ===")
# just search Build* Gap or SADR in first 0x200000 of... actually full
for n in [b"SADR", b"GapMeasure", b"GAP_MEASURE"]:
    # only in paths that look like sit
    pass

# Dump 0x14b7a00 area - how do we jump to cases? Look for computed goto from msg
# Read LDRB msg id at start and compare chain to 0x14b7cae
print("\n=== Search CMP imm matching msg for PAUSE ===")
# From STATUS case MOVW path, msg names registered - try find table of {id, handler}
# Handler 0x14c643c in a table?
SET6 = 0x14C643C
print(f"u32 literals == SET6 func:")
c = 0
o = 0
while o < len(IMG) - 4 and c < 20:
    # Thumb func addr often |1
    w = u32(o)
    if w == SET6 or w == (SET6 | 1) or w == (0x40010000 + SET6 - 0x16C10) or w == ((0x40010000 + SET6 - 0x16C10) | 1):
        print(f"  @{hex(o)} = {hex(w)}")
        c += 1
    o += 4

# Callers of SET#6's parent dispatcher via message: 0x14ae242 - dump name strings
print("\n=== strings / logs in 0x14ae242 ===")
o = 0x14AE242
while o < 0x14AE400:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        if imm < 0x2000:
            print(f"  {hex(o)} MOVW #{hex(imm)}")
    o += 2

# Resolve VA string at 0x4106f618 style for parent - check 0x14ae2ec area MOVW
# Already have 0x18b4 - 

# Key: direction string analysis
print("\n=== Direction summary from log prefixes ===")
for n in [
    b"SADR_GAP_MEASURE_PAUSE_REQ",
    b"SADR_GAP_MEASURE_RESUME_IND",
    b"SADR_GAP_MEASURE_PAUSE_DONE_CNF",
    b"SADR_MEASURE_REQ",
    b"SADR_MEASURE_RSP",
]:
    idx = 0
    while True:
        j = IMG.find(n, idx)
        if j < 0:
            break
        s0 = j
        while s0 > 0 and IMG[s0-1] != 0 and IMG[s0-1] != 0xA:
            s0 -= 1
            if j - s0 > 80:
                s0 = j - 80
                break
        end = IMG.find(b"\x00", j)
        s = IMG[s0:end]
        if b"==>" in s or b"=>" in s or b"SR_IF" in s or b"L1LC" in s:
            print(f"  {s!r}")
        idx = j + 1

print("DONE")
