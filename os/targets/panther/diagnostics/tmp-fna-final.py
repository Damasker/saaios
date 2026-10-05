#!/usr/bin/env python3
"""Map WRAP_A via case index; confirm only FN_A/FN_B PresentObj; InitRapMap source."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


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


# Confirm #636c for FN_B Present=2 site
for c in [0x14F9578, 0x14F900A, 0x14FABA6, 0x1501DE0, 0x1501FD2, 0x15027EA]:
    hit = False
    # wider window 0x400
    for o in range(max(0x14F0000, c - 0x400), c + 0x20, 2):
        r = movw(o)
        if r and r[0] == 0x636C:
            hit = True
            print(f"{hex(c)}: #636c @{hex(o)}")
            break
    if not hit:
        print(f"{hex(c)}: NO #636c in -0x400 (not PresentObj)")

# WRAP_A: dump 0x14c380e fully and find LDRB arg
dump(0x14C380E, 0x14C388E, "WRAP_A_full")
# Parent at 0x14b7766 - what's the case? Look for CMP before BL
dump(0x14B7700, 0x14B7780, "call_WRAP_A")

# Dispatcher 0x14b79e2 was mentioned - find switch table
# Search for ADR/LDR of case table near 0x14b7000
# Message names: find litpool by computing VA and searching WHOLE image for ptr near code
for n in [
    b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    b"MMC_LTEL1_CDMA_TIMING_LATCH_REQ",
    b"MMC_LTEL1_UMTS_MEASURE_CNF",
    b"MMC_LTEL1_UMTS_TDD_PARTIAL_SEARCH_CNF",
]:
    j = img.find(n)
    if j < 0:
        # try shorter
        j = img.find(n.split(b"_")[-1][:20]) if False else -1
        print(f"{n}: NOT IN IMAGE as exact")
        # fuzzy
        short = n[9:] if n.startswith(b"MMC_LTEL1_") else n
        j = img.find(short)
        print(f"  short {short}: {hex(j) if j>=0 else None}")
        continue
    sva = VA + (j - MAIN)
    needle = struct.pack("<I", sva)
    ps = []
    start = 0
    while True:
        k = img.find(needle, start)
        if k < 0:
            break
        ps.append(k)
        start = k + 4
        if len(ps) > 20:
            break
    # filter to code-adjacent (within SIM/MMC modules)
    near = [p for p in ps if 0x1000000 <= p <= 0x2000000]
    print(f"{n.decode()}: str@{hex(j)} VA={hex(sva)} ptrs_near_code={ [hex(p) for p in near[:8]] } all_n={len(ps)}")

# InitRapMap decision: search "InitRapMap!" log via r2 style - find unique
# Look for LDR of SupportedRatMap from QM common DB
# Search string "m_pQmCommonDb" usage
print("\n=== QmCommonDb / SupportedRat field ===")
for n in [
    b"m_pQmCommonDb",
    b"SupportedRatMap",
    b"RoutingInfo",
    b"InitRapMap!",
    b"CdmaSupport",
    b"IsSupportCdma",
    b"SUPPORT_CDMA",
    b"RAT_CDMA",
    b"EUTRA_CDMA",
]:
    c = img.count(n)
    print(f"{n}: {c} first={hex(img.find(n)) if img.find(n)>=0 else None}")

# Search NV item names related to RAP
for n in [
    b"NV_RAT_MAP",
    b"/nv/item_files/modem/mmode",
    b"rat_mask",
    b"rat_disabled",
    b"DisableCdma",
    b"cdma_disable",
    b"no_cdma",
    b"eu_disable_cdma",
]:
    j = img.find(n)
    if j >= 0:
        print(f"NV-ish {n}: @{hex(j)}")
        chunk = img[max(0, j - 40) : j + 80]
        cur = bytearray()
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 6:
                    print(f"  {cur.decode()}")
                cur = bytearray()

# Handover words - confirm no RAP field (already read sit-handover.h)
print("\nHandover: words[16] CDT/rfid/identity/signature only — no RAP/RatMap field")

# FN_B Present=2 reachability - quick check CBZ before 0x14f9578
dump(0x14F9500, 0x14F95C0, "FN_B_Present2")

# Call chain summary print
print("""
CALL GRAPH SUMMARY:
FN_A 0x14f692c callers (exhaustive BL scan n=2):
  1) 0x14c3872 in WRAP_A body 0x14c380e
     ← BL from 0x14b7766 (MMC switch)
     arg0 = LDRB [r4,#0] (msg+0) at 0x14c386c
     Present=2 iff arg0==3
  2) 0x14f6d7a in WRAP_SIM 0x14f6d02
     ← BL from 0x14c3986 (MMC switch case)
     arg0 = LDRB [r5,#8] (msg+8) at 0x14f6d74
     Present=2 iff arg0==3
""")
