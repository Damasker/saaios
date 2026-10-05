#!/usr/bin/env python3
"""Find PinSkip runtime log (r2=#0x11d6 pattern); dump TX pack STRBs; RatMap writers."""
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


# STATUS-style: MOVW r2,#0x11d6 / r2,#0x11ce / r2,#0x347
for iid in (0x11D6, 0x11CE, 0x347, 0x11D0, 0x11D1):
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        r = movw(o)
        if r and r[0] == iid and r[1] == 2:  # r2 = STATUS log style
            hits.append(o)
        o += 2
    print(f"MOVW r2,#{hex(iid)}: {len(hits)} {[hex(h) for h in hits[:12]]}")
    for h in hits[:4]:
        dump(h - 0x60, h + 0x40, f"r2_{hex(iid)}@{hex(h)}")

# Also MOVW r1,#0x347 with nearby LDRB/GET pattern (not reg table)
print("\n=== MOVW r1/#0x347 with GET_APP or BF4 nearby ===")
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == 0x347 and r[1] in (0, 1):
        # check ±0x100 for BL to GET_APP or LDRB BF4
        found = False
        for p in range(max(0, o - 0x100), min(len(img) - 4, o + 0x100), 2):
            if bl(p) == 0x18EC8C0:
                found = True
            hw, hw2 = u16(p), u16(p + 2)
            if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF4:
                found = True
        if found:
            dump(o - 0x80, o + 0x40, f"347_near_app@{hex(o)}")
    o += 2

# Dump 0x18e8348 continuation for STRB of app_state into buffer
dump(0x18E8348, 0x18E8500, "TX_after_get_app")

# In 0x18c64b8: r5=app_state; follow where r5 is stored
dump(0x18C64B8, 0x18C6600, "pack_r5_appstate")

# PinSkip: search USIM for "PIN SKIP" success path too
for n in [
    b"PIN SKIP SUCCESS",
    b"PIN SKIP REQ",
    b"DecodeSimPinSkipReq",
    b"HandleSimPinSkip",
    b"SimPinSkip",
]:
    j = img.find(n)
    print(f"{n}: {hex(j) if j>=0 else None}")

# Search for eSIM gate: LDRB then CMP #0 near string cluster usage
# Find unique "Sending FAILED" after PIN SKIP
j = img.find(b"PIN SKIP FAILED: NOT eSIM")
# Look at USIM code region 0x2bxxxxx that had FCP - search for is_esim field
# Search MOVW of log ids 0x11d6 with r2 in USIM window only
print("\n=== USIM window 0x2b00000-0x2c00000 MOVW #0x11d6/#0x11ce ===")
for iid in (0x11D6, 0x11CE):
    o = 0x2B00000
    while o < 0x2C00000 - 4:
        r = movw(o)
        if r and r[0] == iid:
            print(f"  {hex(o)} r{r[1]}")
            dump(o - 0xA0, o + 0x20, f"usim_{hex(iid)}")
        o += 2

# Broader: 0x28-0x30M
print("\n=== 0x2800000-0x3000000 MOVW #0x11d6 ===")
o = 0x2800000
while o < 0x3000000 - 4:
    r = movw(o)
    if r and r[0] == 0x11D6:
        print(f"  {hex(o)} r{r[1]}")
    o += 2

# RatMap: find InitRapMap function via log id style MOVW r2 of a unique id
# Search for stores of SupportedRatMap - look for string "InitRapMap" registration
# Check if preferred network SIT is documented as changing SupportedRatMap - answer NO from strings
print("\n=== RatMap related function names ===")
for n in [
    b"InitRapMap",
    b"InitRatMap",
    b"SetRapMap",
    b"UpdateRapMap",
    b"SupportedRatMap",
    b"CheckCdmaInSupportedRat",
]:
    j = img.find(n)
    print(f"  {n}: {hex(j) if j>=0 else None}")
    if j and j > 0:
        # name may be in assert - print neighbors
        chunk = img[max(0, j - 80) : j + 80]
        cur = bytearray()
        strs = []
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 6:
                    strs.append(cur.decode())
                cur = bytearray()
        print("   ", strs[:8])
