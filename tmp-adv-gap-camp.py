#!/usr/bin/env python3
"""Does RSM register / GapMeasure require START_NETWORK? Early Present=0 at ctor."""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)

def u16(o): return struct.unpack_from("<H", IMG, o)[0]

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

print("=== RSM register / START_NETWORK / SIM-ready coupling strings ===")
for n in [
    b"Already Deregister",
    b"RequestMeasureResource",
    b"RegisterMeasure",
    b"RSM_RSRC_MEAS",
    b"SIM is not ready",
    b"stack is not started",
    b"Stack not started",
    b"L1 not active",
    b"LTE not active",
    b"not camped",
    b"Not camped",
    b"before camping",
    b"RF not",
]:
    idx, c = 0, 0
    while c < 4:
        j = IMG.find(n, idx)
        if j < 0: break
        s0 = j
        while s0 > 0 and 32 <= IMG[s0-1] < 127: s0 -= 1
        end = IMG.find(b"\x00", j)
        frag = IMG[s0:min(end, s0+130)]
        # filter noise
        if n.startswith(b"RSM") or n.startswith(b"Already") or n.startswith(b"Request") or n.startswith(b"Register") or b"SIT" in frag or b"SADR" in frag or b"Gap" in frag or b"camp" in frag.lower() or b"SIM" in frag or b"stack" in frag.lower() or b"LTE" in frag:
            print(f"  {hex(s0)}: {frag!r}")
            c += 1
        idx = j + 1

# Post PAUSE: who calls send toward L1LC - search BL near string 0x261ad68 via ADR in 0x25-0x28M
# Thumb ADR range only ±1020 from PC - string at 0x261ad68, code must be within ~1KB
print("\n=== ADR window around POST string 0x261ad68 ===")
POST = 0x261AD68
for o in range(max(0, POST - 0x800), min(len(IMG)-2, POST + 0x100), 2):
    hw = u16(o)
    if (hw & 0xF800) == 0xA000:
        imm = (hw & 0xFF) * 4
        base = (o + 4) & ~3
        if base + imm == POST:
            print(f"  ADR @{hex(o)} -> POST")
            # dump surrounding
            for a in range(o - 0x20, o + 0x40, 2):
                h = u16(a)
                b = bl_target(a)
                if b:
                    print(f"    {hex(a)} BL->{hex(b)}")
                elif (h & 0xF800) in (0xE800, 0xF000, 0xF800):
                    pass

# Confirm Present ctor store at 0x1a552a4: 2000; 7030 = STRB r0,[r6,#0] => Present=0
print("\n=== Confirm 0x1a552a4 Present=0 init ===")
print(f"  {u16(0x1a552a4):04x} {u16(0x1a552a6):04x}")  # MOVS r0,#0; STRB r0,[r6,#0]

# FN_A is only Present=2 among getobj sites - confirm 0x14f6a14 still only in FN_A path
print("\n=== All PresentObj[0]=2 STRB (MOVS#2 STRB #0 with #636c in ±0x100) ===")
o = SCAN_LO
while o < SCAN_HI - 4:
    h = u16(o)
    if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
        rd = (h >> 8) & 7
        for a in range(o + 2, min(o + 10, SCAN_HI - 2), 2):
            h2 = u16(a)
            is_strb = False
            if (h2 & 0xF800) == 0x7000 and (h2 & 7) == rd and ((h2 >> 6) & 0x1F) == 0:
                is_strb = True
            if (h2 & 0xFFF0) == 0xF880:
                h3 = u16(a + 2)
                if ((h3 >> 12) & 0xF) == rd and (h3 & 0xFFF) == 0:
                    is_strb = True
            if not is_strb:
                continue
            # #636c nearby?
            has = False
            for b in range(max(SCAN_LO, o - 0x120), min(SCAN_HI - 4, o + 0x40), 2):
                hh, hh2 = u16(b), u16(b + 2)
                if (hh & 0xFBF0) == 0xF240 and not (hh2 & 0x8000):
                    i = (hh >> 10) & 1
                    imm = (i << 11) | ((hh & 0xF) << 12) | (((hh2 >> 12) & 7) << 8) | (hh2 & 0xFF)
                    if imm == 0x636C:
                        has = True
                        break
            if has:
                print(f"  Present=2 @{hex(o)} STRB @{hex(a)}")
    o += 2

# Factory SIT: emergency camp / limited - any Build* we know?
print("\n=== sit Build* emergency/limited/camp (ascii) ===")
for n in [b"EmergencyCamp", b"emergency_camp", b"BuildEmergency", b"LimitedService", b"StartNetwork", b"BuildStartNetwork", b"BuildRadioPower"]:
    idx = 0
    while True:
        j = IMG.find(n, idx)
        if j < 0: break
        # only if in plausible string
        if j > 0 and 32 <= IMG[j-1] < 127:
            idx = j + 1
            continue
        end = IMG.find(b"\x00", j)
        print(f"  {hex(j)}: {IMG[j:min(end,j+60)]!r}")
        idx = j + 1

print("DONE")
