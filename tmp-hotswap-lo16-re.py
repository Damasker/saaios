#!/usr/bin/env python3
"""Find HotSwap handlers via lo16 / ADR / USIM state; RatMap via log-id style."""
from __future__ import annotations
import struct
from pathlib import Path
from collections import defaultdict

PATH = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin")
MAIN, END, VA0 = 0x16C10, 0x16C10 + 0x05917ACC, 0x40010000
img = PATH.read_bytes()

SET_APP, STATUS, FN_A = 0x19916D2, 0x14FB322, 0x14F692C
STATUS_WRAP, GET_APP, SET1 = 0x14C6626, 0x18EC8C0, 0x146A99C
HELPER, GETOBJ = 0x1991838, 0x20EA040
INTEREST = {SET_APP, STATUS, FN_A, STATUS_WRAP, GET_APP, SET1, HELPER, GETOBJ}

def u16(o): return struct.unpack_from("<H", img, o)[0]
def va(o): return VA0 + (o - MAIN)
def off_va(v): return MAIN + (v - VA0)

def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000): return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000): return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

def bl_target(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000: return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1, j2, imm11 = (hw2 >> 13) & 1, (hw2 >> 11) & 1, hw2 & 0x7FF
    i1, i2 = ~(j1 ^ s) & 1, ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000: imm32 -= 0x100000000
    return o + 4 + imm32

def cstr(v, lim=100):
    o = off_va(v)
    if o < 0 or o >= len(img): return None
    b = o
    while b < o + lim and 32 <= img[b] < 127: b += 1
    return img[o:b].decode("ascii", "replace") if b > o else None

def dump(start, end):
    o, lines = start, []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r: extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t: extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None: extra = f" ;BL->{hex(bt)}"
            if (hw & 0xFFF0) == 0xF880: extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}"); o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000: extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}"); o += 2
    return "\n".join(lines)

def fn_start(xo, max_back=0x1800):
    for p in range(xo, max(MAIN, xo - max_back), -2):
        h = u16(p)
        if (h & 0xFFF0) == 0xE92D or h in (0xB5F0, 0xB570, 0xB5B0, 0xB580, 0xB5C0):
            return p
    return None

