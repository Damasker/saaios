#!/usr/bin/env python3
"""RO: all Present=2 / +0xBF6=2 writers; SET#6 interaction."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)

def u16(o): return struct.unpack_from("<H", IMG, o)[0]
def u32(o): return struct.unpack_from("<I", IMG, o)[0]

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

# STRB.W [rN, #0xBF6] with preceding MOVS #2 into that reg? or any store
print("=== ALL STRB.W #0xBF6 ===")
o = SCAN_LO
while o < SCAN_HI - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
        # lookback for MOVS into Rt (bits of STRB.W: Rt in hw2>>12)
        rt = (hw2 >> 12) & 0xF
        imm = None
        for a in range(max(SCAN_LO, o - 24), o, 2):
            h = u16(a)
            if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == (rt & 7):
                imm = h & 0xFF
        print(f"  {hex(o)} STRB.W r{rt},[r{hw&0xf},#0xBF6] lookback_imm={imm}")
    o += 2

# MOVS rX,#2 ; STRB rX,[rY,#0] near getobj #0x636c or FN_A
print("\n=== MOVS #2 + STRB [r*,#0] in 0x14f0000..0x1508000 ===")
o = 0x14F0000
hits = []
while o < 0x1508000 - 4:
    h = u16(o)
    if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
        rd = (h >> 8) & 7
        # next few instr STRB rd,[rn,#0]
        for a in range(o + 2, min(o + 12, 0x1508000 - 2), 2):
            h2 = u16(a)
            # STRB rt,[rn,#imm5] 01110 imm5 rn rt
            if (h2 & 0xF800) == 0x7000:
                rt = h2 & 7
                rn = (h2 >> 3) & 7
                imm5 = (h2 >> 6) & 0x1F
                if rt == rd and imm5 == 0:
                    hits.append((o, a, rn))
                    break
            # STRB.W
            if (h2 & 0xFFF0) == 0xF880:
                h3 = u16(a + 2)
                if ((h3 >> 12) & 0xF) == rd and (h3 & 0xFFF) == 0:
                    hits.append((o, a, h2 & 0xF))
                    break
    o += 2
print(f"  {len(hits)} hits")
for mov, st, rn in hits:
    # context: is #0x636c nearby?
    ctx = ""
    for a in range(max(0x14F0000, mov - 0x80), mov + 0x40, 2):
        h, h2 = u16(a), u16(a + 2)
        if (h & 0xFBF0) == 0xF240 and not (h2 & 0x8000):
            i = (h >> 10) & 1
            imm = (i << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
            if imm in (0x636C, 0xBF6, 0x106A):
                ctx = f" imm#{hex(imm)}@{hex(a)}"
    print(f"  MOVS@{hex(mov)} STRB@{hex(st)} [r{rn},#0]{ctx}")

# FN_A and all BL to it
FN_A = 0x14F692C
print("\n=== ALL BL -> FN_A ===")
o = SCAN_LO
while o < SCAN_HI:
    if bl_target(o) == FN_A:
        print(f"  {hex(o)}")
    o += 2

# Does SET#6 function 0x14c643c write Present / BF6 / call FN_A / STATUS?
print("\n=== BLs from SET#6 func 0x14c643c .. 0x14c6590 ===")
SET6 = 0x14C643C
interesting = {FN_A, 0x14FB322, 0x14C6626, 0x14F6D02, 0x19916D2, 0x20EA040}
o = SET6
while o < 0x14C6590:
    b = bl_target(o)
    if b:
        mark = " ***" if b in interesting or (0x14F6900 <= b <= 0x14F6B00) else ""
        print(f"  {hex(o)} -> {hex(b)}{mark}")
    o += 2

# STRB #0xBF6 or MOVS#2 STRB inside SET#6 body
print("\n=== stores in SET#6 body ===")
o = SET6
while o < 0x14C6590:
    hw = u16(o)
    if (hw & 0xFFF0) == 0xF880:
        hw2 = u16(o + 2)
        print(f"  {hex(o)} STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]")
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        print(f"  {hex(o)} MOVS r{(hw>>8)&7},#2")
    o += 2

# Exhaustive: STRB.W with imm12==0 and Rt loaded as 2, where base from getobj 0x636c
# Also search MOVS #2; STRB.W [r*, #0xBF6] — impossible if only one BF6 store
print("\n=== getobj(#0x636c) sites ===")
o = SCAN_LO
while o < SCAN_HI - 8:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        rd = (hw2 >> 8) & 0xF
        if imm == 0x636C:
            # find BL getobj nearby
            for a in range(o, min(o + 0x30, SCAN_HI - 4), 2):
                b = bl_target(a)
                if b == 0x20EA040:
                    print(f"  MOVW r{rd},#0x636c @{hex(o)}; BL getobj @{hex(a)}")
                    break
    o += 2

print("DONE")
