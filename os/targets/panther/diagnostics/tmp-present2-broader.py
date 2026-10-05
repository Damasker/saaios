#!/usr/bin/env python3
"""Broader Present=2 search: getobj writers, memcpy into PresentObj, FN_A callers/args, MEAS producers."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000
GETOBJ = 0x20EA040
FN_A = 0x14F692C
MEMCPY_CANDIDATES = []  # filled if we find known memcpy


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def bl(o):
    if o < 0 or o + 4 > len(img):
        return None
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


def dump(start, end, lab, max_lines=100):
    print(f"\n=== {lab} ===")
    o = start
    n = 0
    while o < end and n < max_lines:
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
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                extra = f" ;STR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF840:  # STR.W reg
                extra = f" ;STR.W form"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xFF87) == 0x4780:
                extra = f" ;BLX r{(hw >> 3) & 0xF}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2
        n += 1


# --- 1) All getobj(#0x636c) sites: scan for ANY [obj,#0] store of 2 ---
print("=== getobj#636c sites + nearby STRB #0..3 ===")
sites = []
o = MAIN
while o < len(img) - 8:
    r = movw(o)
    if r and r[0] == 0x636C:
        for d in range(0, 40, 2):
            if bl(o + d) == GETOBJ:
                sites.append(o)
                break
    o += 2
print(f"sites={len(sites)} {[hex(s) for s in sites]}")

for s in sites:
    # scan function-ish window +0x200 for STRB to [rN,#0] with imm 0/1/2/3
    window_end = min(s + 0x280, len(img) - 4)
    writes = []
    o = s
    while o < window_end:
        hw = u16(o)
        # MOVS #imm then STRB.W [rN,#0]
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) <= 3:
            imm = hw & 0xFF
            for p in range(o + 2, min(o + 16, window_end), 2):
                h2 = u16(p)
                if (h2 & 0xF800) in (0xE800, 0xF000, 0xF800):
                    h3 = u16(p + 2)
                    if (h2 & 0xFFF0) == 0xF880 and (h3 & 0xFFF) == 0:
                        writes.append((o, p, imm, f"STRB.W [r{h2 & 0xf},#0]={imm}"))
                    break
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0:
                    writes.append((o, p, imm, f"STRB t16 [r{(h2 >> 3) & 7},#0]={imm}"))
                    break
        o += 2
    print(f"  {hex(s)} writes-to-#0: {writes[:12]}")

# --- 2) FN_A full prologue: what arg0 values accepted; callers already known ---
dump(FN_A, FN_A + 0x120, "FN_A body start")

# --- 3) Find MEAS_RESULT / TIMING_LATCH / msg ids that call FN_A path ---
print("\n=== strings near CDMA/MEAS/LATCH ===")
for n in [
    b"CDMA_MEAS_RESULT_IND",
    b"CDMA_TIMING_LATCH",
    b"MEAS_RESULT_IND",
    b"UMTS_MEASURE",
    b"LTE_MEAS",
    b"MMC_LTEL1_",
]:
    idxs = []
    pos = 0
    while len(idxs) < 8:
        j = img.find(n, pos)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        idxs.append(img[a:b].decode("ascii", "replace")[:100])
        pos = j + 1
    for s in idxs:
        print(f"  {n.decode(errors='replace')}: {s}")

# --- 4) Who calls WRAP that leads to FN_A (0x14c3872 region = SIMWRAP?) ---
# Find function containing 0x14c3872 - scan back for push
print("\n=== callers of wrapper containing FN_A BL @0x14c3872 ===")
# Find entry of that function
entry = 0x14C3812
for back in range(0, 0x80, 2):
    o = 0x14C3872 - back
    hw = u16(o)
    if hw in (0xB570, 0xB5F0, 0xB5B0, 0xB510) or hw == 0xE92D:
        entry = o
        break
    if (hw & 0xFF00) == 0xB500:
        entry = o
        break
print(f"approx entry {hex(entry)}")
# BL to entry
callers = []
o = MAIN
while o < len(img) - 4:
    if bl(o) == entry or bl(o) == (entry & ~1):
        callers.append(o)
    o += 2
print(f"direct BL to {hex(entry)}: {len(callers)} {[hex(c) for c in callers[:20]]}")

# Also BL to 0x14c388e (next fn after return) and to SIMWRAP 0x14c3986
for tgt in (0x14C388E, 0x14C3986, 0x14F6D02, 0x14F692C):
    cs = []
    o = MAIN
    while o < len(img) - 4:
        if bl(o) == tgt:
            cs.append(o)
        o += 2
    print(f"BL->{hex(tgt)}: {len(cs)} {[hex(c) for c in cs[:15]]}")

# --- 5) memcpy/memmove: find by string, then scan BLs near PresentObj uses ---
print("\n=== memcpy/memmove symbols ===")
for n in [b"memcpy", b"memmove", b"__aeabi_memcpy", b"__aeabi_memmove"]:
    j = img.find(n + b"\x00")
    if j < 0:
        j = img.find(n)
    print(f"  {n}: {hex(j) if j >= 0 else None}")

# Search for size==1 or size==4 copies where dest might be PresentObj
# Heuristic: after getobj return, BL memcpy with r2=#1 or #4
print("\n=== after getobj#636c: BL with MOVS r2,#1/#4 (possible memcpy) ===")
for s in sites:
    for o in range(s, min(s + 0x200, len(img) - 4), 2):
        hw = u16(o)
        # MOVS r2,#1 = 0x2201; MOVS r2,#4 = 0x2204
        if hw in (0x2201, 0x2204, 0x2101, 0x2104):
            for p in range(o, min(o + 12, len(img) - 4), 2):
                b = bl(p)
                if b and abs(b - o) > 0x100:
                    print(f"  near {hex(s)}: {hex(o)} imm={hex(hw)} BL->{hex(b)}")

# --- 6) STRB.W [rN, rM] computed offset where base is PresentObj ---
# Look for LDRB/STRB with register offset near getobj result in FN_A and STATUS
print("\n=== STRB.W register-offset forms near FN_A / STATUS / getobj sites ===")
for lo, hi, lab in [
    (0x14F6900, 0x14F6B00, "FN_A"),
    (0x14FB300, 0x14FB700, "STATUS"),
    (0x14C3800, 0x14C3A00, "WRAP"),
]:
    hits = []
    o = lo
    while o < hi - 4:
        hw, hw2 = u16(o), u16(o + 2)
        # STRB.W <Rt>,[<Rn>,<Rm>]  encoding T2: F80C 0000-ish
        if (hw & 0xFFF0) == 0xF800 and (hw2 & 0x0FC0) == 0x0000:
            hits.append(hex(o))
        # STRB.W [Rn, #imm12] already known
        o += 2
    print(f"  {lab} reg-offset STRB.W: {hits[:20]}")

# --- 7) NS message dispatch: who loads arg0==3 into FN_A ---
# Disasm SIMWRAP latch path for CMP #3
dump(0x14C3986, 0x14C3A80, "SIMWRAP_LATCH")
dump(0x14F6CC0, 0x14F6D90, "MEAS path to FN_A")
