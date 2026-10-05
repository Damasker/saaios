#!/usr/bin/env python3
"""Find SET#6 switch discriminant / msg id; string 0x8e3; Present default."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()

def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]

def u32(o):
    return struct.unpack_from("<I", IMG, o)[0]

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

# Walk back from 0x14b7b90 for TBB/TBH or CMP switch
print("=== Prolog of dispatcher containing SET#6 ===")
o = 0x14B7A00
while o < 0x14B7B90:
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
        # TBB/TBH
        if hw == 0xE8DF:
            extra = f" TBB/TBH {hw2:04x}"
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
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

# Find function start (PUSH) before 0x14b7cbc
print("\n=== Func start candidates ===")
for o in range(0x14B7900, 0x14B7B00, 2):
    hw = u16(o)
    if hw == 0xE92D:
        print(f"  PUSH.W @{hex(o)} {u16(o+2):04x}")
    if (hw & 0xFF00) == 0xB500:
        print(f"  PUSH @{hex(o)} {hw:04x}")

# Search shannon string table for id 0x8e3 with different hdr
print("\n=== brute string id 0x8e3 ===")
count = 0
for off in range(0, len(IMG) - 12, 4):
    w = u32(off)
    if ((w >> 8) & 0xFFFF) == 0x8E3 and (w & 0xFF) in (0x44, 0x45, 0x40, 0x41, 0x48):
        s = off + 8
        if s < len(IMG) and 32 <= IMG[s] < 127:
            end = IMG.find(b"\x00", s)
            print(f"  {hex(off)} lo={w&0xff:02x}: {IMG[s:min(end,s+100)]!r}")
            count += 1
            if count > 15:
                break

# Literal pool near SET#6 case: a127 at 0x14b7cb2 = ADR?
# 0xa127 = ADD r1, pc, #0x9C? 
# Thumb ADR: 1010 0 Rd imm8 -> pc&~3 + imm8*4
print("\n=== ADR at SET#6 case (msg name?) ===")
for adr_off in (0x14B7CB2, 0x14B7CC6, 0x14B7C26):
    hw = u16(adr_off)
    if (hw & 0xF800) == 0xA000:
        rd = (hw >> 8) & 7
        imm = (hw & 0xFF) * 4
        base = (adr_off + 4) & ~3
        target = base + imm
        print(f"  {hex(adr_off)} ADR r{rd},#{hex(imm)} -> {hex(target)}")
        # dump as string if ascii
        if target < len(IMG):
            chunk = IMG[target:target+64]
            print(f"    bytes: {chunk[:32].hex()}")
            # maybe pointer
            if target + 4 <= len(IMG):
                p = u32(target)
                print(f"    u32: {hex(p)}")
                if p < len(IMG) and 32 <= IMG[p] < 127:
                    end = IMG.find(b"\x00", p)
                    print(f"    str: {IMG[p:min(end,p+80)]!r}")

# Compare: STATUS_WRAP case ADR at 0x14b7c94 MOVW r1,#0xf618 — that's a VA/string id style
# MOVW r1,#0xf618; MOVT r1,#0x4106 → VA 0x4106f618?
print("\n=== Case label VAs (MOVW/MOVT r1 pairs) ===")
cases = []
o = 0x14B7B90
while o < 0x14B7CD4:
    hw = u16(o)
    if (hw & 0xFBF0) == 0xF240:
        hw2 = u16(o + 2)
        if not (hw2 & 0x8000) and ((hw2 >> 8) & 0xF) == 1:
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            # look for MOVT following
            o2 = o + 4
            hi = None
            while o2 < o + 12:
                h = u16(o2)
                h2 = u16(o2 + 2)
                if (h & 0xFBF0) == 0xF2C0 and not (h2 & 0x8000) and ((h2 >> 8) & 0xF) == 1:
                    i2 = (h >> 10) & 1
                    hi = (i2 << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
                    break
                o2 += 2
            # find BL after
            bl = None
            for a in range(o, o + 0x30, 2):
                b = bl_target(a)
                if b and 0x14C0000 <= b <= 0x14C8000:
                    bl = (a, b)
                    break
            va = (hi << 16 | imm) if hi is not None else imm
            cases.append((o, hex(va), bl))
            print(f"  {hex(o)} r1_lo={hex(imm)} r1_hi={hex(hi) if hi else None} va={hex(va) if hi else hex(imm)} bl={bl}")
    o += 2

# Resolve VA to file offset: VA_BASE 0x40010000, MAIN_OFF 0x16c10
VA_BASE = 0x40010000
MAIN_OFF = 0x16C10
print("\n=== Case label strings ===")
for o, va_s, bl in cases:
    va = int(va_s, 16)
    if va < 0x41000000:
        continue
    foff = MAIN_OFF + (va - VA_BASE)
    if 0 <= foff < len(IMG) - 4:
        # often pointer to string
        p = u32(foff) if foff + 4 <= len(IMG) else 0
        # or direct string
        if 32 <= IMG[foff] < 127:
            end = IMG.find(b"\x00", foff)
            print(f"  {hex(o)} VA {hex(va)} direct: {IMG[foff:min(end,foff+60)]!r} -> {bl}")
        elif 0x40000000 <= p <= 0x48000000:
            sf = MAIN_OFF + (p - VA_BASE)
            if 0 <= sf < len(IMG) and 32 <= IMG[sf] < 127:
                end = IMG.find(b"\x00", sf)
                print(f"  {hex(o)} VA {hex(va)} ptr->{hex(p)}: {IMG[sf:min(end,sf+60)]!r} -> {bl}")

print("DONE")
