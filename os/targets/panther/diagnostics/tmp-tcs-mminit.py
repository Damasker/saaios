#!/usr/bin/env python3
"""Trace TCS_CDMA_SUPPORT, GmcM_CommSendMmInitReq, RRM SupportedRatMap source."""
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
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
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
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# TCS_CDMA_SUPPORT log string
for n in [
    b"TCS_CDMA_SUPPORT",
    b"A[I][[TCS_CDMA_SUPPORT]]",
    b"DS_TCS_GV_CDMA_SUPPORT",
    b"GmcM_CommSendMmInitReq",
    b"CommSendMmInitReq",
    b"SendMmInitReq",
    b"SupportedRatMap(0x%X)",
    b"SupportedRatMap(%d)",
    b"No CDMA in InitRapMap!",
]:
    j = img.find(n)
    print(f"{n!r}: {hex(j) if j>=0 else None}")
    if j and j > 0:
        sva = VA + (j - MAIN)
        print(f"  VA={hex(sva)}")

# Find MOVW/MOVT of TCS_CDMA_SUPPORT string VA
j = img.find(b"A[I][[TCS_CDMA_SUPPORT]]")
if j < 0:
    j = img.find(b"TCS_CDMA_SUPPORT")
sva = VA + (j - MAIN)
lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
print(f"\nTCS str VA={hex(sva)} lo={hex(lo)} hi={hex(hi)}")
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
print(f"TCS MOVW/MOVT hits: {len(hits)} {[hex(h) for h in hits[:10]]}")
for h in hits[:3]:
    dump(h - 0x60, h + 0x40, f"TCS@{hex(h)}")

# Also try packed log - search for unique nearby "TCS_OPT_CARRIER"
j = img.find(b"TCS_OPT_CARRIER_TYPE")
print(f"\nTCS_OPT_CARRIER_TYPE @{hex(j) if j>=0 else None}")
chunk = img[max(0, j - 200) : j + 200] if j and j > 0 else b""
cur = bytearray()
for b in chunk:
    if 32 <= b < 127:
        cur.append(b)
    else:
        if len(cur) >= 8:
            print(f"  '{cur.decode()}'")
        cur = bytearray()

# GmcM_CommSendMmInitReq - find as C string, then find code referencing it
j = img.find(b"GmcM_CommSendMmInitReq")
print(f"\nGmcM_CommSendMmInitReq @{hex(j)}")
sva = VA + (j - MAIN)
# Often assert/log with function name - search MOVW of VA or find nearby code via name table
# Search for "MmInitReq" more broadly
for n in [
    b"GmcM_CommSendMmInitReq",
    b"GmcM_SendMmInit",
    b"Send QM_MM_INIT",
    b"QM_MM_INIT_REQ send",
    b"Sending QM_MM_INIT",
    b"SendMmInitReq",
    b"MmInitReq SupportedRat",
]:
    print(f"  {n}: {img.find(n)}")

# Find No CDMA InitRapMap runtime via MOVW r2 style - first find if there's a log id
# Search for TST/AND of CDMA-ish bits near QM_MM - use string registration offset
# Approach: find "No CDMA in InitRapMap!" via packed - search code that does
# LDR of SupportedRatMap then branch to No-CDMA log
# Look for MOVW r2 of sequential ids near InitRapMap registration

# Search DS_TCS_GV_CDMA_SUPPORT as NV/feature key - often used with feature getter
j = img.find(b"DS_TCS_GV_CDMA_SUPPORT")
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
print(f"\nDS_TCS_GV_CDMA_SUPPORT MOVW hits: {len(hits)} {[hex(h) for h in hits[:8]]}")
for h in hits[:4]:
    dump(h - 0x40, h + 0x50, f"DS_TCS@{hex(h)}")

# CDMA_CAL strings
print("\n=== CDMA_CAL neighbors ===")
s = 0
shown = 0
while shown < 8:
    j = img.find(b"CDMA_CAL", s)
    if j < 0:
        break
    chunk = img[max(0, j - 40) : j + 60]
    cur = bytearray()
    ss = []
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 5:
                ss.append(cur.decode())
            cur = bytearray()
    print(f"  @{hex(j)}: {ss}")
    s = j + 1
    shown += 1

# RRM SupportedRatMap(%d) - find who passes the value
# Search for string "SupportedRatMap(%d)" VA loads
j = img.find(b"RRM_RRC_INIT_REQ_Handler - SupportedRatMap(%d)")
print(f"\nRRM INIT SupportedRatMap str @{hex(j)}")
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
print(f"RRM INIT fmt MOVW hits: {len(hits)} {[hex(h) for h in hits[:6]]}")
for h in hits[:2]:
    dump(h - 0x80, h + 0x40, f"RRM_INIT@{hex(h)}")
