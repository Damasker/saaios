#!/usr/bin/env python3
"""Focused: +0xBF4 readers in SIT range; sitSendSimStatus; PinSkip eSIM gate; RatMap CDMA bit."""
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


# LDRB #0xBF4 only in SIT/USIM-ish windows (faster)
windows = [
    (0x14F0000, 0x1505000, "STATUS"),
    (0x1990000, 0x19A0000, "SET_APP"),
    (0x1D00000, 0x1F80000, "SIT"),
    (0x1050000, 0x1200000, "SIT_TX?"),
]
for lo, hi, lab in windows:
    hits = []
    o = lo
    while o < hi - 4:
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF4:
            hits.append(o)
        o += 2
    print(f"{lab} LDRB #BF4: {len(hits)} {[hex(h) for h in hits[:15]]}")
    for h in hits[:3]:
        dump(h - 0x30, h + 0x50, f"{lab}_BF4@{hex(h)}")

# sitSendSimStatusChangeInd - find via string in SIT0 region using Shannon id
# Search ASCII near "Tx SIM Status(app_state"
j = img.find(b"Tx SIM Status(app_state=%d")
print(f"\nTx SIM Status str off={hex(j)}")
# In SIT code, often log via MOVW r0,#id where id is computed - search BL sites that load app_state
# Search for STRB to offset 5 in buffer after LDRB BF4 in SIT window
print("\n=== SIT window: LDRB BF4 then STRB nearby ===")
o = 0x1D00000
while o < 0x1F80000 - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF4:
        # look ahead 0x40 for STRB imm 5 or 17
        for p in range(o, min(o + 0x60, 0x1F80000 - 4), 2):
            h = u16(p)
            if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) in (5, 17):
                print(f"  {hex(o)} LDRB BF4 -> STRB imm={(h>>6)&0x1f} @{hex(p)}")
            h2 = u16(p)
            if (h2 & 0xF800) in (0xE800, 0xF000, 0xF800):
                h3 = u16(p + 2)
                if (h2 & 0xFFF0) == 0xF880 and (h3 & 0xFFF) in (5, 17):
                    print(f"  {hex(o)} LDRB BF4 -> STRB.W imm={h3&0xfff} @{hex(p)}")
    o += 2

# DecodeSimPinSkipReq string - find function by scanning for "NOT eSIM" usage
# Search for unique substring bytes as litpool pointer
for n in [b"PIN SKIP FAILED: NOT eSIM", b"PIN SKIP FAILED: Invalid SimState", b"DecodeSimPinSkipReq"]:
    j = img.find(n)
    print(f"\n{n.decode()}: {hex(j)}")
    # print surrounding C strings (message catalog)
    a = max(0, j - 200)
    chunk = img[a : j + 80]
    # extract printable runs
    cur = bytearray()
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 8:
                print(" ", cur.decode())
            cur = bytearray()

# Search HAL for PinSkip / SupportedRat / SetPreferred that might set CDMA bit
for libname in ("libsitril.so", "sit-stream.so", "sit-base.so"):
    p = Path(libname)
    if not p.exists():
        continue
    data = p.read_bytes()
    for n in [b"PinSkip", b"PIN_SKIP", b"SupportedRat", b"SetCdma", b"CDMA", b"RatMap"]:
        c = data.count(n)
        if c:
            print(f"{libname} {n}: {c}")

# No CDMA handler: search code near QM_MM_STOP that tests a bit
# Find MOVW of low 16 of string VA 0x4449c860
sva = VA + (img.find(b"No CDMA in SupportedRatMap") - MAIN)
print(f"\nNoCDMA VA={hex(sva)}")
lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
# wider scan with ADR.W pattern - also try PC-relative from nearby
# Search for CMP/TST of CDMA bit masks near "SupportedRatMap(0x%X)" log sites
# Common CDMA bit in rat map: try find TST rX, #imm with CDMA-ish values near QM strings
j = img.find(b"@QM_MM_STOP_REQ_Handler: No CDMA")
# Look back in file for code - strings are often far from code; find via log id table
print("looking for bit test patterns in 0x44a0000 area - may be string pool only")

# Search "SupportedRatMap(%d)" usage - RRM_RRC_INIT
j = img.find(b"RRM_RRC_INIT_REQ_Handler - SupportedRatMap")
print(f"RRM INIT str {hex(j) if j>=0 else None}")

# Property / SIT that sets rat map - search sit-stream BuildSetPreferred / DualNetwork
stream = Path("sit-stream.so").read_bytes()
for n in [
    b"BuildSetPreferredNetworkType",
    b"BuildSetDualNetwork",
    b"BuildSetCdma",
    b"BuildSetDSNetworkType",
    b"SupportedRat",
]:
    print(f"stream {n}: {stream.find(n)}")
