#!/usr/bin/env python3
"""Find MOVW #0x347 (Tx SIM Status) sites; dump app_state source; PinSkip eSIM gate; RatMap."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000
stream = Path("sit-stream.so").read_bytes()


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
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
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


# Shannon packed log: often MOVW r0,#id then BL log; OR MOVW with id<<8|0x44
# Search both #0x347 and #0x34744 / packed forms
print("=== MOVW imm involving 0x347 ===")
hits347 = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and (r[0] == 0x347 or r[0] == 0x34744 or (r[0] & 0xFFFF) == 0x347):
        hits347.append((o, r[0], r[1]))
    o += 2
print(f"count={len(hits347)}")
for o, imm, rd in hits347[:30]:
    print(f"  {hex(o)} MOVW r{rd},#{hex(imm)}")

# Filter to SIT/USIM-ish regions that also have LDRB BF4 nearby
print("\n=== 0x347 sites with context (SIT window prefer) ===")
for o, imm, rd in hits347:
    if 0x1C00000 <= o <= 0x2000000 or 0x14F0000 <= o <= 0x1520000 or 0x18E0000 <= o <= 0x1A00000:
        dump(o - 0x60, o + 0x50, f"log347@{hex(o)}")

# If none in those windows, dump first 5 overall with BF4/BF5/BF6 in ±0x100
print("\n=== 0x347 sites near BF4/5/6 ===")
for o, imm, rd in hits347:
    window = img[max(0, o - 0x100) : o + 0x80]
    # search LDRB.W #bf4/bf5/bf6 encodings in window
    has = False
    for p in range(0, len(window) - 4, 2):
        hw = struct.unpack_from("<H", window, p)[0]
        hw2 = struct.unpack_from("<H", window, p + 2)[0]
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) in (0xBF4, 0xBF5, 0xBF6):
            has = True
            break
    if has:
        dump(o - 0x80, o + 0x40, f"347+BF@{hex(o)}")

# Also search MOVW #0x106a (SIM STATUS update) — that function sets app_state
print("\n=== MOVW #0x106a ===")
hits106a = []
o = 0x1400000
while o < 0x1600000 - 4:
    r = movw(o)
    if r and r[0] == 0x106a:
        hits106a.append(o)
    o += 2
print([hex(h) for h in hits106a])
for h in hits106a[:3]:
    dump(h - 0x20, h + 0x100, f"106a@{hex(h)}")

# sitSendSimStatus — search MOVW of related ids near "ChangeInd"
# Find SIT TX that stores msgid 0x0200: look for STRH #0x200 into buffer
print("\n=== STRH/STR.W of 0x200 in SIT TX regions ===")
# Thumb STRH rt,[rn,#imm] = 0x8000 | ...
# Better: MOVW rX,#0x200 then STRH
sit_movw200 = []
for lo, hi in [(0x1D00000, 0x1F80000), (0x14F0000, 0x1520000), (0x1F00000, 0x1F80000)]:
    o = lo
    while o < hi - 4:
        r = movw(o)
        if r and r[0] == 0x200:
            sit_movw200.append(o)
        o += 2
print(f"MOVW #0x200 in SIT/STATUS: {len(sit_movw200)} {[hex(x) for x in sit_movw200[:20]]}")
for h in sit_movw200[:8]:
    dump(h - 0x40, h + 0x60, f"msgid200@{hex(h)}")

# PinSkip: MOVW #0x11d6 and #0x11ce — dump gates
print("\n=== PinSkip fail log sites (filter USIM) ===")
for iid in (0x11D6, 0x11CE):
    hits = []
    o = 0x1000000
    while o < 0x3000000 - 4:
        r = movw(o)
        if r and r[0] == iid:
            hits.append(o)
        o += 2
    print(f"id {hex(iid)}: {len(hits)} total")
    for h in hits:
        if 0x2B00000 <= h <= 0x2C00000 or 0x1F00000 <= h <= 0x2000000 or 0x1140000 <= h <= 0x1180000:
            dump(h - 0xA0, h + 0x30, f"pinskip_{hex(iid)}@{hex(h)}")

# SupportedRatMap: find string ptr or log near CDMA bit test
# Search "No CDMA in SupportedRatMap" via packed log — find unique nearby string ids
j = img.find(b"No CDMA in SupportedRatMap")
print(f"\nNoCDMA off={hex(j)}")
# surrounding strings
a = max(0, j - 300)
chunk = img[a : j + 100]
cur = bytearray()
strs = []
for b in chunk:
    if 32 <= b < 127:
        cur.append(b)
    else:
        if len(cur) >= 10:
            strs.append(cur.decode())
        cur = bytearray()
for s in strs:
    print(f"  '{s}'")

# Find BuildSetCdma in sit-stream — does it claim to set SupportedRatMap?
idx = stream.find(b"BuildSetCdma")
print(f"\nBuildSetCdma @{hex(idx)}")
# dump nearby strings
chunk = stream[max(0, idx - 200) : idx + 200]
cur = bytearray()
for b in chunk:
    if 32 <= b < 127:
        cur.append(b)
    else:
        if len(cur) >= 8:
            print(" ", cur.decode())
        cur = bytearray()

# Search stream for SupportedRat / SetPreferredNetwork
for n in [
    b"SupportedRatMap",
    b"SetPreferredNetworkType",
    b"preferred_network",
    b"CDMA_EVDO",
    b"RADIO_TECH_CDMA",
]:
    print(f"stream {n}: {stream.count(n)} @{hex(stream.find(n)) if stream.find(n)>=0 else None}")

# Property strings in CP for rat map
for n in [
    b"persist.vendor.radio",
    b"SupportedRatMap",
    b"rat_mode",
    b"vendor.ril.cdma",
    b"ro.vendor.ril.cdma",
]:
    c = img.count(n)
    if c:
        print(f"CP {n}: {c} @{hex(img.find(n))}")
