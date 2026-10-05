#!/usr/bin/env python3
"""RE MAIN B: SIT 0x0704 (SetNetworkSelectionAuto) → error_raw=2 preconditions.

Finds opcode-literal sites, dispatch→handler, CMPs / GET_APP / Present gates,
and error=2 store paths tied to START_NETWORK / network-selection.
"""
from __future__ import annotations

import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/"
    "diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

GET_APP = 0x18EC8C0
START_NET_GATE = 0x18E831A  # known GET_APP CMP {1,4,5}
SET_APP = 0x19916D2


def u16(o: int) -> int:
    return struct.unpack_from("<H", IMG, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", IMG, o)[0]


def bl_target(o: int):
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


def movw(o: int):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    rd = (hw2 >> 8) & 0xF
    return imm, rd


def movt(o: int):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    rd = (hw2 >> 8) & 0xF
    return imm, rd


def movs_imm(o: int):
    hw = u16(o)
    if (hw & 0xFF00) == 0x2000:
        return hw & 0xFF, (hw >> 8) & 7
    return None


def cmp_imm(o: int):
    hw = u16(o)
    if (hw & 0xFF00) == 0x2800:
        return hw & 0xFF, (hw >> 8) & 7
    return None


def ldrb_w(o: int):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) != 0xF890:
        return None
    rt = (hw2 >> 12) & 0xF
    rn = hw & 0xF
    imm = hw2 & 0xFFF
    return rt, rn, imm


def dump(start: int, end: int, lab: str):
    print(f"\n=== {lab} [{hex(start)}..{hex(end)}] ===")
    o = start
    while o < end and o + 2 <= len(IMG):
        hw = u16(o)
        extra = ""
        mw = movw(o)
        mt = movt(o)
        b = bl_target(o)
        lb = ldrb_w(o)
        cm = cmp_imm(o)
        ms = movs_imm(o)
        if mw:
            extra = f" MOVW r{mw[1]},#{hex(mw[0])}"
        if mt:
            extra = f" MOVT r{mt[1]},#{hex(mt[0])}"
        if b is not None:
            tag = ""
            if b == GET_APP:
                tag = " GET_APP"
            elif b == SET_APP:
                tag = " SET_APP"
            elif abs(b - START_NET_GATE) < 0x40:
                tag = " ~START_NET"
            extra = f" BL->{hex(b)}{tag}"
        if lb:
            extra += f" LDRB.W r{lb[0]},[r{lb[1]},#{hex(lb[2])}]"
        if cm:
            extra += f" CMP r{cm[1]},#{cm[0]}"
        if ms:
            extra += f" MOVS r{ms[1]},#{ms[0]}"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
            o += 4
        else:
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


def find_movw_imm(imm: int, limit: int = 80):
    hits = []
    o = 0
    while o + 4 <= len(IMG) and len(hits) < limit:
        mw = movw(o)
        if mw and mw[0] == imm:
            hits.append((o, mw[1]))
        o += 2
    return hits


def find_u16_literal(val: int, limit: int = 40):
    """Raw little-endian halfword occurrences (may be data tables)."""
    needle = struct.pack("<H", val)
    out, off = [], 0
    while len(out) < limit:
        i = IMG.find(needle, off)
        if i < 0:
            break
        out.append(i)
        off = i + 1
    return out


print("IMG size", len(IMG), "sha-ish head", IMG[:4].hex())

# --- strings ---
print("\n=== key strings ===")
for s in [
    b"START_NETWORK Ignored: SIM is not ready",
    b"START_NETWORK Ignored",
    b"START_NETWORK",
    b"Stop other stack",
    b"SIM is not ready",
    b"SetNetworkSelection",
    b"NETWORK_SELECTION",
    b"NS_START",
    b"SIT_NET",
]:
    c = IMG.count(s)
    i = IMG.find(s)
    ctx = b""
    if i >= 0:
        ctx = IMG[i : i + 64].split(b"\x00", 1)[0]
    print(f"  count={c:3d} first={hex(i) if i>=0 else '-':10s} {s!r} ctx={ctx!r}")

# --- MOVW #0x704 sites (Thumb2) ---
print("\n=== MOVW #0x0704 sites ===")
mw704 = find_movw_imm(0x0704, 60)
print(f"  count={len(mw704)}")
for o, rd in mw704[:40]:
    # peek nearby MOVW of other SIT ids / MOVS #2
    near = []
    for j in range(max(0, o - 0x40), min(len(IMG) - 4, o + 0x80), 2):
        mw = movw(j)
        if mw and mw[0] in (0x0700, 0x0701, 0x0702, 0x0703, 0x0704, 0x0705, 0x0706, 0x070A, 0x070B, 0x0710, 0x0711, 2, 0x0200):
            near.append(f"{hex(j)}:MOVW#{hex(mw[0])}->r{mw[1]}")
        ms = movs_imm(j)
        if ms and ms[0] == 2:
            near.append(f"{hex(j)}:MOVS#2->r{ms[1]}")
        cm = cmp_imm(j)
        if cm and cm[0] in (1, 2, 4, 5):
            near.append(f"{hex(j)}:CMP#{cm[0]}")
        b = bl_target(j)
        if b == GET_APP:
            near.append(f"{hex(j)}:BL_GET_APP")
        if b is not None and abs(b - START_NET_GATE) < 0x80:
            near.append(f"{hex(j)}:BL~START@{hex(b)}")
    print(f"  {hex(o)} r{rd} near={near[:12]}")

