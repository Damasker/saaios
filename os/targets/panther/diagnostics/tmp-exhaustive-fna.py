#!/usr/bin/env python3
"""Exhaustive FN_A / WRAP_A / Present=2 callers + InitRapMap inputs."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000

FN_A = 0x14F692C
WRAP_A = 0x14C380E  # case body that BLs FN_A
WRAP_SIM = 0x14F6D02
FN_B = 0x14F9108
HELPER = 0x14F6900


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
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


def find_nearby_c_strings(site, radius=0x200):
    """Find printable ASCII near site by scanning for litpool ptrs to MAIN strings."""
    found = []
    lo = max(0, site - radius)
    hi = min(len(img) - 4, site + radius)
    # Also scan backwards for message name strings commonly placed before switch cases
    # Look for ASCII runs in a wider window before the site
    for base in range(max(0, site - 0x800), site, 1):
        if img[base : base + 4] == b"MMC_" or img[base : base + 4] == b"LTEL" or img[base : base + 5] == b"USIM ":
            # extract string
            end = base
            while end < len(img) and 32 <= img[end] < 127:
                end += 1
            s = img[base:end].decode()
            if len(s) >= 8 and s not in found:
                found.append(s)
    # Also search VA ptrs in ±0x100 that point into string pool
    for p in range(lo, hi, 4):
        v = u32(p)
        if 0x41000000 <= v <= 0x45000000:
            fo = v - VA + MAIN
            if 0 <= fo < len(img) - 8:
                if 32 <= img[fo] < 127 and img[fo] >= ord("A"):
                    end = fo
                    while end < len(img) and 32 <= img[end] < 127:
                        end += 1
                    s = img[fo:end].decode()
                    if len(s) >= 10 and ("MMC" in s or "CDMA" in s or "UMTS" in s or "LTE" in s or "SIM" in s or "LATCH" in s or "MEAS" in s):
                        if s not in found:
                            found.append(s)
    return found[:12]


# 1) ALL BL → FN_A
print("=== ALL BL → FN_A ===")
fna_callers = []
o = 0x1000000
while o < 0x3C00000 - 4:
    if bl(o) == FN_A:
        fna_callers.append(o)
    o += 2
print(f"n={len(fna_callers)} {[hex(c) for c in fna_callers]}")

# 2) ALL BL → WRAP_SIM / WRAP_A / HELPER
for name, tgt in [("WRAP_SIM", WRAP_SIM), ("WRAP_A_body", WRAP_A), ("HELPER", HELPER), ("FN_B", FN_B)]:
    cs = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        if bl(o) == tgt:
            cs.append(o)
        o += 2
    print(f"BL→{name}({hex(tgt)}): n={len(cs)} {[hex(c) for c in cs]}")

# 3) For each FN_A caller: dump context + nearby strings + arg source
for c in fna_callers:
    strs = find_nearby_c_strings(c)
    print(f"\n--- FN_A caller {hex(c)} nearby_strs={strs[:6]} ---")
    dump(c - 0x40, c + 0x20, f"FNA_call@{hex(c)}")

# 4) Present=2 stores: MOVS #2 then STRB to PresentObj
# Known: 0x14f6a14/16 and 0x14f9578/7c
# Exhaustive: in SIM module find STRB #0 with nearby MOVS #2 OR IT EQ MOVS #2
print("\n=== PresentObj STRB #0 sites in 0x14f0000-0x1508000 ===")
# Heuristic: STRB rt,[rn,#0] where rn is PresentObj - hard.
# Instead find all MOVS rX,#2 followed within 8 instr by STRB to [rY,#0]
o = 0x14F0000
p2_sites = []
while o < 0x1508000 - 4:
    hw = u16(o)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        rd = (hw >> 8) & 7
        for p in range(o + 2, min(o + 0x20, 0x1508000 - 2), 2):
            h = u16(p)
            # STRB rd,[rn,#0] encoding: 0x7000 | (imm5=0)<<6 | rn<<3 | rt
            if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 0 and (h & 7) == rd:
                p2_sites.append((o, p, rd, (h >> 3) & 7))
            # STRB.W
            if (h & 0xF800) in (0xE800, 0xF000, 0xF800):
                h2 = u16(p + 2)
                if (h & 0xFFF0) == 0xF880 and (h2 & 0xFFF) == 0 and ((h2 >> 12) & 0xF) == rd:
                    p2_sites.append((o, p, rd, h & 0xF))
    o += 2
print(f"MOVS#2→STRB#0 pairs: {len(p2_sites)}")
for m, s, rd, rn in p2_sites:
    print(f"  MOVS r{rd},#2 @{hex(m)} → STRB @{hex(s)} [r{rn},#0]")
    print(f"    strs={find_nearby_c_strings(m)[:4]}")

# Also IT EQ path in FN_A: CMP #3 then conditional MOVS #2
dump(FN_A, FN_A + 0x120, "FN_A_body")
dump(WRAP_SIM, WRAP_SIM + 0xA0, "WRAP_SIM_body")
dump(WRAP_A, WRAP_A + 0x80, "WRAP_A_body")

# 5) Dispatcher: who BLs WRAP_A and WRAP_SIM - already have callers of WRAP_SIM
# Dump switch around 0x14c380e and 0x14c3986 with MORE string context
dump(0x14C37C0, 0x14C3A00, "DISPATCH_CDMA_CASES")

# Find ALL case labels in the MMC switch that eventually reach FN_A
# by walking backwards from WRAP_A/WRAP_SIM callers for message name strings
print("\n=== Wide string scan before WRAP sites ===")
for site in [0x14C380E, 0x14C3986, 0x14C3872, 0x14F6D7A]:
    # collect MMC_/LTEL1_ strings in 0x1000 before
    strs = []
    for base in range(max(0, site - 0x1000), site):
        if img[base : base + 8] in (b"MMC_LTEL", b"LTEL1_MM", b"MMC_L1LC"):
            end = base
            while end < len(img) and 32 <= img[end] < 127:
                end += 1
            s = img[base:end].decode()
            if s not in strs:
                strs.append(s)
    print(f"{hex(site)}: {strs}")

# Also search for any OTHER BL to FN_A via indirect - already have direct BLs

# 6) InitRapMap / SupportedRatMap inputs
print("\n=== InitRapMap / SupportedRatMap strings ===")
for n in [
    b"No CDMA in InitRapMap",
    b"No CDMA in SupportedRatMap",
    b"InitRapMap",
    b"SupportedRatMap(%d)",
    b"SupportedRatMap(0x%X)",
    b"RRM_RRC_INIT_REQ_Handler - SupportedRatMap",
    b"QM_MM_INIT_REQ",
    b"QM_MM_STOP_REQ_Handler: No CDMA",
]:
    j = img.find(n)
    print(f"{n[:50]!r}: {hex(j) if j>=0 else None}")
    if j and j > 0:
        a = max(0, j - 200)
        chunk = img[a : j + 120]
        cur = bytearray()
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 10:
                    print(f"  '{cur.decode()}'")
                cur = bytearray()

# Find MOVW r2 of log ids near No CDMA - need runtime sites
# Search for unique nearby: "InitRapMap!" as part of longer - use r2 log pattern
# Find "No CDMA in SupportedRatMap" packed - search code that tests CDMA bit
# Look for TST/AND with common CDMA rat bits near QM_MM_STOP
print("\n=== Search RAP/NV/product inputs near InitRapMap ===")
for n in [
    b"RapMap",
    b"RAP_MAP",
    b"rap_map",
    b"SupportedRat",
    b"ProductCode",
    b"product_code",
    b"SalesCode",
    b"NV_RAT",
    b"nv_rat",
    b"RatCapability",
    b"CDMA_SUPPORT",
    b"SupportCdma",
    b"IsCdmaSupported",
    b"bCdma",
    b"cdma_support",
]:
    c = img.count(n)
    if c:
        print(f"  {n}: {c} @{hex(img.find(n))}")
        # neighbors
        j = img.find(n)
        chunk = img[max(0, j - 60) : j + 80]
        cur = bytearray()
        ss = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 6:
                    ss.append(cur.decode())
                cur = bytearray()
        print(f"    {ss[:6]}")
