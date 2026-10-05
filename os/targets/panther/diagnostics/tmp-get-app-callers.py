#!/usr/bin/env python3
"""Trace get_app_state 0x18ec8c0 callers → SIT 0x0200 TX builder; PinSkip real gate; RatMap mutability."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000
stream = Path("sit-stream.so").read_bytes()
ril = Path("libsitril.so").read_bytes()


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


GET_APP = 0x18EC8C0  # returns LDRB +0xBF4
SET_APP = 0x19916D2

# Full dump of getter
dump(GET_APP, GET_APP + 0x40, "GET_APP_STATE")

# Find all BL → GET_APP
print("\n=== callers of GET_APP_STATE ===")
callers = []
o = 0x1000000
while o < 0x3C00000 - 4:
    t = bl(o)
    if t == GET_APP:
        callers.append(o)
    o += 2
print(f"n={len(callers)}")
for c in callers:
    print(f"  BL @{hex(c)}")
    dump(c - 0x30, c + 0x50, f"caller_get@{hex(c)}")

# Also BL → SET_APP (already known) — after SET_APP does it notify SIT?
print("\n=== SET_APP epilogue / notify ===")
dump(SET_APP, SET_APP + 0xC0, "SET_APP_body")

# Find sitSend after SET_APP: look at BL targets from SET_APP region
print("\n=== BLs from SET_APP function ===")
o = SET_APP
seen = set()
while o < SET_APP + 0x100:
    t = bl(o)
    if t is not None:
        seen.add((o, t))
    hw = u16(o)
    o += 4 if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) else 2
for o, t in sorted(seen):
    print(f"  {hex(o)} -> {hex(t)}")

# After SET_APP stores BF4, there's BL 0x20e184e with r0=#2 — pin? 
# And earlier analysis: 0x199178e MOVS r0,#2; BL 0x20e184e with r3=old BF4
# Need function that builds SIT response: search STRB after BL GET_APP

print("\n=== After GET_APP: STRB imm patterns ===")
for c in callers:
    for p in range(c, min(c + 0x80, len(img) - 4), 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFFF0) == 0xF880:
            print(f"  after {hex(c)}: STRB.W @{hex(p)} [r{hw&0xf},#{hex(hw2&0xfff)}]")
        if (hw & 0xF800) == 0x7000:
            print(f"  after {hex(c)}: STRB @{hex(p)} imm={(hw>>6)&0x1f}")

# PinSkip real call sites: search for MOV packed id or BL that uses 0x11d6 as arg differently
# Pattern: MOVW r0, #0x11d644 (id<<8|0x44) common Shannon
print("\n=== packed log ids 0x11d6 / 0x11ce ===")
for iid in (0x11D6, 0x11CE, 0x347):
    packed = (iid << 8) | 0x44
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        r = movw(o)
        if r and r[0] == packed:
            hits.append(o)
        o += 2
    print(f"MOVW #{hex(packed)} (id {hex(iid)}): {len(hits)} {[hex(h) for h in hits[:10]]}")
    for h in hits[:3]:
        dump(h - 0x60, h + 0x30, f"packed_{hex(iid)}@{hex(h)}")

# Alternate packing: id in high half MOVT
# Search USIM window for "eSIM" check near PIN SKIP string usage via BL to strcmp-like
# Find IsEsim / eSIM flag readers near PinSkip handler by string "NOT eSIM" registration offset
# From table at 0x11608ee: r1=#0x7ec with MOVT #0x4074 → VA 0x407407ec = string?
# Check: VA 0x407407ec → file = VA - VA_base + MAIN = 0x407407ec - 0x40010000 + 0x16c10
str_va = 0x407407EC
str_off = str_va - VA + MAIN
print(f"\nreg table string candidate off={hex(str_off)}")
if 0 <= str_off < len(img):
    print(repr(img[str_off : str_off + 60]))

# Find real PinSkip handler: search for BL sites that load fail path - look for
# unique sequence MOVS then branch after eSIM check
# Search "DecodeSimPinSkipReq" as debug name — may have adjacent function ptr
j = img.find(b"DecodeSimPinSkipReq")
print(f"DecodeSimPinSkipReq @{hex(j)}")
# Often these names are in a cmd table: [id, handler, name_ptr]
# Search for VA of this string
dva = VA + (j - MAIN)
needle = struct.pack("<I", dva)
ps = []
s = 0
while True:
    k = img.find(needle, s)
    if k < 0:
        break
    ps.append(k)
    s = k + 4
print(f"DecodeSimPinSkipReq ptrs: {[hex(p) for p in ps]}")
for p in ps[:4]:
    # dump words around
    for i in range(-4, 6):
        off = p + i * 4
        v = struct.unpack_from("<I", img, off)[0]
        # if looks like code VA in MAIN range
        mark = ""
        if 0x41000000 <= v <= 0x46000000:
            fo = v - VA + MAIN
            mark = f" ->file {hex(fo)}"
        print(f"  [{i:+d}] {hex(off)}={hex(v)}{mark}")

# SupportedRatMap mutability: find writers of the map variable
# Search STR to locations logged as SupportedRatMap
# Find RRM_RRC_INIT_REQ_Handler code via unique nearby
print("\n=== SupportedRatMap log sites (MOVW search for unique) ===")
# Use string "RRM_RRC_INIT_REQ_Handler - SupportedRatMap" 
j = img.find(b"RRM_RRC_INIT_REQ_Handler - SupportedRatMap")
print(f"RRM INIT str @{hex(j) if j>=0 else None}")
# Search InitRapMap / SupportedRatMap(%d) 
for n in [
    b"InitRapMap",
    b"SupportedRatMap(%d)",
    b"SupportedRatMap(0x%X)",
    b"No CDMA in InitRapMap",
    b"UpdateSupportedRat",
    b"SetSupportedRat",
]:
    print(f"  {n}: {img.find(n)}")

# AP BuildSetPreferredNetworkType msgid
idx = stream.find(b"BuildSetPreferredNetworkType")
print(f"\nBuildSetPreferredNetworkType stream @{hex(idx)}")
# Find the function - look for nearby 0x07xx network SIT ids in code section
# From prior docs preferred=12 already tried negative

# Check if any SIT/property string mentions writing SupportedRatMap
for n in [
    b"SupportedRatMap",
    b"InitRapMap",
    b"SetRatMap",
    b"UpdateRatMap",
]:
    for label, blob in [("stream", stream), ("ril", ril)]:
        c = blob.count(n)
        if c:
            print(f"{label} {n}: {c}")
