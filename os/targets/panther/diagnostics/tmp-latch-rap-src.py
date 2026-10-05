#!/usr/bin/env python3
"""Confirm LATCH case VA; DS_TCS_GV_CDMA_SUPPORT; FN_B #636c; product→RAP."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def bl(o):
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


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Find MOVW r1,#0xefb3 (LATCH_CNF low) near 0x14b7xxx / 0x14c39xx
print("=== MOVW of LATCH_CNF / MEAS lows in MMC switch ===")
targets = {
    0xEC0B: "CDMA_MEAS_RESULT_IND",
    0xEFB3: "CDMA_TIMING_LATCH_CNF",
    0xEEE0: "UMTS_MEASURE_CNF",
    0xEF38: "UMTS_TDD_PARTIAL_SEARCH_CNF",
    0xEF1B: "?",
}
for lo, name in targets.items():
    hits = []
    o = 0x14B0000
    while o < 0x14D0000 - 4:
        r = movw(o)
        if r and r[0] == lo:
            t = movt(o + 4)
            hi = None
            if t and t[1] == r[1]:
                hi = t[0]
            t2 = movt(o - 4) if o >= 4 else None
            if t2 and t2[1] == r[1]:
                hi = t2[0]
            hits.append((o, hi))
        o += 2
    print(f"  {name} #{hex(lo)}: {[(hex(a), hex(b) if b else None) for a,b in hits[:6]]}")

# Who BLs WRAP_SIM 0x14c3986 - already know. Dump case name load before it
dump(0x14B7900, 0x14B7B00, "around_LATCH_case")
# Search BL→0x14c3986 parent
print("\n=== callers of 0x14c3986 region ===")
o = 0x14B0000
while o < 0x14C4000 - 4:
    t = bl(o)
    if t == 0x14C3986 or t == 0x14C388E:
        print(f"  BL @{hex(o)} -> {hex(t)}")
        dump(o - 0x30, o + 0x10, f"latch_call@{hex(o)}")
    o += 2

# FN_B function: find #636c anywhere in 0x14f9108..0x14f9800
print("\n=== #636c in FN_B function body ===")
o = 0x14F9108
while o < 0x14F9800 - 4:
    r = movw(o)
    if r and r[0] == 0x636C:
        print(f"  @{hex(o)}")
    o += 2

# DS_TCS_GV_CDMA_SUPPORT xrefs
j = img.find(b"DS_TCS_GV_CDMA_SUPPORT")
print(f"\nDS_TCS_GV_CDMA_SUPPORT @{hex(j)}")
# neighbors
chunk = img[j : j + 200]
cur = bytearray()
for b in chunk:
    if 32 <= b < 127:
        cur.append(b)
    else:
        if len(cur) >= 6:
            print(f"  {cur.decode()}")
        cur = bytearray()

# bcProductCode - is it used for RAP?
j = img.find(b"bcProductCode")
print(f"\nbcProductCode @{hex(j)}")
chunk = img[max(0, j - 100) : j + 150]
cur = bytearray()
for b in chunk:
    if 32 <= b < 127:
        cur.append(b)
    else:
        if len(cur) >= 8:
            print(f"  {cur.decode()}")
        cur = bytearray()

# Search if InitRapMap / SupportedRatMap log shares code with ProductCode or CDMA_SUPPORT
# Find MOVW r2 for a unique id - skip; instead search for string "No CDMA in InitRapMap" 
# distance to ProductCode in code is large - conclude product is separate subsystem

# Preferred CDMA types - factory enum exists but panther EU RatMap has no CDMA
# Confirm ROADMAP: no live CDMA preferred without proof it enables latch on panther
print("\n=== Verdict inputs for No CDMA ===")
print("QM_MM_INIT_REQ_Handler logs SupportedRatMap then may log No CDMA in InitRapMap")
print("Map value arrives WITH the INIT_REQ (from RRM/lower), not from preferred SIT")
print("No SetRapMap; handover has no RAP word; preferred only selects among SupportedRatMap")
print("DS_TCS_GV_CDMA_SUPPORT / bcProductCode: separate strings, no proven link to InitRapMap store")