def scan_fn(fn, span=0x1400):
    ints, p2, bf6 = [], [], []
    for p in range(fn, min(END, fn + span), 2):
        bt = bl_target(p)
        if bt in INTEREST: ints.append((p, bt))
        hw, hw2 = u16(p), u16(p + 2) if p + 2 < END else 0
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6: bf6.append(p)
        if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) == 0:
            for b in range(2, 12, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2 and ((h >> 8) & 7) == (hw & 7):
                    p2.append(p); break
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
            for b in range(2, 20, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
                    p2.append(p); break
    return ints, p2, bf6

print("index MOVW lo16...")
movw_idx = defaultdict(list)
for o in range(MAIN, END - 4, 2):
    r = movw(o)
    if r: movw_idx[r[0]].append((o, r[1]))

# HotSwap string VAs — search lo16 with nearby MOVT in 0x44xx
targets = {
    "HotSwapInsertTimer state": 0x44CFD847,
    "HotSwapInsert hotswapstate": 0x44CFD893,
    "SIM insertion diag": 0x44D48F6F,
    "SIM_PRESENT_IND CARD_PRESENT": 0x44D02CB3,
    "SIM_PRESENT_IND WAIT_INIT": 0x44D02997,
    "ABSENT->WAIT_INIT": 0x44CF2F6B,
    "sitTx CardPower SIT1": 0x44C2D737,
    "SIM STATUS update Present": 0x44D48A3B,
    "NoCDMA Init": 0x4449C737,
    "NoCDMA Supp": 0x4449C85F,
    "SuppRatMap hex": 0x4449C6B3,
}

print("\n=== lo16 + nearby MOVT matching hi16 ===")
for name, v in targets.items():
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    hits = []
    for o, rd in movw_idx.get(lo, ()):
        # wider window; also accept ADD/ORR patterns after
        matched = False
        for p in range(max(MAIN, o - 32), min(END, o + 40), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == rd:
                hits.append((o, p, "movt")); matched = True; break
        if not matched:
            # record lo16-only in USIM-ish bands for manual
            if 0x1400000 <= o <= 0x1C00000 or 0x1700000 <= o <= 0x1800000:
                hits.append((o, None, "lo16-band"))
    print(f"\n## {name} VA={hex(v)} hits={len(hits)}")
    for o, p, kind in hits[:8]:
        print(f"  {kind} MOVW@{hex(o)}" + (f" MOVT@{hex(p)}" if p else ""))
        if kind == "movt":
            fn = fn_start(o)
            print(f"  fn~{hex(fn) if fn else None}")
            if fn:
                ints, p2, bf6 = scan_fn(fn)
                print(f"  BLs: {[(hex(a), hex(b)) for a,b in ints[:16]]}")
                print(f"  STRB#0=2: {list(map(hex, p2[:6]))} BF6: {list(map(hex, bf6[:4]))}")
            print(dump(o - 0x10, o + 0x30))

# ADR.W T3: ADDW Rd, PC, #imm12  encoding F20F / F20D bits
# Also LDR.W Rt,[PC,#imm] F8DF
print("\n=== ADR/LDR.W PC-rel to HotSwapInsert VA ===")
hot = 0x44CFD847
adr_hits = []
for o in range(MAIN, END - 4, 2):
    hw, hw2 = u16(o), u16(o + 2)
    # ADDW Rd, Rn, #imm12 where Rn=15 (PC): hw & 0xFBFF == 0xF20F? 
    # ADDW: 11110 i 10 0000 Rn | 0 imm3 Rd imm8  → F2x0 / F2xF for Rn=15
    if (hw & 0xFBEF) == 0xF20F and (hw2 & 0x8000) == 0:
        i = (hw >> 10) & 1
        imm = (i << 11) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        # PC = o+4 aligned
        base = (o + 4) & ~3
        if base + imm == off_va(hot) or VA0 + (base - MAIN) + imm == hot:
            adr_hits.append(o)
    # LDR.W Rt, [PC, #imm12]  F8DF
    if hw == 0xF8DF:
        imm = hw2 & 0xFFF
        base = (o + 4) & ~3
        lit = base + imm
        if lit + 4 <= len(img):
            val = struct.unpack_from("<I", img, lit)[0]
            if val == hot:
                adr_hits.append(o)
print(f"ADR/LDR hits: {list(map(hex, adr_hits[:20]))}")

# Trace parents of SET#1 callers — who calls those fns?
print("\n=== Callers of SET#1 parent fns ===")
for parent in (0x1453306, 0x14535A2, 0x1A2AA38):
    cs = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == parent]
    print(f"BL->{hex(parent)} n={len(cs)} {list(map(hex, cs[:20]))}")
    for c in cs[:5]:
        fn = fn_start(c)
        print(f"  from {hex(c)} fn~{hex(fn) if fn else None}")
        if fn:
            ints, p2, bf6 = scan_fn(fn, 0x800)
            print(f"  BLs: {[(hex(a), hex(b)) for a,b in ints[:12]]}")

# CardPower SIT handler: find SIT_SET_SIM_CARD_POWER string (0x40 range)
print("\n=== CardPower SIT path vs SET#1 ===")
so = img.find(b"SIT_SET_SIM_CARD_POWER")
print(f"SIT_SET_SIM_CARD_POWER @{hex(so) if so>=0 else -1} VA={hex(va(so)) if so>=0 else None}")
if so >= 0:
    v = va(so)
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    for o, rd in movw_idx.get(lo, ()):
        for p in range(o, min(END, o + 20), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == rd:
                fn = fn_start(o)
                print(f"  xref@{hex(o)} fn~{hex(fn) if fn else None}")
                if fn:
                    ints, p2, bf6 = scan_fn(fn, 0x600)
                    print(f"  BLs: {[(hex(a), hex(b)) for a,b in ints]}")
                    print(f"  touches SET1? {any(b==SET1 for _,b in ints)} SET_APP? {any(b==SET_APP for _,b in ints)}")

# 0x024c opcode dispatch — already known live CardPower works without ABSENT
# Search MOVW #0x24c with MOVT that looks like SIT table in SIM module
print("\n=== sitTxSimSetCardPower via 0x40xxxxxx string (working pattern) ===")
# From prior: A[SIT_*] might be log-only. Find "sitTxSimSetCardPower" lowercase at 0x40?
for n in [b"sitTxSimSetCardPower", b"SitTxSimSetCardPower", b"SetCardPower"]:
    i = 0
    while True:
        j = img.find(n, i)
        if j < 0: break
        print(f"  {n!r} @{hex(j)} VA={hex(va(j))} ctx={img[max(0,j-8):j+40]!r}")
        i = j + 1
        if i > MAIN + 0x2000000: break

# Helper 0x1991838: decode switch on arg — CMP #0x11 at start
print("\n=== Helper 0x1991838 role (L1LC msg object state) ===")
print(dump(0x1991838, 0x19918C0))
# string near helper from MOVW in body
for o in range(0x1991838, 0x1991900, 2):
    r = movw(o)
    if not r: continue
    for p in range(o, o + 16, 2):
        t = movt(p)
        if t and t[1] == r[1]:
            s = cstr((t[0] << 16) | r[0])
            if s: print(f"  str@{hex(o)}: {s[:80]}")

# Signed SADR: is there AP SIT opcode for measure? Search Build* table near 0x07xx
print("\n=== SIT Build* strings containing Measure/Sadr/Camp ===")
for n in [b"BuildSet", b"BuildGet", b"BuildStart", b"BuildMeasure", b"BuildSadr",
          b"sitTxSim", b"sitTxNet", b"sitTxNas"]:
    count = 0
    i = 0
    samples = []
    while count < 200:
        j = img.find(n, i)
        if j < 0: break
        s = img[j:j+48]
        if all(32 <= b < 127 or b == 0 for b in s[:20]):
            samples.append(s.split(b"\0")[0].decode("ascii", "replace"))
            count += 1
        i = j + 1
    meas = [s for s in samples if any(k in s.lower() for k in ("meas", "sadr", "camp", "attach", "hot"))]
    if meas:
        print(f"  {n!r} meas-like: {meas[:15]}")

print("\nDONE")
