#!/usr/bin/env python3
"""Confirm WRAP_A/LATCH case names + LTE/NR MEAS strings never reach FN_A."""
from __future__ import annotations
import struct
from pathlib import Path

raw = Path("/tmp/modem-re/b.bin").read_bytes()
FN_A, WRAP_A, LATCH, WRAP_SIM = 0x14F692C, 0x14C380E, 0x14C388E, 0x14F6D02
STATUS_WRAP = 0x14C6626


def bl_target(off: int) -> int | None:
    if off + 4 > len(raw):
        return None
    w0, w1 = struct.unpack_from("<HH", raw, off)
    if (w0 & 0xF800) != 0xF000 or (w1 & 0xD000) != 0xD000 or (w1 & 0x1000) == 0:
        return None
    s = (w0 >> 10) & 1
    imm10 = w0 & 0x3FF
    j1 = (w1 >> 13) & 1
    j2 = (w1 >> 11) & 1
    imm11 = w1 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= -1 << 25
    return ((off + 4 + imm32) & 0xFFFFFFFF) & ~1


def find_bl(tgt: int) -> list[int]:
    t = tgt & ~1
    return [o for o in range(0, len(raw) - 3, 2) if raw[o + 1] & 0xF8 == 0xF0 and bl_target(o) == t]


def movw_movt(off: int) -> int | None:
    if off + 4 > len(raw) or off < 4:
        return None
    w0, w1 = struct.unpack_from("<HH", raw, off)
    if (w0 & 0xFBF0) != 0xF2C0 or (w1 & 0x8000) != 0:
        return None
    i = (w0 >> 10) & 1
    imm_hi = ((w0 & 0xF) << 12) | (i << 11) | (((w1 >> 12) & 7) << 8) | (w1 & 0xFF)
    rd = (w1 >> 8) & 0xF
    for back in range(4, 96, 2):
        p = off - back
        a0, a1 = struct.unpack_from("<HH", raw, p)
        if (a0 & 0xFBF0) != 0xF240 or (a1 & 0x8000) != 0:
            continue
        if ((a1 >> 8) & 0xF) != rd:
            continue
        i2 = (a0 >> 10) & 1
        imm_lo = ((a0 & 0xF) << 12) | (i2 << 11) | (((a1 >> 12) & 7) << 8) | (a1 & 0xFF)
        return (imm_hi << 16) | imm_lo
    return None


def cstr(off: int) -> bytes | None:
    if off < 0 or off >= len(raw) or not (0x20 <= raw[off] < 0x7F):
        return None
    e = raw.find(b"\0", off, off + 100)
    return raw[off : e if e >= 0 else off + 100]


def name(imm: int) -> bytes | None:
    for c in (imm, imm - 0x40000000 if imm >= 0x40000000 else None):
        if c is None:
            continue
        s = cstr(c)
        if s and len(s) > 4:
            return s
    return None


print("=== Callers of WRAP_A / LATCH: case names within 0x120 ===")
for label, tgt in [("WRAP_A", WRAP_A), ("LATCH", LATCH), ("WRAP_SIM", WRAP_SIM)]:
    for o in find_bl(tgt):
        names = []
        for b in range(max(0, o - 0x120), o + 8, 2):
            imm = movw_movt(b)
            if imm is None:
                continue
            s = name(imm)
            if s and (b"MMC" in s or b"CDMA" in s or b"SADR" in s or b"MEASURE" in s or b"LATCH" in s):
                names.append((hex(b), hex(imm), s))
        print(f"{label} BL@{hex(o)} names={names}")

print("\n=== Literal xrefs to LTE_MEAS_RESULT / NR_MEAS_RESULT / CDMA strings ===")
for needle in [
    b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    b"MMCIF_L1LC_DSL1C_SADR_MEASURE_RSP",
    b"LTE_MEAS_RESULT",
    b"NR_MEAS_RESULT",
]:
    # find all occurrences; for each, find 32-bit ptr literals
    start = 0
    occ = []
    while True:
        i = raw.find(needle, start)
        if i < 0:
            break
        occ.append(i)
        start = i + 1
    print(f"\n{needle!r} occ={list(map(hex, occ[:5]))}")
    for so in occ[:3]:
        for ptr in (so, so + 0x40000000):
            pat = struct.pack("<I", ptr & 0xFFFFFFFF)
            idx = 0
            n = 0
            while n < 8:
                j = raw.find(pat, idx)
                if j < 0:
                    break
                # nearby BL targets
                near = []
                for b in range(max(0, j - 0x40), min(len(raw) - 4, j + 0x40), 2):
                    t = bl_target(b)
                    if t is None:
                        continue
                    tt = t & ~1
                    lab = {
                        WRAP_A: "WRAP_A",
                        LATCH: "LATCH",
                        WRAP_SIM: "WRAP_SIM",
                        FN_A: "FN_A",
                        STATUS_WRAP: "STATUS_WRAP",
                    }.get(tt)
                    if lab:
                        near.append((hex(b), lab))
                if near or needle.startswith(b"MMC_LTEL1_CDMA") or needle.startswith(b"MMCIF"):
                    print(f"  lit@{hex(j)} ptr={hex(ptr)} near={near}")
                idx = j + 4
                n += 1

print("\n=== lo16 case match: CDMA ec0b/efb3 vs any other FN_A feed ===")
# Prior: MOVW #0xec0b MOVT #0x4106 for CDMA MEAS; #0xefb3 for LATCH
for lo16, tag in [(0xEC0B, "CDMA_MEAS"), (0xEFB3, "CDMA_LATCH"), (0xEEE0, "UMTS_MEAS"), (0xEF38, "UMTS_TDD")]:
    # Thumb MOVW encoding is annoying; search MOVW imm low half via 0xF240 pattern with imm
    hits = []
    for o in range(0x14B7000, 0x14B9000, 2):
        imm = movw_movt(o)
        if imm and (imm & 0xFFFF) == lo16 and (imm >> 16) == 0x4106:
            hits.append(hex(o))
    print(f"  {tag} lo16={lo16:#x} movt_sites={hits}")

print("\nDONE")
