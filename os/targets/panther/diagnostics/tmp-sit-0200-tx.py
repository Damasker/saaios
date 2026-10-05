#!/usr/bin/env python3
"""Trace SIT 0x0200 / SIT_GET_SIM_STATUS TX: where app_state byte is filled."""
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


def dump(start, end, lab, max_lines=120):
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
        n += 1


def find_movw_movt(sva, limit=40):
    lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
    hits = []
    o = MAIN
    while o < len(img) - 8 and len(hits) < limit:
        r = movw(o)
        if r and r[0] == lo:
            for d in range(0, 24, 2):
                t = movt(o + d)
                if t and t[1] == r[1] and t[0] == hi:
                    hits.append(o)
                    break
        o += 2
    return hits


# Strings
for n in [
    b"Tx SIM Status(app_state=%d",
    b"SIT_GET_SIM_STATUS",
    b"sitSendSimStatus",
    b"DecodeSimPinSkipReq",
    b"PIN SKIP FAILED: NOT eSIM",
    b"PIN SKIP FAILED: Invalid SimState",
    b"QM_MM_STOP_REQ_Handler: No CDMA",
    b"QM_MM_INIT_REQ_Handler: SupportedRatMap",
]:
    j = img.find(n)
    print(f"{n!r}: {hex(j) if j>=0 else None} VA={hex(VA+(j-MAIN)) if j and j>=MAIN else None}")

# Xref Tx SIM Status log id 0x347 — prior docs said id 0x347
# Search MOVW #0x347 with MOVT to log region
print("\n=== MOVW #0x347 (Tx SIM Status) ===")
hits347 = []
o = MAIN
while o < len(img) - 8:
    r = movw(o)
    if r and r[0] == 0x347:
        hits347.append(o)
    o += 2
print(f"count={len(hits347)} {[hex(h) for h in hits347[:20]]}")
for h in hits347[:4]:
    dump(h - 0x40, h + 0x80, f"TxSimStatus@{hex(h)}")

# SIT_GET_SIM_STATUS string xrefs
j = img.find(b"SIT_GET_SIM_STATUS")
if j >= MAIN:
    sva = VA + (j - MAIN)
    print(f"\nSIT_GET_SIM_STATUS VA={hex(sva)}")
    hits = find_movw_movt(sva, 20)
    print(f"MOVW+MOVT hits: {len(hits)} {[hex(h) for h in hits]}")
    for h in hits[:3]:
        dump(h - 0x20, h + 0x60, f"GetSimStatus@{hex(h)}")

# LDRB +0xBF4 readers — SIT TX should read app_state mirror
print("\n=== ALL LDRB.W [*,#0xBF4] ===")
bf4r = []
o = MAIN
while o < len(img) - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF4:
        bf4r.append(o)
    o += 2
print(f"count={len(bf4r)} {[hex(x) for x in bf4r[:30]]}")
for addr in bf4r[:8]:
    dump(addr - 0x20, addr + 0x40, f"LDRB_BF4@{hex(addr)}", max_lines=50)

# PIN_SKIP DecodeSimPinSkipReq
j = img.find(b"DecodeSimPinSkipReq")
print(f"\nDecodeSimPinSkipReq: {hex(j) if j>=0 else None}")
j = img.find(b"PIN SKIP FAILED: NOT eSIM")
if j and j > 0:
    sva = VA + (j - MAIN) if j >= MAIN else None
    print(f"NOT eSIM str VA={hex(sva) if sva else None}")
    # find nearby code via MOVW of low half of a related log id 0x11d6
print("\n=== MOVW #0x11d6 / #0x11ce (PIN SKIP fail) ===")
for imm in (0x11D6, 0x11CE):
    hs = []
    o = MAIN
    while o < len(img) - 4:
        r = movw(o)
        if r and r[0] == imm:
            hs.append(o)
        o += 2
    print(hex(imm), [hex(h) for h in hs[:10]])
    for h in hs[:2]:
        dump(h - 0x60, h + 0x40, f"PinSkipFail@{hex(h)}")

# SupportedRatMap — QM_MM_INIT: is map from message payload?
j = img.find(b"QM_MM_INIT_REQ_Handler: SupportedRatMap")
print(f"\nQM_MM_INIT str {hex(j) if j>=0 else None}")
# Find code using format string - often ADR to str then BL log
# Search for No CDMA branch: MOVW to that string
j = img.find(b"No CDMA in SupportedRatMap")
if j >= MAIN:
    sva = VA + (j - MAIN)
    hits = find_movw_movt(sva, 10)
    print(f"No CDMA xrefs: {len(hits)} {[hex(h) for h in hits]}")
    for h in hits[:3]:
        dump(h - 0x80, h + 0x40, f"NoCDMA@{hex(h)}")
