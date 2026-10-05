#!/usr/bin/env python3
"""Veneer at BNE targets; SET#6 function; Pin1Verified→SET#6; Present=0 path."""
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

def b_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x9000:
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
    return None

def dump(start, n=60, lab=""):
    print(f"\n=== {lab} @ {hex(start)} ===")
    o = start
    end = start + n * 4
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            bw = b_w(o)
            if b: extra = f" BL->{hex(b)}"
            elif bw: extra = f" B.W->{hex(bw)}"
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
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400: imm -= 0x800
                extra = f" B->{hex(o+4+imm*2)}"
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80: imm -= 0x100
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14:
                    extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
            # CBZ
            if (hw & 0xF500) == 0xB100:
                op = (hw >> 11) & 1
                i = (hw >> 9) & 1
                imm5 = (hw >> 3) & 0x1F
                rn = hw & 7
                imm = (i << 5 | imm5) << 1
                extra = f" {'CBNZ' if op else 'CBZ'} r{rn}->{hex(o+2+imm)}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

# Veneers — wrong Bcc decode? Check bytes at computed targets
for t in (0x15BB4CA, 0x15BB4F8, 0x15BB616, 0x14FB4CA):
    if t < len(IMG):
        dump(t, 40, f"target {hex(t)}")

# SET#6 function entry 0x14c643c full through SET#6
dump(0x14C643C, 50, "SET#6 func 0x14c643c")

# Who calls 0x14c643c — already: 0x14b7cbc. Dump that caller
dump(0x14B7C80, 40, "caller of SET#6 func")

# Search BL->0x14c643c and BL->0x14c64a4 region from VerifyPin BFS callees
# Also: does Pin1Verified=1 path call 0x18c2acc (seen before SET#6)?
print("\n=== BL sites to SET#6 entry 0x14c643c ===")
o = SCAN_LO
while o < SCAN_HI:
    if bl_target(o) == 0x14C643C:
        print(f"  {hex(o)}")
    o += 2

print("\n=== BL sites to SET#7 region entry 0x18ec428 ===")
o = SCAN_LO
while o < SCAN_HI:
    b = bl_target(o)
    if b in (0x18EC428, 0x18EC458, 0x18EC4FC):
        print(f"  {hex(o)} -> {hex(b)}")
    o += 2

# Strings near SET#6 / SET#7 for naming app states
def nearby_str(off, radius=0x200):
    # look for printable runs in nearby .rodata refs via MOVW — skip, scan ascii near code? useless
    pass

# Search log ids near SET#6
print("\n=== MOVW log-ish near SET#6 site ===")
o = 0x14C6400
while o < 0x14C6550:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        if imm < 0x2000:
            print(f"  {hex(o)} MOVW r{(hw2>>8)&0xf},#{hex(imm)}")
    o += 2

print("DONE")
