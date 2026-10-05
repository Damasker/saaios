#!/usr/bin/env python3
"""Resolve SIT 0x0704 handler from register(site) → function; map error=2 checks."""
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


def u32(o):
    return struct.unpack_from("<I", IMG, o)[0]


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


def bcond_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0x8000:
        return None
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
    o = max(0, start)
    end = min(end, len(IMG))
    while o < end:
        hw = u16(o)
        extra = ""
        mw, mt, b = movw(o), movt(o), bl_target(o)
        bc = bcond_w(o)
        if mw:
            extra = f" MOVW r{mw[1]},#{hex(mw[0])}"
        if mt:
            extra = f" MOVT r{mt[1]},#{hex(mt[0])}"
        if b is not None:
            tag = ""
            if b == GET_APP:
                tag = " GET_APP"
            elif abs(b - START_NET) < 8:
                tag = " START_NET"
            elif b == 0x20D1AFA:
                tag = " SIT_REG"
            extra = f" BL->{hex(b)}{tag}"
        if bc:
            extra += f" Bcond.W->{hex(bc[0])} c={bc[1]}"
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
            if (hw & 0xFF87) == 0x4780:
                extra += f" BLX r{(hw>>3)&0xf}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


def va_to_off(va: int):
    """Try common Shannon TOC bases used in this MAIN."""
    cands = []
    for base in (0x40000000, 0x0, 0x1000000, 0x40D40000, 0x407B0000):
        off = va - base
        if 0 <= off < len(IMG):
            cands.append((base, off))
    # also: sometimes VA == file offset for code, data at VA-0x40000000
    return cands


# From registration:
# MOVW r1,#0xe4a0; MOVT r1,#0x407b => 0x407BE4A0
desc_va = 0x407BE4A0
print(f"=== descriptor VA {hex(desc_va)} ===")
for base, off in va_to_off(desc_va):
    print(f"  base {hex(base)} off {hex(off)}")
    # dump 64 bytes as words
    words = [hex(u32(off + i)) for i in range(0, 64, 4)]
    print(f"  words: {words}")
    # look for thumb ptrs
    for i in range(0, 64, 4):
        p = u32(off + i) & ~1
        if 0x1000000 <= p < 0x4000000:
            print(f"  +{i} codeish {hex(p)} prolog={u16(p):04x}")

# Also resolve sibling descriptors around registration (0x702..0x706)
print("\n=== register cluster 0x1150ae8 descriptors ===")
dump(0x1150AE0, 0x1150B80, "sit_reg_702_706")

# Parse each MOVW opcode + MOVW/MOVT r1 pair
o = 0x1150AE0
while o < 0x1150B80:
    mw0 = movw(o)
    if mw0 and 0x700 <= mw0[0] <= 0x710 and mw0[1] == 0:
        # find r1 address nearby
        addr = None
        for j in range(o - 0x10, o + 0x20, 2):
            mw = movw(j)
            mt = movt(j)
            if mw and mw[1] == 1 and mt is None:
                # look for matching MOVT r1
                for k in range(j + 4, j + 16, 2):
                    mt = movt(k)
                    if mt and mt[1] == 1:
                        addr = (mt[0] << 16) | mw[0]
                        break
            if mt and mt[1] == 1:
                # paired with earlier movw
                pass
        print(f"  opcode site {hex(o)} imm={hex(mw0[0])}")
    o += 2

# Manual extract from dump pattern:
# 0x1150ae4 MOVW r1,#0xe3f8; ... MOVT r1,#0x407b => 0x407BE3F8 for 0x702
# 0x1150afa MOVW r1,#0xe44c; MOVT #0x407b => 0x407BE44C for 0x703
# 0x1150b10 MOVW r1,#0xe4a0; MOVT #0x407b => 0x407BE4A0 for 0x704
for name, va in [
    ("0x702", 0x407BE3F8),
    ("0x703", 0x407BE44C),
    ("0x704", 0x407BE4A0),
    ("0x705", 0x407BE4A0 + 0x54),  # guess stride
]:
    print(f"\n-- desc {name} VA {hex(va)} --")
    for base, off in va_to_off(va):
        if base != 0x40000000 and base != 0x0:
            # try 0x40000000 primarily for 0x40xxxxxx
            pass
        # For 0x407Bxxxx, base 0x40000000 → off 0x7Bxxxx
        if (va & 0xFF000000) == 0x40000000:
            off = va - 0x40000000
            if off >= len(IMG):
                continue
            print(f"  file off {hex(off)}")
            for i in range(0, 0x50, 4):
                w = u32(off + i)
                p = w & ~1
                mark = ""
                if 0x1000000 <= p < 0x4000000:
                    mark = f"  << handler? prolog={u16(p):04x}"
                print(f"   +{i:02x}: {w:08x}{mark}")
            break

