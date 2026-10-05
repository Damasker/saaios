#!/usr/bin/env python3
"""Correct Bcc.W decode (J1/J2 not inverted); SET#6 chain; Present=0 path."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)

def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]

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

def bcc_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    # Bcond.W T3: imm32 = S:J2:J1:imm6:imm11:0  (J1/J2 direct, NOT inverted)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0x8000:
        return None
    s = (hw >> 10) & 1
    cond = (hw >> 6) & 0xF
    imm6 = hw & 0x3F
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    imm32 = (s << 20) | (j2 << 19) | (j1 << 18) | (imm6 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 21) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
    return (names[cond] if cond < 14 else str(cond), o + 4 + imm32)

print("=== Corrected Bcc.W STATUS skips ===")
for o in (0x14FB34E, 0x14FB3D8, 0x14FB4A4, 0x14FB4D8, 0x14FB514):
    print(f"  {hex(o)}: {bcc_w(o)}")

def dump(start, nbytes=0x80, lab=""):
    print(f"\n=== {lab} ===")
    o = start
    end = start + nbytes
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            bc = bcc_w(o)
            if b: extra = f" BL->{hex(b)}"
            elif bc: extra = f" B{bc[0]}.W->{hex(bc[1])}"
            if (hw & 0xFFF0) == 0xF880: extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF500) == 0xB100:
                op = (hw >> 11) & 1
                i = (hw >> 9) & 1
                imm5 = (hw >> 3) & 0x1F
                rn = hw & 7
                imm = (i << 5 | imm5) << 1
                extra = f" {'CBNZ' if op else 'CBZ'} r{rn}->{hex(o+2+imm)}"
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80: imm -= 0x100
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14:
                    extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400: imm -= 0x800
                extra = f" B->{hex(o+4+imm*2)}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

# After Present==0 CBZ skip: from 0x14fb402
dump(0x14FB402, 0xA0, "Present==0 skip -> Pin1Verified / exit?")

# SET#6 function + callers
dump(0x14C643C, 0x90, "SET#6 function")
dump(0x14B7C80, 0x80, "caller @0x14b7cbc")

print("\n=== All BL -> 0x14c643c ===")
o = SCAN_LO
while o < SCAN_HI:
    if bl_target(o) == 0x14C643C:
        print(f"  {hex(o)}")
    o += 2

print("\n=== All BL -> 0x18ec428 / 0x18ec458 ===")
o = SCAN_LO
while o < SCAN_HI:
    b = bl_target(o)
    if b in (0x18EC428, 0x18EC458):
        print(f"  {hex(o)} -> {hex(b)}")
    o += 2

# Does FirstPIN or SimInfo post call into SET#6 caller?
# Trace: 0x1f1458c callees include 0x1d76a80 — BFS for BL to 0x14c643c / 0x14b7xxx
SET6_FUNCS = {0x14C643C, 0x14B7CBC, 0x14B7CD0, 0x18EC428, 0x18EC458, 0x19916D2, 0x14FB322, 0x14C6626}
print("\n=== BFS FirstPIN for SET#6/#7/STATUS (depth 8) ===")
start = 0x1F0452C
q = [(start, 0)]
seen = {start}
hits = []
while q:
    pc, d = q.pop(0)
    if d > 8:
        continue
    o = pc
    end = min(pc + 0x300, len(IMG) - 4)
    while o < end:
        b = bl_target(o)
        if b is not None:
            if b in SET6_FUNCS or (0x14C6400 <= b <= 0x14C6600) or (0x14B7C00 <= b <= 0x14B7E00):
                hits.append((pc, d, o, b))
            if b not in seen and d < 8 and SCAN_LO <= b < SCAN_HI:
                seen.add(b)
                q.append((b, d + 1))
            o += 4
            continue
        hw = u16(o)
        if hw in (0xBD10, 0xBD30, 0xBD70, 0xBDF0):
            break
        if (hw & 0xFF00) == 0x4700 and (hw & 0x87) == 0x70:  # BX LR-ish
            break
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
        else:
            o += 2
print(f"visited={len(seen)} hits={len(hits)}")
for h in hits[:50]:
    print(f"  from={hex(h[0])} d={h[1]} at={hex(h[2])} -> {hex(h[3])}")

# What is 0x18c2acc (called before SET#6)? Pin check?
dump(0x18C2ACC, 0x40, "0x18c2acc (pre-SET#6)")

print("DONE")
