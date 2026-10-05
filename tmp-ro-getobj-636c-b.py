#!/usr/bin/env python3
"""Deeper: BL getobj with size 0x636c; singleton store; allocator vs BSS."""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
GETOBJ = 0x20EA040


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


def movw_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return rd, (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8


def movt_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return rd, (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8


# All BL -> GETOBJ
print("=== all BL getobj ===")
bls = []
for o in range(0x1000000, 0x2800000, 2):
    if bl_target(o) == GETOBJ:
        bls.append(o)
print(f"count={len(bls)}")
for o in bls:
    # look back 0x30 for MOVW #0x636c into r1 (or any)
    has = False
    size_rd = None
    for j in range(0, 0x40, 2):
        p = o - j
        if p < 0:
            break
        mw = movw_imm(p)
        if mw and mw[1] == 0x636C:
            has = True
            size_rd = mw[0]
            break
    if has:
        print(f"  {hex(o)} size_r{size_rd}=0x636c")
        # dump -0x30..+0x50
        for k in range(-0x28, 0x50, 2):
            q = o + k
            if q < 0:
                continue
            t = bl_target(q)
            mw = movw_imm(q)
            mt = movt_imm(q)
            hw = u16(q)
            bits = []
            if t is not None:
                bits.append(f"BL->{hex(t)}")
                print(f"    {hex(q)}: {' '.join(bits)}")
                continue
            if mw:
                bits.append(f"MOVW r{mw[0]},#{hex(mw[1])}")
            if mt:
                bits.append(f"MOVT r{mt[0]},#{hex(mt[1])}")
            if (hw & 0xFF00) == 0x2000:
                bits.append(f"MOVS r0,#{hw&0xff}")
            if (hw & 0xFF00) == 0x2100:
                bits.append(f"MOVS r1,#{hw&0xff}")
            # STR Rt,[Rn,#imm] Thumb 0x60xx / STRB
            if (hw & 0xF800) == 0x6000:
                bits.append(f"STR {hw:04x}")
            if (hw & 0xF800) == 0x7000:
                bits.append(f"STRB imm5={(hw>>6)&0x1f}")
            if bits:
                print(f"    {hex(q)}: {' '.join(bits)}")

# Analyze early getobj path: r12=0x49682618 string?
print("\n=== getobj tag string @ 0x49682618 ===")
# file offset may equal VA for this image layout - check
for va in [0x49682618, 0x20ea042]:
    pass
# In Shannon MAIN, often VA = file offset for code; data may differ.
# Try reading as file offset if in range
for off in [0x49682618]:
    if off < len(IMG):
        s = IMG[off : off + 80].split(b"\x00")[0]
        print(f"  file@{hex(off)}: {s!r}")
    # also search string L1LC_IratController
idx = IMG.find(b"L1LC_IratController")
print(f"  L1LC_IratController @ {hex(idx) if idx>=0 else None}")
idx2 = IMG.find(b"getobj")
print(f"  getobj str first @ {hex(idx2) if idx2>=0 else None}")

# Analyze BL 0x20e90e0 — likely allocator
ALLOC = 0x20E90E0
print(f"\n=== likely allocator {hex(ALLOC)} dump ===")
o = ALLOC
while o < ALLOC + 0x100:
    t = bl_target(o)
    mw = movw_imm(o)
    mt = movt_imm(o)
    hw = u16(o)
    if t is not None:
        print(f"  {hex(o)} BL->{hex(t)}")
        o += 4
        continue
    if mw:
        print(f"  {hex(o)} MOVW r{mw[0]},#{hex(mw[1])}")
        o += 4
        continue
    if mt:
        print(f"  {hex(o)} MOVT r{mt[0]},#{hex(mt[1])}")
        o += 4
        continue
    if (hw & 0xF800) == 0x4800:
        rt = (hw >> 8) & 7
        imm = (hw & 0xFF) << 2
        pc = (o + 4) & ~3
        addr = pc + imm
        if addr + 4 <= len(IMG):
            print(f"  {hex(o)} LDR r{rt} lit {hex(addr)}={hex(u32(addr))}")
    if (hw & 0xFF00) == 0xBD00 or hw == 0x4770:
        print(f"  {hex(o)} RET {hw:04x}")
        if o > ALLOC + 0x40:
            break
    o += 2

# Singleton globals from call sites: 0x48b87b7c
print("\n=== singleton candidate VAs ===")
cands = [0x48B87B7C, 0x45272AE4, 0x452728EC]
for va in cands:
    # Is VA in MAIN image as data?
    print(f"  VA {hex(va)} in_img={va < len(IMG)} val={hex(u32(va)) if va+4<=len(IMG) else 'n/a'}")

# Search STR r0 to absolute via MOVW/MOVT pair forming pointer then STR
print("\n=== FN_A getobj call 0x14f6956 neighborhood full ===")
base = 0x14F6920
for o in range(base, base + 0x80, 2):
    t = bl_target(o)
    mw = movw_imm(o)
    mt = movt_imm(o)
    hw = u16(o)
    bits = []
    if t:
        bits.append(f"BL->{hex(t)}")
        print(f"  {hex(o)} {' '.join(bits)}")
        continue
    if mw:
        bits.append(f"MOVW r{mw[0]},#{hex(mw[1])}")
    if mt:
        bits.append(f"MOVT r{mt[0]},#{hex(mt[1])}")
    if (hw & 0xFF00) == 0x2000:
        bits.append(f"MOVS r0,#{hw&0xff}")
    if (hw & 0xFF00) == 0x2100:
        bits.append(f"MOVS r1,#{hw&0xff}")
    if bits:
        print(f"  {hex(o)} {' '.join(bits)}")

print("DONE")
