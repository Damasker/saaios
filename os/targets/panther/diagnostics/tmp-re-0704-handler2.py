#!/usr/bin/env python3
"""Pin SIT 0x0704 handler → error=2 path; list every precondition."""
from __future__ import annotations
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/"
    "diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

GET_APP = 0x18EC8C0
START_NET = 0x18E8028


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(o):
    if o + 4 > len(IMG):
        return None
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


def b_w_target(o):
    """Decode B.W / Bxx.W (unconditional/conditional wide branch)."""
    hw, hw2 = u16(o), u16(o + 2)
    # B.W: 11110 S imm10 | 10 J1 1 J2 imm11
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
    # B<cond>.W: 11110 S cond imm6 | 10 J1 0 J2 imm11
    if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x8000:
        s = (hw >> 10) & 1
        cond = (hw >> 6) & 0xF
        imm6 = hw & 0x3F
        j1 = (hw2 >> 13) & 1
        j2 = (hw2 >> 11) & 1
        imm11 = hw2 & 0x7FF
        i1 = ~(j1 ^ s) & 1
        i2 = ~(j2 ^ s) & 1
        imm32 = (s << 20) | (i1 << 19) | (i2 << 18) | (imm6 << 12) | (imm11 << 1)
        if s:
            imm32 |= ~((1 << 21) - 1) & 0xFFFFFFFF
            if imm32 >= 0x80000000:
                imm32 -= 0x100000000
        return o + 4 + imm32, cond
    return None


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


def dump(start, end, lab):
    print(f"\n=== {lab} [{hex(start)}..{hex(end)}] ===")
    o = start
    while o < end:
        hw = u16(o)
        extra = ""
        mw, mt, b = movw(o), movt(o), bl_target(o)
        bw = b_w_target(o)
        if mw:
            extra = f" MOVW r{mw[1]},#{hex(mw[0])}"
        if mt:
            extra = f" MOVT r{mt[1]},#{hex(mt[0])}"
        if b is not None:
            tag = " GET_APP" if b == GET_APP else (" START_NET" if abs(b - START_NET) < 8 else "")
            extra = f" BL->{hex(b)}{tag}"
        if isinstance(bw, tuple):
            extra += f" Bcond.W->{hex(bw[0])} cond={bw[1]}"
        elif isinstance(bw, int):
            extra += f" B.W->{hex(bw)}"
        if (hw & 0xFF00) == 0x2000:
            extra += f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFFF0) == 0xF890:
            hw2 = u16(o + 2)
            extra += f" LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
        if (hw & 0xFFF0) == 0xF880:
            hw2 = u16(o + 2)
            extra += f" STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
            o += 4
        else:
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# 1) Resolve ignore branch after CMP#5
print("=== START_NET ignore branch target ===")
# 0x18e8330: f040 80a8
r = b_w_target(0x18E8330)
print(f"  @0x18e8330 -> {r}")
# continue path uses string 0x44f39ddc — find "START_NETWORK Ignored" xref via nearby ADR
ign = IMG.find(b"START_NETWORK Ignored: SIM is not ready")
print(f"  string @ {hex(ign)}")

# Scan for ADR/MOVW pairs pointing near that string (VA often = file offset for this TOC)
# Many Shannon MAINs: string VA = 0x40000000+off or similar; prior used 0x44f3xxxx
# Look for MOVW/MOVT building address near ignore string file offset
# From dump: MOVW r0,#0x9ddc; MOVT r0,#0x44f3 at continue — different string
# Search MOVW immediates that match low 16 of ignore string file offs
for label, off in [
    ("Ignored SIM not ready", ign),
    ("Ignored SIM Absent", IMG.find(b"START_NETWORK Ignored: SIM%d Absent")),
]:
    lo = off & 0xFFFF
    hi = (off >> 16) & 0xFFFF
    print(f"  {label} file={hex(off)} lo={hex(lo)} hi={hex(hi)}")
    # also try with common bases
    for base_hi in (0x44F3, 0x4C18, hi, (0x40000000 + off) >> 16):
        hits = []
        o = 0x1800000
        while o < 0x1A00000 and len(hits) < 8:
            mw = movw(o)
            if mw and mw[0] == (off & 0xFFFF):
                mt = movt(o + 4)
                if mt and mt[1] == mw[1]:
                    hits.append((o, mt[0]))
            o += 2
        if hits:
            print(f"    base_hi try {hex(base_hi)} movw-hits {hits[:5]}")