# --- Dump likely handler if found ---
# Also check registration helper 0x20d1afa signature by dumping it
dump(0x20D1AFA, 0x20D1B80, "SIT_REG_helper")

# --- Site 0x1738978 region: multiple 0x704 — likely request handler ---
dump(0x1738800, 0x1738F80, "region_1738_704")

# --- Site 0x114e626: CMP dispatch? ---
dump(0x114E5C0, 0x114E6C0, "dispatch_114e")

# --- Site 0x1333b1e ---
dump(0x1333A80, 0x1333B80, "dispatch_1333")

# --- Trace START_NET fail: after CMP#5 BNE, also check short local branches ---
# Maybe the real ignore is the short path via f040 being misread?
# Look for ADR to "START_NETWORK Ignored: SIM is not ready"
ign = IMG.find(b"START_NETWORK Ignored: SIM is not ready")
print(f"\nignore string file off {hex(ign)}")
# In this image, log IDs often loaded as MOVW/MOVT to 0x44F3xxxx style
# From earlier continue path uses 0x44f39ddc — compute delta to ignore string
# Search MOVW of low16 near START_NET for ignore string
# String at 0x4c18afc → if VA = 0x40000000+off = 0x4c18afc, lo=0x8afc hi=0x4c1
# Search MOVW #0x8afc in START_NET fn
print("MOVW #0x8afc / #0xafc near START_NET:")
for j in range(0x18E8028, 0x18E8800, 2):
    mw = movw(j)
    if mw and mw[0] in (0x8AFC, 0xAFC, 0x814C, 0x9DDC, 0x9CD0, 0x9D30, 0x9D80, 0xA0D0, 0xA138):
        mt = movt(j + 4)
        print(f"  {hex(j)} MOVW r{mw[1]},#{hex(mw[0])} mot={mt}")

# Find all xrefs (MOVW lo) to ignore string within NET
lo = ign & 0xFFFF
print(f"\nAll MOVW #{hex(lo)} in image (ignore str lo):")
hits = []
o = 0
while o + 4 < len(IMG) and len(hits) < 20:
    mw = movw(o)
    if mw and mw[0] == lo:
        mt = movt(o + 4)
        hits.append((o, mw[1], mt))
    o += 2
for h in hits:
    print(f"  {hex(h[0])} r{h[1]} mot={h[2]}")
    dump(h[0] - 0x40, h[0] + 0x80, f"xref_ignore_{hex(h[0])}")

# --- Critical: who returns SIT error for network selection? ---
# Search factory sit-base enum wasn't in MAIN; look for response builder that
# stores error at packet+10. Pattern: MOVS rX,#2 then STRB to offset 10.
print("\n=== MOVS#2 + STRB.W #0x0a (error_raw offset) near NET ===")
for lo, hi in [(0x1700000, 0x1800000), (0x18E0000, 0x1980000), (0x1100000, 0x1200000)]:
    p = lo
    while p + 8 < hi:
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            # look ahead 0x30 for STRB.W rt,[rn,#0xa] or [#0x0a]
            for j in range(p, p + 0x30, 2):
                h1 = u16(j)
                if (h1 & 0xFFF0) == 0xF880:
                    h2 = u16(j + 2)
                    imm = h2 & 0xFFF
                    if imm in (0xA, 0x10, 8, 12):
                        print(f"  {hex(p)} MOVS#2 → STRB.W #{hex(imm)} @{hex(j)}")
        p += 2

print("\nDONE")
