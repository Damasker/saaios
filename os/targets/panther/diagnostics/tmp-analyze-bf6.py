#!/usr/bin/env python3
"""RO: all writers of STRB/STRH/STR to [Rn,#0xBF6] in MAIN; map values; FCP/VerifyPin."""
import struct
from pathlib import Path
from collections import Counter, defaultdict

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


# Thumb-2 STRB.W Rt,[Rn,#imm12]: F88n | Rt imm12
# Thumb-2 STR.W  Rt,[Rn,#imm12]: F8Cn | Rt imm12
# Thumb-2 STRH.W: F8An

writers = []
loaders = []
for o in range(MAIN_OFF, END - 4, 2):
    hw = u16(o)
    hw2 = u16(o + 2)
    # STRB.W
    if (hw & 0xFFF0) == 0xF880:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm = hw2 & 0xFFF
        if imm == 0xBF6:
            writers.append(("STRB.W", o, rt, rn, imm))
    # STRH.W
    if (hw & 0xFFF0) == 0xF8A0:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm = hw2 & 0xFFF
        if imm == 0xBF6:
            writers.append(("STRH.W", o, rt, rn, imm))
    # STR.W
    if (hw & 0xFFF0) == 0xF8C0:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm = hw2 & 0xFFF
        if imm == 0xBF6:
            writers.append(("STR.W", o, rt, rn, imm))
    # LDRB.W
    if (hw & 0xFFF0) == 0xF890:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm = hw2 & 0xFFF
        if imm == 0xBF6:
            loaders.append(("LDRB.W", o, rt, rn, imm))
    # LDRH.W
    if (hw & 0xFFF0) == 0xF8B0:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        imm = hw2 & 0xFFF
        if imm == 0xBF6:
            loaders.append(("LDRH.W", o, rt, rn, imm))

print(f"=== Writers of [Rn,#0xBF6]: {len(writers)} ===")
for kind, o, rt, rn, imm in writers:
    # preceding MOVS #imm into Rt
    vals = []
    for b in range(2, 40, 2):
        p = o - b
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) == rt:
            vals.append(hw & 0xFF)
        # MOV.W Rd,#imm8 / MOVS wide
        r = movw(p)
        if r and r[1] == rt and r[0] <= 0xFF:
            vals.append(r[0])
        # MOV.W modified imm F04F/F44F
        if (hw & 0xFBF0) == 0xF04F or (hw & 0xFBF0) == 0xF44F:
            hw2 = u16(p + 2)
            rd = (hw2 >> 8) & 0xF
            if rd == rt:
                vals.append(("MOV.W", hex(hw), hex(hw2)))
    print(f"  {kind} @{o:#x}/va{va(o):#x} rt=r{rt} rn=r{rn} preceding_vals={vals[:8]}")

print(f"\n=== Loaders of [Rn,#0xBF6]: {len(loaders)} ===")
for kind, o, rt, rn, imm in loaders[:40]:
    print(f"  {kind} @{o:#x}/va{va(o):#x} rt=r{rt} rn=r{rn}")
if len(loaders) > 40:
    print(f"  ... +{len(loaders)-40} more")

# Context dump each writer ±0x60 looking for log ids / MOVS
print("\n=== Writer contexts (MOVS + MOVW log + BL) ===")


def dump_ctx(start, length):
    o = start
    end = start + length
    lines = []
    while o < end:
        hw = u16(o)
        b = bl_target(o)
        r = movw(o)
        t = movt(o)
        if b is not None and (hw & 0xF800) == 0xF000:
            lines.append(f"  {o:#x}: BL->{b:#x}")
            o += 4
            continue
        if r:
            note = ""
            if r[0] in (0x7AB, 0x18E, 0x106A, 0x106B, 0xC6B, 0xC5B, 0xBDA, 0x2C5E):
                note = f" ;LOG_{r[0]:#x}"
            lines.append(f"  {o:#x}: MOVW r{r[1]},#{r[0]:#x}{note}")
            o += 4
            continue
        if t:
            lines.append(f"  {o:#x}: MOVT r{t[1]},#{t[0]:#x}")
            o += 4
            continue
        if (hw & 0xFF00) == 0x2000:
            lines.append(f"  {o:#x}: MOVS r{(hw>>8)&7},#{hw&0xff}")
            o += 2
            continue
        if (hw & 0xFFF0) == 0xF880 and (u16(o + 2) & 0xFFF) == 0xBF6:
            lines.append(f"  {o:#x}: STRB.W r{(u16(o+2)>>12)&0xf},[r{hw&0xf},#0xBF6]  <<WRITE")
            o += 4
            continue
        if (hw & 0xFFF0) == 0xF890 and (u16(o + 2) & 0xFFF) == 0xBF6:
            lines.append(f"  {o:#x}: LDRB.W <<READ")
            o += 4
            continue
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
            continue
        o += 2
    return "\n".join(lines)


for kind, o, rt, rn, imm in writers:
    print(f"\n---- {kind} @{o:#x} ----")
    print(dump_ctx(o - 0x80, 0xC0))

# Proximity: any writer within ±0x800 of 0x7ab real sites or 0x18e sites?
print("\n=== Real 0x7ab / 0x18e sites near any +0xBF6 writer (±0x1000) ===")


def real_log(log_id):
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != log_id:
            continue
        # skip dense false (next MOVW consecutive id)
        dense = False
        for p in range(o + 4, o + 32, 2):
            r2 = movw(p)
            if r2 and abs(r2[0] - log_id) == 1:
                dense = True
                break
        if dense:
            continue
        ctx = None
        for p in range(o - 24, o + 28, 2):
            t = movt(p)
            if t and 0x4000 <= t[0] <= 0x45FF:
                ctx = t[0]
                break
        if ctx is None:
            continue
        hits.append((o, ctx))
    return hits


w_offs = [o for _, o, *_ in writers]
for lid, name in [(0x7AB, "FCP_PIN_DISABLED"), (0x18E, "FirstPIN1Verify"), (0xC6B, "sitSetPin1")]:
    for o, ctx in real_log(lid):
        near = [w for w in w_offs if abs(w - o) < 0x1000]
        if near:
            print(f"  {name} @{o:#x} ctx={ctx:#x} near writers={[hex(x) for x in near]}")
        else:
            # also report distance to nearest writer
            if w_offs:
                nearest = min(w_offs, key=lambda w: abs(w - o))
                d = abs(nearest - o)
                if d < 0x8000:
                    print(f"  {name} @{o:#x} nearest writer {nearest:#x} dist={d:#x}")
