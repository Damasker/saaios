#!/usr/bin/env python3
"""RSM GapMeasurePause gates vs SIM/READY; early Present init @0x1a55204; E-Camping."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)
VA_BASE, MAIN_OFF = 0x40010000, 0x16C10
GET_APP = 0x18EC8C0

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

def dump(start, nbytes=0xC0, lab=""):
    print(f"\n=== {lab} ===")
    o, end = start, start + nbytes
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b: extra = f" BL->{hex(b)}"
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
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80: imm -= 0x100
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14: extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

# Find ReceiveGapMeasurePauseFromSRL1RC code via string
s = b"ReceiveGapMeasurePauseFromSRL1RC"
j = IMG.find(s)
print(f"string @{hex(j)}")
# Shannon log: often hdr before string. Find code MOVW that loads nearby - search ADR in huge range hard.
# Find xref: VA of string
va = VA_BASE + (j - MAIN_OFF)
lo, hi = va & 0xFFFF, (va >> 16) & 0xFFFF
print(f"VA {hex(va)}")
hits = []
o = SCAN_LO
while o < SCAN_HI - 8:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        rd = (hw2 >> 8) & 0xF
        if imm == lo:
            for a in range(o + 4, min(o + 12, SCAN_HI - 4), 2):
                h, h2 = u16(a), u16(a + 2)
                if (h & 0xFBF0) == 0xF2C0 and not (h2 & 0x8000) and ((h2 >> 8) & 0xF) == rd:
                    i2 = (h >> 10) & 1
                    himm = (i2 << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
                    if himm == hi:
                        hits.append(o)
                    break
    o += 2
print(f"MOVW/T hits: {[hex(h) for h in hits[:20]]}")
for h in hits[:3]:
    dump(max(SCAN_LO, h - 0x40), 0xA0, f"ReceiveGapMeasurePause near {hex(h)}")

# Strings about gap meas needing camp/register/SIM
print("\n=== RSM/SADR strings mentioning SIM/READY/camp/register ===")
for n in [
    b"GapMeasure",
    b"SADR",
    b"E-Camping",
    b"E_CAMPING",
    b"Emergency camp",
    b"emergency camp",
    b"Limited service",
    b"cell search",
    b"CellSearch",
]:
    idx, c = 0, 0
    while c < 8:
        k = IMG.find(n, idx)
        if k < 0: break
        s0 = k
        while s0 > 0 and 32 <= IMG[s0-1] < 127: s0 -= 1
        end = IMG.find(b"\x00", k)
        frag = IMG[s0:min(end, s0 + 120)]
        low = frag.lower()
        if any(x in low for x in [b"sim", b"ready", b"pin", b"camp", b"regist", b"attach", b"plmn", b"start_net", b"absent", b"init"]):
            print(f"  {hex(s0)}: {frag!r}")
            c += 1
        idx = k + 1

# 0x18db24e - what does SET#6 gate call? (pin/app check)
dump(0x18DB24E, 0x80, "0x18db24e (SET#6 gate callee)")

# Early Present init site 0x1a55204
dump(0x1A551E0, 0x100, "getobj#636c @0x1a55204 region")

# Who BLs 0x1a55204 function - is it BOOT/USIM init?
# find func start
o = 0x1A55204
while o > 0x1A55000:
    if u16(o) == 0xE92D and (u16(o + 2) & 0x4000):
        print(f"\nfunc start {hex(o)}")
        entry = o
        callers = []
        a = SCAN_LO
        while a < SCAN_HI and len(callers) < 20:
            if bl_target(a) == entry:
                callers.append(a)
            a += 2
        print(f"callers: {[hex(c) for c in callers]}")
        break
    o -= 2

# Does START_NETWORK SIT path check GET_APP==5? Find site with CMP pattern (1,4,5)
print("\n=== GET_APP sites with CMP pattern including 5 ===")
o = SCAN_LO
while o < SCAN_HI - 20:
    if bl_target(o) == GET_APP:
        cmps = []
        a = o + 4
        for _ in range(10):
            h = u16(a)
            if (h & 0xFF00) == 0x2800:
                cmps.append((a, h & 0xFF))
            if (h & 0xF800) in (0xE800, 0xF000, 0xF800):
                a += 4
            else:
                a += 2
        vals = [c[1] for c in cmps]
        if 5 in vals:
            print(f"  {hex(o)} cmps={vals}")
            # nearby MOVW log
            for b in range(max(SCAN_LO, o - 0x30), o + 0x40, 2):
                h, h2 = u16(b), u16(b + 2)
                if (h & 0xFBF0) == 0xF240 and not (h2 & 0x8000):
                    i = (h >> 10) & 1
                    imm = (i << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
                    if imm < 0x2000:
                        print(f"    MOVW @{hex(b)} #{hex(imm)}")
    o += 2

# SRL1RC grant gap measure - need registered?
print("\n=== SRL1RC / GapMeasure grant strings ===")
for n in [b"GapMeasureUse", b"RSM_RSRC_MEAS", b"SRL1RC", b"Meas_Paused", b"RequestGapMeasure"]:
    idx, c = 0, 0
    while c < 6:
        k = IMG.find(n, idx)
        if k < 0: break
        s0 = k
        while s0 > 0 and 32 <= IMG[s0-1] < 127: s0 -= 1
        end = IMG.find(b"\x00", k)
        print(f"  {hex(s0)}: {IMG[s0:min(end,s0+110)]!r}")
        idx, c = k + 1, c + 1

print("DONE")
