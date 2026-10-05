#!/usr/bin/env python3
"""Trace FirstPIN → SET_APP#7; fix Bcc; Present vs +0xBF6 on #7 path."""
import struct
from pathlib import Path
from collections import deque

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)
SET_APP = 0x19916D2
SET7_SITE = 0x18EC4FE
FIRSTPIN = 0x1F0452C

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

def bcc_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0x8000:
        return None
    s = (hw >> 10) & 1
    cond = (hw >> 6) & 0xF
    imm6 = hw & 0x3F
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    # ARM: SignExtend(S:J2:J1:imm6:imm11:0)
    imm32 = (s << 20) | (j2 << 19) | (j1 << 18) | (imm6 << 12) | (imm11 << 1)
    if s:
        imm32 -= 1 << 21
    names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
    return (names[cond] if cond < 14 else str(cond), o + 4 + imm32)

print("=== Bcc.W fixed ===")
for o in (0x14FB34E, 0x14FB3D8, 0x14FB4D8):
    c, t = bcc_w(o)
    print(f"  {hex(o)} B{c}.W -> {hex(t)}")

# Parent map: BFS forward, record parent BL site
print("\n=== Path FirstPIN → SET#7 site ===")
parent = {FIRSTPIN: None}  # func -> (caller_func, bl_site)
q = deque([FIRSTPIN])
found_func = None
while q:
    pc = q.popleft()
    o = pc
    end = min(pc + 0x400, len(IMG) - 4)
    while o < end:
        b = bl_target(o)
        if b is not None:
            if o == SET7_SITE or b == SET_APP and o == SET7_SITE:
                pass
            # detect SET#7 site as BL location inside some func
            if o == SET7_SITE:
                found_func = pc
                parent[pc] = parent.get(pc)  # ensure
                # record specially
                print(f"  reached SET#7 BL inside func {hex(pc)}")
                # reconstruct
                chain = []
                cur = pc
                while cur is not None:
                    chain.append(cur)
                    p = parent.get(cur)
                    if p is None:
                        break
                    cur = p[0]
                print("  call chain (funcs):", " -> ".join(hex(x) for x in reversed(chain)))
                # also print BL sites
                cur = pc
                sites = []
                while cur is not None:
                    p = parent.get(cur)
                    if p is None:
                        break
                    sites.append(p[1])
                    cur = p[0]
                print("  BL sites:", " -> ".join(hex(x) for x in reversed(sites)), "->", hex(SET7_SITE))
                q.clear()
                break
            if SCAN_LO <= b < SCAN_HI and b not in parent:
                parent[b] = (pc, o)
                q.append(b)
            o += 4
            continue
        hw = u16(o)
        if (hw & 0xFF00) == 0xBD00 or hw == 0x4770:
            break
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
        else:
            o += 2
    else:
        continue
    break
else:
    print("  NOT FOUND via forward BFS from FirstPIN alone — try reverse")

# Reverse: who BLs into function containing SET#7 (0x18ec31c from prior hit)
# Prior BFS said from=0x18ec31c d=6 at=0x18ec4fe
print("\n=== Reverse parents to 0x18ec31c / SET#7 ===")
# Find all BL to addresses in [0x18ec300, 0x18ec510]
targets_wanted = set()
# include likely function entries
for entry in range(0x18EC300, 0x18EC510, 2):
    targets_wanted.add(entry)

rev = {}  # target -> list of bl sites
o = SCAN_LO
while o < SCAN_HI:
    b = bl_target(o)
    if b and 0x18EC300 <= b <= 0x18EC510:
        rev.setdefault(b, []).append(o)
    o += 2
for t in sorted(rev):
    print(f"  BL->{hex(t)}: {[hex(x) for x in rev[t][:8]]}")

# Walk reverse from 0x18ec31c (func containing SET#7) toward FirstPIN
# First find function start: look for PUSH typically
print("\n=== Func prologue before SET#7 ===")
o = SET7_SITE
while o > SET7_SITE - 0x400:
    hw = u16(o)
    # PUSH.W lr: e92d 4xxx or b5xx
    if hw == 0xE92D or (hw & 0xFF00) == 0xB500 or (hw & 0xFF00) == 0xB400:
        # check if looks like func start
        if hw == 0xE92D or (hw & 0xFE00) == 0xB400:
            print(f"  candidate prologue {hex(o)}: {hw:04x} {u16(o+2):04x}")
    o -= 2

# Dump 0x18ec31c through SET#7
print("\n=== Dump 0x18ec31c .. SET#7+0x20 ===")
o = 0x18EC31C
while o < SET7_SITE + 0x30:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        b = bl_target(o)
        if b: extra = f" BL->{hex(b)}"
        bc = bcc_w(o)
        if bc and not b: extra = f" B{bc[0]}.W->{hex(bc[1])}"
        if (hw & 0xFF00) == 0x2000: pass
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
        mark = " <<<SET7" if o == SET7_SITE else ""
        print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}{mark}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2

# Trace path using parent from earlier BFS style with depth to SET7_SITE as instruction
print("\n=== Forward path recording BL to reach 0x18ec31c ===")
parent = {FIRSTPIN: None}
q = deque([FIRSTPIN])
hit = None
while q:
    pc = q.popleft()
    o = pc
    end = min(pc + 0x400, len(IMG) - 4)
    while o < end:
        b = bl_target(o)
        if b is not None:
            if b == 0x18EC31C or (0x18EC31C <= b <= 0x18EC4FE):
                hit = (pc, o, b)
                parent[b] = (pc, o)
                q.clear()
                break
            if SCAN_LO <= b < SCAN_HI and b not in parent and len(parent) < 80000:
                parent[b] = (pc, o)
                q.append(b)
            o += 4
            continue
        hw = u16(o)
        if (hw & 0xFF00) == 0xBD00 or hw == 0x4770:
            break
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
        else:
            o += 2
    if hit:
        break

if hit:
    print(f"  hit BL {hex(hit[1])} from func {hex(hit[0])} -> {hex(hit[2])}")
    chain = []
    cur = hit[2]
    seen = set()
    while cur is not None and cur not in seen:
        seen.add(cur)
        chain.append(cur)
        p = parent.get(cur)
        if p is None:
            break
        cur = p[0]
    print("  funcs:", " -> ".join(hex(x) for x in reversed(chain)))
else:
    print("  no hit to 0x18ec31c range")
    # check if 0x18ec31c was visited
    print(f"  parent has 0x18ec31c? {0x18EC31C in parent}")
    print(f"  parent has 0x18ec4fe? {SET7_SITE in parent}")

# Imm of SET_APP at 0x1940f34 and 0x18fe4ce (other FirstPIN hits)
print("\n=== Other SET_APP on FirstPIN BFS ===")
for site in (0x1940F34, 0x18FE4CE, 0x18EC4FE):
    imm = None
    for a in range(site - 12, site, 2):
        h = u16(a)
        if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == 0:
            imm = h & 0xFF
    print(f"  {hex(site)} imm={imm} BL->{hex(bl_target(site))}")

print("DONE")
