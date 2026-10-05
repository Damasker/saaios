#!/usr/bin/env python3
"""Who feeds FN_A arg0; CNT latch path loads [msg+8]; any non-CDMA case==3."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10


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


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            b = bl(o)
            extra = f" ;BL->{hex(b)}" if b is not None else ""
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                extra = f" ;LDR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# LATCH wrap: load msg+8 as arg to FN_A
# From prior: at 0x14c386c LDRB [r0,#0]; 0x14c386e LDRB [r1,#8?]; BL FN_A
# 7820 = LDRB r0,[r4,#0]; 7861 = LDRB r1,[r4,#1]? Let's decode:
# LDRB Rt, [Rn, #imm5]: 01111 imm5 Rn Rt -> (hw>>6)&0x1f = imm, (hw>>3)&7=Rn, hw&7=Rt
# 0x7820: imm=0, Rn=4, Rt=0 -> LDRB r0,[r4,#0]
# 0x7861: imm=1, Rn=4, Rt=1 -> LDRB r1,[r4,#1]  -- NOT +8!
# 0x1ca2 = ADDS r2,r4,#2?
dump(0x14C385A, 0x14C3880, "LATCH args to FN_A")

# Full latch handler from 0x14c388e / earlier SIMWRAP entry that checks [msg+8]==3
# Prior knowledge: WRAP_SIM at 0x14c3986 -> 0x14f6d02; and 0x14c380e path
# Find CMP #3 before Present=2 in FN_A
dump(0x14F6A00, 0x14F6A30, "FN_A CMP arg")

# Search switch/case tables referencing CDMA_MEAS handler VA near 0x14b77xx
# Disasm larger dispatch around 0x14b7700 for case labels
dump(0x14B7680, 0x14B7820, "big dispatch CDMA cases")

# Is there LTE_MEASURE_CNF path that BL 0x14f6d02 or FN_A?
targets = {0x14F6D02, 0x14F692C, 0x14C3986, 0x14C380E}
hits = {t: [] for t in targets}
o = MAIN
while o < len(img) - 4:
    b = bl(o)
    if b in targets:
        hits[b].append(o)
    o += 2
for t, cs in hits.items():
    print(f"BL->{hex(t)}: {len(cs)} {[hex(c) for c in cs]}")

# Check if 0x14f6d02 is ONLY reached from CDMA wrap 0x14c3986
print("MEAS_FN sole caller CDMA wrap?", hits[0x14F6D02] == [0x14C3986])

# Scan for "No CDMA in SupportedRatMap" and SupportedRatMap builders
for n in [b"No CDMA in SupportedRatMap", b"SupportedRatMap", b"CDMA is not supported"]:
    j = img.find(n)
    print(f"{n!r}: {hex(j) if j>=0 else None}")
    if j and j > 0:
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        print(" ", img[a:b].decode("ascii", "replace")[:120])