# --- halfword 0x0704 in likely dispatch tables near NET strings ---
print("\n=== raw u16 0x0704 near SIT_NET / START_NETWORK (sample) ===")
raw704 = find_u16_literal(0x0704, 200)
print(f"  total raw hits (capped scan later): scanning {len(raw704)} first batch")
# Prefer hits that sit in aligned tables with neighboring 0x070x
tableish = []
for o in raw704:
    if o < 4 or o + 6 > len(IMG):
        continue
    prev = u16(o - 2)
    nxt = u16(o + 2)
    if 0x0700 <= prev <= 0x0715 or 0x0700 <= nxt <= 0x0715:
        tableish.append((o, prev, nxt))
print(f"  tableish neighbors: {len(tableish)}")
for o, prev, nxt in tableish[:30]:
    print(f"  {hex(o)} prev={hex(prev)} next={hex(nxt)}")

# --- Dump known START_NETWORK gate + prologue ---
print("\n=== START_NETWORK gate function (back to PUSH) ===")
prolog = None
o = START_NET_GATE
while o > START_NET_GATE - 0x600:
    hw = u16(o)
    if hw in (0xB570, 0xB5F0, 0xB5F8, 0xB5C0, 0xB580) or (hw & 0xFF00) == 0xB500:
        prolog = o
        break
    if hw == 0xE92D:  # PUSH.W
        prolog = o
        break
    o -= 2
print(f"  prolog={hex(prolog) if prolog else 'NOTFOUND'}")
if prolog:
    dump(prolog, START_NET_GATE + 0xA0, "START_NETWORK_fn")

# Who BL → START_NETWORK entry?
entry = prolog or (START_NET_GATE & ~1)
print(f"\n=== callers BL→entry~{hex(entry)} (scan NET windows) ===")
ranges = [
    (0x1800000, 0x1A00000),
    (0x1400000, 0x1600000),
    (0x2500000, 0x2700000),
]
callers = []
for lo, hi in ranges:
    p = lo
    while p + 4 < hi:
        t = bl_target(p)
        if t is not None and abs(t - entry) < 0x10:
            callers.append((p, t))
        p += 2
print(f"  callers={len(callers)}")
for p, t in callers[:40]:
    # look back for MOVW 0x704 / CMP app
    ctx = []
    for j in range(max(0, p - 0x60), p, 2):
        mw = movw(j)
        if mw:
            ctx.append(f"MOVW#{hex(mw[0])}")
        cm = cmp_imm(j)
        if cm:
            ctx.append(f"CMP#{cm[0]}")
        b = bl_target(j)
        if b == GET_APP:
            ctx.append("GET_APP")
    print(f"  BL@{hex(p)}->{hex(t)} ctx={ctx[-10:]}")

# --- For each MOVW#0x704, dump surrounding function-ish window ---
print("\n=== dumps around MOVW#0x0704 (handlers / tables) ===")
for o, rd in mw704[:12]:
    # find nearby prolog
    start = o
    for back in range(0, 0x200, 2):
        hw = u16(o - back)
        if hw in (0xB570, 0xB5F0, 0xB5F8, 0xB5C0) or hw == 0xE92D:
            start = o - back
            break
    dump(start, min(o + 0xC0, len(IMG)), f"near_MOVW704@{hex(o)}")

# --- Error path: MOVS #2 then store near START_NETWORK / 0x704 handlers ---
print("\n=== error=#2 near START_NETWORK gate (±0x200) ===")
for j in range(START_NET_GATE - 0x200, START_NET_GATE + 0x200, 2):
    ms = movs_imm(j)
    if ms and ms[0] == 2:
        print(f"  {hex(j)} MOVS r{ms[1]},#2")
    mw = movw(j)
    if mw and mw[0] == 2:
        print(f"  {hex(j)} MOVW r{mw[1]},#2")

# Trace ignore branch after CMP#5 at START_NET
print("\n=== post-CMP detail at START_NET (0x18e831a..+0x80) ===")
dump(0x18E8310, 0x18E83A0, "gate_detail")

# Search for SIT response builder that takes error code — look for string
# "GENERIC_FAILURE" / RIL errno near NET
print("\n=== GENERIC / failure strings near NET ===")
for s in [
    b"GENERIC_FAILURE",
    b"Generic failure",
    b"REQUEST_NOT_SUPPORTED",
    b"OPERATION_NOT_ALLOWED",
    b"SIM_PIN",
    b"not ready",
    b"Radio not available",
]:
    off = 0
    n = 0
    while n < 5:
        i = IMG.find(s, off)
        if i < 0:
            break
        ctx = IMG[i : i + 56].split(b"\x00", 1)[0]
        print(f"  {hex(i)} ctx={ctx!r}")
        off = i + 1
        n += 1

# Cross-check: does ignore path return to a SIT completer with #2?
# Find BLs from START_NET fn to anything, list unique targets
if prolog:
    print("\n=== BL targets from START_NETWORK fn ===")
    targets = []
    p = prolog
    end = START_NET_GATE + 0x180
    while p + 4 < end:
        t = bl_target(p)
        if t is not None:
            targets.append((p, t))
        p += 2
    for p, t in targets:
        print(f"  {hex(p)} -> {hex(t)}")

print("\nDONE")
