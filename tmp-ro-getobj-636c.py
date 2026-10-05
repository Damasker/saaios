#!/usr/bin/env python3
"""RO: getobj(#0x636c) PresentObj — heap vs BSS; fixed VA/PA?"""
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
    """Decode MOVW Rd,#imm16 at o; return (rd, imm) or None."""
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240:
        return None
    if (hw2 & 0x8000) != 0:
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
    return rd, imm16


def movt_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0:
        return None
    if (hw2 & 0x8000) != 0:
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
    return rd, imm16


print("=== getobj body dump", hex(GETOBJ), ".. +0x120 ===")
o = GETOBJ
while o < GETOBJ + 0x140:
    hw = u16(o)
    t = bl_target(o)
    mw = movw_imm(o)
    mt = movt_imm(o)
    note = []
    if t is not None:
        note.append(f"BL->{hex(t)}")
        print(f"  {hex(o)}: {hw:04x} {u16(o+2):04x} {' '.join(note)}")
        o += 4
        continue
    if mw:
        note.append(f"MOVW r{mw[0]},#{hex(mw[1])}")
        print(f"  {hex(o)}: {hw:04x} {u16(o+2):04x} {' '.join(note)}")
        o += 4
        continue
    if mt:
        note.append(f"MOVT r{mt[0]},#{hex(mt[1])}")
        print(f"  {hex(o)}: {hw:04x} {u16(o+2):04x} {' '.join(note)}")
        o += 4
        continue
    if (hw & 0xFF00) == 0x2000:
        note.append(f"MOVS r0,#{hw & 0xFF}")
    if (hw & 0xFF00) == 0x2100:
        note.append(f"MOVS r1,#{hw & 0xFF}")
    if (hw & 0xF800) == 0x4800:
        # LDR Rt,[PC,#imm]
        rt = (hw >> 8) & 7
        imm = (hw & 0xFF) << 2
        # PC aligned
        pc = (o + 4) & ~2
        # actually Thumb LDR lit: PC = (o+4)&~3
        pc = (o + 4) & ~3
        addr = pc + imm
        if addr + 4 <= len(IMG):
            note.append(f"LDR r{rt},[PC]=->{hex(addr)} val={hex(u32(addr))}")
    if note:
        print(f"  {hex(o)}: {hw:04x} {' '.join(note)}")
    o += 2

# Find all sites that load #0x636c then BL getobj
print("\n=== sites MOVW/MOVS #0x636c near BL getobj ===")
sites = []
for o in range(0x1400000, 0x2200000, 2):
    mw = movw_imm(o)
    if not mw or mw[1] != 0x636C:
        # also MOVS can't do 0x636c
        continue
    # look ahead 0x40 for BL getobj
    for j in range(0, 0x50, 2):
        t = bl_target(o + 4 + j) if True else None
        t2 = bl_target(o + j)
        if t2 == GETOBJ or (j >= 4 and bl_target(o + j) == GETOBJ):
            sites.append((o, o + j, mw[0]))
            break
        if bl_target(o + 4 + j) == GETOBJ:
            sites.append((o, o + 4 + j, mw[0]))
            break

# broader: any MOVW #0x636c
print("all MOVW #0x636c:")
movw_sites = []
for o in range(0x1000000, 0x2800000, 2):
    mw = movw_imm(o)
    if mw and mw[1] == 0x636C:
        movw_sites.append((o, mw[0]))
print(f"count={len(movw_sites)}")
for o, rd in movw_sites[:20]:
    # find nearest BL getobj within +/- 0x80
    bls = []
    for j in range(-0x20, 0x80, 2):
        p = o + j
        if p < 0:
            continue
        if bl_target(p) == GETOBJ:
            bls.append(p)
    print(f"  {hex(o)} MOVW r{rd},#0x636c  BLgetobj={[hex(x) for x in bls[:3]]}")

# Does getobj use malloc pool or return constant pointer?
print("\n=== getobj: search LDR lit / static ptr returns in first 0x200 ===")
# Look for pool pointers in getobj callees - first BL targets
print("getobj first BLs / lit pool scan already above")

# Heuristic: if getobj is allocator, it typically calls something with size in r0/r1
# Dump more of getobj including after returns
print("\n=== getobj extended", hex(GETOBJ), hex(GETOBJ + 0x280), "===")
o = GETOBJ
end = GETOBJ + 0x280
while o < end:
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
        print(f"  {hex(o)} MOVT r{mt[0]},#{hex(mt[1])})")
        o += 4
        continue
    if (hw & 0xF800) == 0x4800:
        rt = (hw >> 8) & 7
        imm = (hw & 0xFF) << 2
        pc = (o + 4) & ~3
        addr = pc + imm
        if addr + 4 <= len(IMG):
            print(f"  {hex(o)} LDR r{rt},[PC] -> {hex(addr)} = {hex(u32(addr))}")
    if hw in (0x4770, 0xBD00) or (hw & 0xFF00) == 0xBD00:
        print(f"  {hex(o)} RET-ish {hw:04x}")
    o += 2

# Singleton pattern: after getobj, STR to a global BSS slot?
print("\n=== after BL getobj: STR to lit-pool global? (singleton) ===")
for o, rd in movw_sites:
    for j in range(0, 0x60, 2):
        p = o + j
        if bl_target(p) != GETOBJ:
            continue
        # after BL: look for STR r0,[pc,#] or MOVW/MOVT then STR
        for k in range(4, 0x40, 2):
            q = p + k
            hw = u16(q)
            # STR Rt,[Rn,#imm] classic
            if (hw & 0xF800) == 0x6000:
                print(f"  call@{hex(p)} +{k}: STR {hw:04x}")
            if (hw & 0xF800) == 0x4800:
                rt = (hw >> 8) & 7
                imm = (hw & 0xFF) << 2
                pc = (q + 4) & ~3
                addr = pc + imm
                val = u32(addr) if addr + 4 <= len(IMG) else 0
                print(f"  call@{hex(p)} +{k}: LDR r{rt} lit->{hex(addr)}={hex(val)}")
            mw2 = movw_imm(q)
            mt2 = movt_imm(q)
            if mw2:
                print(f"  call@{hex(p)} +{k}: MOVW r{mw2[0]},#{hex(mw2[1])}")
            if mt2:
                print(f"  call@{hex(p)} +{k}: MOVT r{mt2[0]},#{hex(mt2[1])}")
        break

print("\nDONE")
