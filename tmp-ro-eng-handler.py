#!/usr/bin/env python3
"""RO CP handler for SIT 0x908 ENG_MODE — does it START_NETWORK / skip GET_APP?"""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


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


# Handler candidate 0x195b076 — dump ±0x80 with BLs
print("=== handler @0x195b000 ===")
base = 0x195B000
for o in range(base, base + 0x200, 2):
    t = bl_target(o)
    m = movw_imm(o)
    hw = u16(o)
    bits = []
    if t is not None:
        bits.append(f"BL->{hex(t)}")
        print(f"  {hex(o)}: {' '.join(bits)}")
        continue
    if m:
        bits.append(f"MOVW r{m[0]},#{hex(m[1])}")
    if (hw & 0xFF00) == 0x2000:
        bits.append(f"MOVS r0,#{hw&0xff}")
    if (hw & 0xFF00) == 0x2100:
        bits.append(f"MOVS r1,#{hw&0xff}")
    if bits:
        print(f"  {hex(o)}: {' '.join(bits)}")

# Does any BL from 0x908 sites land near START_NETWORK 0x18e831a?
START = 0x18E831A
print("\n=== BLs from MOVW#0x908 neighborhoods toward START/GET_APP ===")
sites = []
for o in range(0x1400000, 0x2200000, 2):
    m = movw_imm(o)
    if m and m[1] == 0x908:
        sites.append(o)

GETAPP = 0x18EC8C0
for o in sites:
    bls = []
    for j in range(-0x40, 0x100, 2):
        p = o + j
        if p < 0:
            continue
        t = bl_target(p)
        if t is None:
            continue
        if abs(t - START) < 0x200 or abs(t - GETAPP) < 0x40:
            bls.append((p, t, "START/GETAPP"))
        # also note interesting
    if bls:
        print(hex(o), bls)

# Search "isn't supported" near ENG
print("\n=== ENG unsupported? ===")
for n in [
    b"SIT_SET_ENG_MODE isn't",
    b"ENG_MODE isn't",
    b"SET_ENG_MODE isn't",
    b"EngMode isn't",
    b"not support Eng",
    b"EngMode timer",
]:
    j = IMG.find(n)
    print(n, hex(j) if j >= 0 else None)

# Xref: who uses EngMode timer string
eng_timer = IMG.find(b"EngMode timer")
print("EngMode timer @", hex(eng_timer) if eng_timer >= 0 else None)

# Lit refs to EngMode timer VA
if eng_timer >= 0:
    pat = struct.pack("<I", eng_timer)
    refs = []
    start = 0
    while len(refs) < 10:
        j = IMG.find(pat, start)
        if j < 0:
            break
        if abs(j - eng_timer) > 4:
            refs.append(j)
        start = j + 1
    print("lit refs", [hex(r) for r in refs])

# Search for SIT dispatch table entry: halfword 0x0908 followed by handler ptr
print("\n=== scan for 08 09 as sit-id little-endian near code ptrs ===")
# In SIT tables often struct {u16 id; ...; void* handler}
count = 0
for o in range(0x1000000, 0x2800000, 2):
    if u16(o) == 0x0908:
        # check if nearby looks like table (another sit id within ±0x20)
        neigh = [u16(o + k) for k in range(-0x20, 0x30, 2)]
        sitish = [hex(x) for x in neigh if 0x200 <= x <= 0xA00]
        if len(sitish) >= 3:
            print(f"  tableish @{hex(o)} neigh_sit={sitish[:12]}")
            count += 1
            if count >= 15:
                break

# Critical: START_NETWORK gate still only place? Confirm no second ignore string path
print("\n=== all START_NETWORK Ignored sites ===")
off = 0
while True:
    j = IMG.find(b"START_NETWORK Ignored", off)
    if j < 0:
        break
    print(hex(j), IMG[j : j + 60].split(b"\x00")[0])
    off = j + 1

print("DONE")
