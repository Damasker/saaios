#!/usr/bin/env python3
"""Find InitRapMap CDMA bit test; MmInitReq builder; TCS CDMA feature runtime."""
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
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
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
            # TST rn, #imm Thumb: 0x4200 area / W: F01x
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# 1) Find registration of "No CDMA in InitRapMap" - sequential id table
# Pattern from PinSkip: MOVW r0,#id; MOVW r1,#str_off; MOVT; BL register
# Search for unique low 16 of string offset as r1 in registration
j = img.find(b"No CDMA in InitRapMap!")
print(f"InitRapMap! @{hex(j)}")
# String often referenced as offset within a pool - try find nearby unique
# Search MOVW r2 with STATUS-style near QM_MM handler region
# QM strings around 0x44a32c4 - handler code is elsewhere; find via
# "SupportedRatMap(0x%X), RoutingInfo" log - often right before No-CDMA test

# Find ALL occurrences of MOVW that load low half of "No CDMA in InitRapMap" 
# as part of ADR - already failed. Use log id scan: find string in registration
# by looking for the relative offset stored in reg tables.

# Brute: search for BL sites that have unique sequence - CMP/TST then branch to log
# Search for function containing both SupportedRatMap(0x%X) log and No CDMA log
# by finding MOVW r2 of TWO related sequential ids

# Find registration table containing both string VAs as consecutive words
sva1 = VA + (img.find(b"@QM_MM_INIT_REQ_Handler: SupportedRatMap(0x%X)") - MAIN)
sva2 = VA + (img.find(b"No CDMA in InitRapMap!") - MAIN)
print(f"SuppRatMap fmt VA={hex(sva1)}")
print(f"NoCDMA Init VA={hex(sva2)}")
# Search for sva1 as word, then look at neighbors
n1 = struct.pack("<I", sva1)
n2 = struct.pack("<I", sva2)
ps = []
s = 0
while True:
    k = img.find(n1, s)
    if k < 0:
        break
    ps.append(k)
    s = k + 4
    if len(ps) > 20:
        break
print(f"ptrs to SuppRatMap fmt: {len(ps)} {[hex(p) for p in ps[:10]]}")
ps2 = []
s = 0
while True:
    k = img.find(n2, s)
    if k < 0:
        break
    ps2.append(k)
    s = k + 4
    if len(ps2) > 20:
        break
print(f"ptrs to NoCDMA Init: {len(ps2)} {[hex(p) for p in ps2[:10]]}")

# 2) GmcM_CommSendMmInitReq - search name as assert: MOVW of VA
j = img.find(b"GmcM_CommSendMmInitReq")
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
print(f"\nGmcM_CommSendMmInitReq MOVW: {len(hits)} {[hex(h) for h in hits[:8]]}")
for h in hits[:3]:
    dump(h - 0xA0, h + 0x40, f"MmInit@{hex(h)}")

# 3) Search for "InitRapMap" bit test: common CDMA rat bits
# In Samsung/Shannon, SupportedRatMap often uses bits:
# GSM=1, UMTS=2, LTE=4, CDMA=8 or similar; or SYS_RAT bitmask
# Search TST rX, #imm with imm in {0x8,0x10,0x20,0x40,0x80,0x100,0x200,0x400,0x800}
# near QM_MM - hard without handler location.

# Find handler by unique: search for MOVW # of RoutingInfo string nearby
j = img.find(b"SupportedRatMap(0x%X), RoutingInfo(0x%X), SrcDsdsDomain")
print(f"\nQM INIT full fmt @{hex(j)}")
# This is the primary log in QM_MM_INIT_REQ_Handler - find via packed log id
# Search registration of this long string - r1 = offset pattern

# 4) TCS feature dump path A[I][[TCS_CDMA_SUPPORT]] - find via shorter unique
# Search "TCS_CDMA_SUPPORT]]" 
j = img.find(b"TCS_CDMA_SUPPORT]]")
print(f"TCS_CDMA_SUPPORT]] @{hex(j)}")
# Find "A[I][[" pattern used for feature dump - runtime loops features
# Search code that prints TCS features - "Feature Control Index"
j = img.find(b"Feature Control Index")
print(f"Feature Control Index @{hex(j)}")
sva = VA + (j - MAIN)
lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
hits = []
o = 0x1000000
while o < 0x5200000 - 8:
    r = movw(o)
    if r and r[0] == lo:
        t = movt(o + 4)
        if t and t[0] == hi and t[1] == r[1]:
            hits.append(o)
        t2 = movt(o - 4) if o >= 4 else None
        if t2 and t2[0] == hi and t2[1] == r[1]:
            hits.append(o)
    o += 2
print(f"Feature Control Index MOVW: {len(hits)} {[hex(h) for h in hits[:6]]}")
for h in hits[:2]:
    dump(h - 0x40, h + 0x80, f"TCS_feat@{hex(h)}")

# 5) Search NV item path strings for rat / cdma support
print("\n=== NV path-like strings for RAT/CDMA ===")
for n in [
    b"/nv/item_files/modem/mmode",
    b"/nv/item_files/modem/lte",
    b"/nv/item_files/rfnv",
    b"mmode/sv",
    b"mmode/sd",
    b"mmode/cm",
    b"rat_acq_order",
    b"acq_order",
    b"sv_config",
    b"svdoe",
    b"CDMA_SUPPORT",
    b"cdma_support",
    b"support_cdma",
    b"tcs_cdma",
    b"TCS_CDMA",
    b"gv_cdma",
    b"GV_CDMA",
]:
    j = img.find(n)
    if j >= 0:
        print(f"  {n!r} @{hex(j)}")
        chunk = img[max(0, j - 30) : j + 80]
        cur = bytearray()
        ss = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 5:
                    ss.append(cur.decode())
                cur = bytearray()
        print(f"    {ss[:5]}")

# 6) Embedded default: search for "No CDMA" implying the bit test is compile-time or always
# Count whether there's a success path "CDMA in InitRapMap" or only No CDMA
for n in [
    b"CDMA in InitRapMap",
    b"CDMA in SupportedRatMap",
    b"has CDMA",
    b"HasCdma",
    b"CdmaPresent",
    b"CdmaInRap",
    b"RapHasCdma",
]:
    print(f"  {n}: {img.find(n)}")

# 7) Who sends QM_MM_INIT - message name table with builder
print("\n=== QM_MM_INIT related send/build ===")
for n in [
    b"QM_MM_INIT_REQ",
    b"GMC_QM_INIT",
    b"RRM_QM_INIT",
    b"SendQmInit",
    b"QmInitReq",
    b"FillInitRapMap",
    b"InitRapMap =",
    b"InitRapMap=",
    b"SupportedRatMap =",
    b"SupportedRatMap=",
    b"m_SupportedRatMap",
    b"g_SupportedRatMap",
]:
    j = img.find(n)
    if j >= 0:
        print(f"  {n!r} @{hex(j)}")
        chunk = img[max(0, j - 40) : j + 60]
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
