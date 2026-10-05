#!/usr/bin/env python3
"""Verify extra Present=2 candidates; map WRAP_A parent; InitRapMap bit test."""
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
            if (hw & 0xFFF0) == 0xF880:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
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


def has_636c(start, end):
    """PresentObj id #0x636c nearby?"""
    o = start
    while o < end - 4:
        r = movw(o)
        if r and r[0] == 0x636C:
            return True
        o += 2
    return False


# Check each candidate site for PresentObj getobj #0x636c in containing function
cands = [0x14F6A14, 0x14F900A, 0x14F9578, 0x14FABA6, 0x1501DE0, 0x1501FD2, 0x15027EA]
for c in cands:
    # scan back up to 0x200 for PUSH / #0x636c
    window_lo = max(0x14F0000, c - 0x200)
    hit = has_636c(window_lo, c + 0x40)
    print(f"cand {hex(c)} has_#636c_in_-0x200={hit}")
    dump(c - 0x60, c + 0x30, f"cand@{hex(c)}")

# WRAP_A parent 0x14b7766 - dump and find message
dump(0x14B7700, 0x14B7800, "WRAP_A_parent_call")
# Who calls into the big switch containing WRAP_A?
# Find function containing 0x14c380e - look for prologue before WRAP_A
# And find BL→0x14c380e already = 0x14b7766
# Dump more context around 0x14b7766 for message id / case number
dump(0x14B7600, 0x14B7900, "parent_wide")

# String table near MMC switch - search file offsets of known message names
for n in [
    b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    b"MMC_LTEL1_UMTS_MEASURE_CNF",
    b"MMC_LTEL1_UMTS_TDD_PARTIAL_SEARCH_CNF",
    b"CDMA_MEAS_RESULT",
    b"CDMA_TIMING_LATCH",
]:
    positions = []
    s = 0
    while True:
        j = img.find(n, s)
        if j < 0:
            break
        positions.append(j)
        s = j + 1
        if len(positions) > 5:
            break
    print(f"{n}: {[hex(p) for p in positions]}")

# Map VA of those strings → litpool entries near switch 0x14c0000-0x14d0000
print("\n=== litpool ptrs to CDMA/UMTS msg names in switch region ===")
for n in [
    b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    b"MMC_LTEL1_UMTS_MEASURE_CNF",
    b"MMC_LTEL1_UMTS_TDD_PARTIAL_SEARCH_CNF",
    b"MMC_LTEL1_GSM_MEASURE_CNF",
    b"MMC_LTEL1_NR_MEASURE",
]:
    j = img.find(n)
    if j < 0:
        print(f"{n}: not found")
        continue
    sva = VA + (j - MAIN)
    needle = struct.pack("<I", sva)
    # search litpool in 0x14b0000-0x14e0000
    ps = []
    s = 0x14B0000
    while s < 0x14E0000 - 4:
        if img[s : s + 4] == needle:
            ps.append(s)
        s += 4
    print(f"{n.decode()}: VA={hex(sva)} litpool={[hex(p) for p in ps]}")

# InitRapMap: find runtime code via MOVW r2 of related - search "No CDMA in InitRapMap"
# Use Shannon: find unique substring offset used as fmt - try packed log
# Broader: find AND/TST of SupportedRatMap global
# Search for stores TO a word that is later logged as SupportedRatMap
# Look for RRM_RRC_INIT - MOVW r2 style
print("\n=== MOVW r2 near SupportedRat / InitRap log ids ===")
# First find if there's a unique short id - search registration of the string
j = img.find(b"No CDMA in InitRapMap")
print(f"InitRapMap str @{hex(j)}")
j2 = img.find(b"No CDMA in SupportedRatMap")
print(f"SupportedRatMap NoCDMA @{hex(j2)}")

# Search code for LDR of a bitmask then BNE to No-CDMA path
# Find BL sites that load "No CDMA in SupportedRatMap" via log helper with string VA
# Try MOVW+MOVT of string VA low/high
for label, j in [("InitRap", img.find(b"No CDMA in InitRapMap")), ("SuppRat", img.find(b"No CDMA in SupportedRatMap"))]:
    sva = VA + (j - MAIN)
    lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 8:
        r = movw(o)
        if r and r[0] == lo:
            t = movt(o + 4)
            if t and t[0] == hi and t[1] == r[1]:
                hits.append(o)
            t2 = movt(o - 4) if o >= 4 else None
            if t2 and t2[0] == hi and t2[1] == r[1]:
                hits.append(o)
        o += 2
    print(f"{label} MOVW/MOVT VA hits: {len(hits)} {[hex(h) for h in hits[:8]]}")
    for h in hits[:2]:
        dump(h - 0x80, h + 0x40, f"{label}@{hex(h)}")

# Product / NV near QM_MM_INIT
print("\n=== QM_MM_INIT strings ===")
for n in [
    b"QM_MM_INIT_REQ_Handler",
    b"SupportedRatMap",
    b"InitRapMap",
    b"RapInfo",
    b"gSupportedRat",
    b"sSupportedRat",
]:
    j = img.find(n)
    print(f"{n}: {hex(j) if j>=0 else None}")

# Factory preferred CDMA enum in sit-stream
stream = Path("sit-stream.so").read_bytes()
for n in [
    b"CDMA",
    b"LTE_CDMA",
    b"GLOBAL",
    b"PREF_NET",
    b"SIT_NET_PREF",
    b"CdmaEvdo",
    b"LTE_CMDA_EVDO",  # typo variant
    b"LTE_CDMA_EVDO",
]:
    c = stream.count(n)
    if c:
        print(f"stream {n}: {c}")
        # print nearby unique strings
        idx = 0
        shown = 0
        while shown < 5:
            j = stream.find(n, idx)
            if j < 0:
                break
            chunk = stream[max(0, j - 40) : j + 60]
            cur = bytearray()
            for b in chunk:
                if 32 <= b < 127:
                    cur.append(b)
                else:
                    if len(cur) >= 8 and b"CDMA" in cur or b"PREF" in cur or b"NET" in cur:
                        print(f"  ...{cur.decode()}...")
                        shown += 1
                    cur = bytearray()
            idx = j + 1