# 2) Dump ignore target region + look for MOVS #2 / error complete
if isinstance(r, tuple):
    tgt = r[0]
elif isinstance(r, int):
    tgt = r
else:
    tgt = 0x18E8480  # guess
print(f"\n=== ignore/fail path dump around {hex(tgt)} ===")
dump(tgt - 0x20, tgt + 0x100, "ignore_path")

# Also dump after the BNE.W landing from CMP#5 fail — compute carefully
hw, hw2 = u16(0x18E8330), u16(0x18E8332)
print(f"raw branch words {hw:04x} {hw2:04x}")

# 3) Interesting SIT 0x704 sites with MOVS#2 nearby
sites = [0x146C6C2, 0x17DB3B2, 0x1958C12, 0x1C69D66, 0x1F95F32, 0x114E626, 0x1333B1E]
for s in sites:
    dump(s - 0x80, s + 0xA0, f"site_{hex(s)}")

# 4) Who BL→START_NET from SIT NET layer? Look for MOVW#0x704 within 0x100 of callers
print("\n=== START_NET callers with nearby 0x704 / SIM checks ===")
callers = []
for lo, hi in [(0x1400000, 0x1600000), (0x1800000, 0x1A00000)]:
    p = lo
    while p + 4 < hi:
        t = bl_target(p)
        if t == START_NET:
            callers.append(p)
        p += 2
for p in callers:
    has704 = False
    near = []
    for j in range(max(0, p - 0x100), min(len(IMG) - 4, p + 0x40), 2):
        mw = movw(j)
        if mw and mw[0] == 0x704:
            has704 = True
            near.append(f"704@{hex(j)}")
        if mw and mw[0] in (0x700, 0x701, 0x703, 0x70A, 0x710, 2):
            near.append(f"#{hex(mw[0])}@{hex(j)}")
        b = bl_target(j)
        if b == GET_APP:
            near.append(f"GET_APP@{hex(j)}")
    print(f"  caller {hex(p)} has704={has704} near={near[:15]}")

# 5) Find SIT dispatch: register 0x704 with function pointer
# Pattern at 0x114166e: MOVW r0,#0x704; MOV r1,#imm; BLX table
# Dump more of registration and resolve handler ptr if stored
print("\n=== SIT register #0x704 context (0x114166e) — look for handler addr ===")
dump(0x1141660, 0x11416A0, "reg704")

# Search for function that CMPs request id to 0x704 then branches
print("\n=== CMP against 0x704 (MOVW then CMP) ===")
# MOVW rx,#0x704 then later CMP ry,rx — hard; instead find LDRH/CMP of #0x704
count = 0
o = 0x1000000
while o < 0x2000000 and count < 40:
    mw = movw(o)
    if mw and mw[0] == 0x704:
        # look ahead 0x40 for CMP rn,rm or SUBS
        window = []
        for j in range(o, o + 0x50, 2):
            hw = u16(j)
            if (hw & 0xFF00) == 0x2800:
                window.append(f"CMP#{hw&0xff}@{hex(j)}")
            b = bl_target(j)
            if b == GET_APP:
                window.append(f"GET_APP@{hex(j)}")
            if b == START_NET:
                window.append(f"START_NET@{hex(j)}")
            # MOVS #2
            if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
                window.append(f"MOVS#2@{hex(j)}")
        if any("GET_APP" in x or "START_NET" in x or "MOVS#2" in x for x in window):
            print(f"  {hex(o)} r{mw[1]} => {window}")
            count += 1
    o += 2

# 6) Resolve string at continue path 0x44f39ddc — what does it say?
# Prior dump used file-relative? Check if 0x44f39ddc - 0x40000000 style
for va in (0x44F39DDC, 0x44F39CD0, 0x44F39D30, 0x44F39D80, 0x44F3A0D0, 0x44F3A138):
    # try file offset = va - base for common Shannon bases
    for base in (0x40000000, 0x0, 0x1000000):
        off = va - base
        if 0 <= off < len(IMG) - 8:
            s = IMG[off : off + 64].split(b"\x00", 1)[0]
            if s and all(32 <= b < 127 or b in (9, 10) for b in s[:20]):
                print(f"  VA {hex(va)} base {hex(base)} -> {s!r}")

print("\nDONE")
