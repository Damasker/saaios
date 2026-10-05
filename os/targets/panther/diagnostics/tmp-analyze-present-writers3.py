#!/usr/bin/env python3
"""Resolve case strings; full Present writer list; FN_A r9; caller of wrap case."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def off_va(v):
    return MAIN_OFF + (v - VA_BASE)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def bl_target(o):
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


def cstr_at_va(v):
    o = off_va(v)
    if o < 0 or o >= len(img):
        return None
    b = o
    while b < o + 120 and 32 <= img[b] < 127:
        b += 1
    if b == o:
        return None
    return img[o:b].decode("ascii", "replace")


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r and t is None:
                # try find movt
                pass
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                extra = f" ;BL->{hex(bt)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                extra = f" ;MOV r{hw&7},r{(hw>>3)&7}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


# Resolve format VAs used near wrap cases
print("=== Format strings near wrap dispatch ===")
pairs = [
    (0x9A14, 0x44F4, "WRAP_A log"),
    (0x99B4, 0x44F4, "near 0x14c38ce"),
    (0x9210, 0x44F4, "0x14c39c0"),
    (0x925C, 0x44F4, "0x14c39f4"),
    (0x9168, 0x44F4, "0x14c3a7c"),
    (0x91B8, 0x44F4, "0x14c3ab0"),
    (0x92B4, 0x44F4, "0x14c3b38"),
    (0x9300, 0x44F4, "0x14c3b6c"),
    (0xA36C, 0x44F4, "HELPER log"),
    (0x1F68, 0x4527, "FN_A after Present"),
]
for lo, hi, label in pairs:
    v = (hi << 16) | lo
    s = cstr_at_va(v)
    print(f"  {label}: VA={hex(v)} -> {s}")

# Find MMC_LTEL1 string xref used previously
print("\n=== MMC / measure IND strings ===")
for needle in (
    b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"CDMA_MEAS_RESULT",
    b"LTEL1",
    b"MEASURE_CNF",
    b"UMTS_MEASURE",
):
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        print(f"  @{hex(a)}: {img[a:b].decode('ascii','replace')[:100]}")
        i = j + 1
        if i > j + 1 and needle == b"LTEL1":
            break

# Function containing 0x14c3986: starts 0x14c388e
print("\n=== Fn 0x14c388e full to 0x14c398a (SIM-wrap case) ===")
print(dump(0x14C388E, 0x14C3990))

# Who BL's to 0x14c388e / 0x14c380e / WRAP_A?
print("\n=== Callers of WRAP_A / fn@0x14c388e ===")
for target in (0x14C380E, 0x14C388E, 0x14C397E, 0x14C398C):
    cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == target]
    print(f"  BL {hex(target)}: {list(map(hex, cs[:15]))} n={len(cs)}")

# FN_A: where does r9 (CMP #3) come from?
print("\n=== FN_A: track r9 / arg0 before Present store ===")
print(dump(0x14F69DC, 0x14F6A20))
# At entry: r0=arg0. Search MOV r9 from r0
print("--- entry ---")
print(dump(0x14F692C, 0x14F6960))

# Present=3: confirm same site; also search IT/MOVS#3 STRB Present pattern globally in module
print("\n=== All Present-pattern STRB.W #0 with val 0-3 (wider confirm) ===")
for o in range(0x14F0000, 0x1508000, 2):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) != 0xF880 or (hw2 & 0xFFF) != 0:
        continue
    rn, rt = hw & 0xF, (hw2 >> 12) & 0xF
    # look back for movs and IT
    vals = []
    for b in range(2, 40, 2):
        h = u16(o - b)
        if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == rt and (h & 0xFF) <= 3:
            vals.append((o - b, h & 0xFF))
    if not vals:
        continue
    related = False
    for p in range(max(0x14F0000, o - 0x80), min(0x1508000, o + 0x40), 2):
        h, h2 = u16(p), u16(p + 2)
        if (h & 0xFFF0) in (0xF880, 0xF890) and (h & 0xF) == rn and (h2 & 0xFFF) in (1, 8):
            related = True
            break
    if not related:
        continue
    print(f"  @{hex(o)} [r{rn},#0] candidates={[(hex(a),v) for a,v in vals]}")

# Primary/first: who writes Present=0 (boot clear)?
print("\n=== Context Present=0 @0x14f962e ===")
print(dump(0x14F95F0, 0x14F9650))

print("\n=== Context Present=1 @0x14f924e ===")
print(dump(0x14F9200, 0x14F9280))

print("\n=== Context Present=1 @0x14f94c2 ===")
print(dump(0x14F9480, 0x14F94E0))

# Switch table: find dispatcher that jumps to WRAP_A - search TBH or ADR table
# Prior doc said WRAP_A <- MMC_LTEL1_CDMA_MEAS_RESULT_IND switch case.
# Find that string's code xref via MOVW+MOVT of its VA
print("\n=== Xref MMC_LTEL1_CDMA_MEAS_RESULT_IND ===")
needle = b"MMC_LTEL1_CDMA_MEAS_RESULT_IND"
j = img.find(needle)
print(f"str@{hex(j)} VA={hex(va(j))}")
v = va(j)
lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
xrefs = []
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if not r or r[0] != lo:
        continue
    for p in range(max(MAIN_OFF, o - 16), min(END, o + 20), 2):
        t = movt(p)
        if t and t[0] == hi and t[1] == r[1]:
            xrefs.append(o)
            break
print(f"xrefs={list(map(hex, xrefs[:20]))} n={len(xrefs)}")
for xo in xrefs[:5]:
    print(f"\n-- {hex(xo)} --")
    print(dump(xo - 0x20, xo + 0x40))

print("DONE")
