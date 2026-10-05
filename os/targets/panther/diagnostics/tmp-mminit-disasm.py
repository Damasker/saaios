#!/usr/bin/env python3
"""Disasm GmcM_CommSendMmInitReq; find SupportedRatMap field fill; CDMA bit test id 0x7f4 wrong — find real."""
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
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF880:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x6000:
                extra = f" ;STR r{hw&7},[r{(hw>>3)&7},#{((hw>>6)&0x1f)*4}]"
            if (hw & 0xF800) == 0x6800:
                extra = f" ;LDR r{hw&7},[r{(hw>>3)&7},#{((hw>>6)&0x1f)*4}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Function starts right after name at 0x17bfa28
dump(0x17BFA28, 0x17BFC80, "GmcM_CommSendMmInitReq")

# Callers of MmInitReq
print("\n=== callers of GmcM_CommSendMmInitReq ===")
callers = []
o = 0x1000000
while o < 0x3C00000 - 4:
    if bl(o) == 0x17BFA28:
        callers.append(o)
    o += 2
print(f"n={len(callers)} {[hex(c) for c in callers[:20]]}")
for c in callers[:5]:
    dump(c - 0x40, c + 0x20, f"caller@{hex(c)}")

# Find full string start for No CDMA InitRapMap
j = img.find(b"@QM_MM_INIT_REQ_Handler: No CDMA in InitRapMap!")
print(f"\nfull NoCDMA str @{hex(j)}")
sva = VA + (j - MAIN)
print(f"VA={hex(sva)}")
# Find registration with correct pool: MOVT that yields this VA
# Pool base candidates: try matching vlo of full string
vlo, vhi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
hits = []
o = 0x1000000
while o < 0x3C00000 - 8:
    r = movw(o)
    if r and r[0] == vlo:
        t = movt(o + 4)
        if t and t[0] == vhi and t[1] == r[1]:
            hits.append(o)
        t2 = movt(o - 4) if o >= 4 else None
        if t2 and t2[0] == vhi and t2[1] == r[1]:
            hits.append(o)
    o += 2
print(f"full NoCDMA VA MOVW: {len(hits)}")

# Try offset from 0x44a0000 pool: j - 0x44a0000 = 0x3360 for "No CDMA..." alone
# Full string starts earlier - find @QM_MM
jfull = img.find(b"@QM_MM_INIT_REQ_Handler: No CDMA in InitRapMap!")
off = jfull - 0x44A0000
print(f"offset from 0x44a0000: {hex(off) if off>=0 else 'neg'}")
# Registration with MOVT #0x44a?
hits = []
o = 0x1000000
while o < 0x2000000 - 4:
    r = movw(o)
    if r and r[1] == 1 and r[0] == (jfull & 0xFFFF):
        t = movt(o + 4)
        if t and t[1] == 1:
            hits.append((o, t[0], r[0]))
        t2 = movt(o - 4) if o >= 4 else None
        if t2 and t2[1] == 1:
            hits.append((o, t2[0], r[0]))
    o += 2
print(f"MOVW r1,#{hex(jfull & 0xFFFF)} with MOVT: {[(hex(a), hex(b)) for a,b,_ in hits[:8]]}")

# Search MOVW r2,#0x7f4 as potential wrong id - also search for ids near
# Find runtime MOVW r2,#id where we first discover correct id via
# looking at C++ log macro encoding

# Search for TST/AND against CDMA rat bit values in GMC/QM regions
# Common Shannon SYS_RAT: CDMA=0x4 or 0x8; bitmask in SupportedRatMap
print("\n=== Search AND/TST #CDMA-ish in 0x17b0000-0x17e0000 (GMC) ===")
# Thumb ANDS rn,rm; ANDW rn,rn,#imm
# ANDW T1 encoding F01x 0xxx for AND rn,rn,#imm
o = 0x17B0000
and_hits = []
while o < 0x1800000 - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF010:  # ANDW
        # imm12 encoding complex - just note
        and_hits.append(o)
    o += 2
print(f"ANDW count in GMC: {len(and_hits)}")

# Look at TCS_Get / TCS_Init - names at high offsets, function may follow
for name, j in [(b"TCS_Get", img.find(b"TCS_Get")), (b"TCS_Init", img.find(b"TCS_Init"))]:
    print(f"\n{name} @{hex(j)}")
    # dump following bytes as code if Thumb
    dump(j + len(name) + 1, j + len(name) + 0x80, f"{name.decode()}_after")

# Search "TCS_CDMA_SUPPORT" value source - feature table with default bytes
# Look for binary table of GV defaults near DS_TCS_GV registration cluster 0x1763582
dump(0x1763500, 0x1763700, "TCS_GV_reg_cluster")

# Find who READS CDMA support after registration - search BL to feature get
# with arg pointing to CDMA - hard.

# Key: does RRM_RRC_INIT set SupportedRatMap from compile-time band list?
print("\n=== RRM / band / CDMA exclusion strings ===")
for n in [
    b"ExcludeCdma",
    b"exclude_cdma",
    b"RemoveRat",
    b"ClearRatBit",
    b"RatMapClear",
    b"MaskOutCdma",
    b"NonCdma",
    b"nonCdma",
    b"CDMA_NOT_SUPPORTED",
    b"CdmaNotSupported",
    b"NOT_SUPPORT_CDMA",
    b"SupportRatCdma",
    b"IsSupportedRat",
    b"CheckSupportedRat",
    b"GetSupportedRatMap",
    b"SetSupportedRatMap",
    b"UpdateRatMap",
    b"BuildRatMap",
    b"MakeRatMap",
    b"ComposeRatMap",
    b"CalcRatMap",
    b"ComputeRatMap",
]:
    j = img.find(n)
    if j >= 0:
        print(f"  {n}: @{hex(j)}")
        chunk = img[max(0, j - 40) : j + 80]
        cur = bytearray()
        ss = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 6:
                    ss.append(cur.decode())
                cur = bytearray()
        print(f"    {ss[:5]}")
