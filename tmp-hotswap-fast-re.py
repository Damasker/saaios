#!/usr/bin/env python3
"""FAST RO: HotSwap INSERT chain, getobj#0x10, RatMap, SADR — single-pass index."""
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
            if (hw & 0xFFF0) == 0xF890: extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}"); o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000: extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}"); o += 2
    return "\n".join(lines)

def fn_start(xo, max_back=0x1000):
    for p in range(xo, max(MAIN, xo - max_back), -2):
        h = u16(p)
        if (h & 0xFFF0) == 0xE92D or h in (0xB5F0, 0xB570, 0xB5B0, 0xB580, 0xB5C0):
            return p
    return None

print("Building MOVW index (single pass)...")
# lo16 -> list of (off, rd)
movw_idx = defaultdict(list)
for o in range(MAIN, END - 4, 2):
    r = movw(o)
    if r: movw_idx[r[0]].append((o, r[1]))
print(f"  movw keys={len(movw_idx)}")

def xrefs_va(v):
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    hits = []
    for o, rd in movw_idx.get(lo, ()):
        for p in range(max(MAIN, o - 24), min(END, o + 28), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == rd:
                hits.append(o); break
    return hits

def scan_fn(fn, span=0xC00):
    ints, p2, bf6 = [], [], []
    for p in range(fn, min(END, fn + span), 2):
        bt = bl_target(p)
        if bt in INTEREST: ints.append((p, bt))
        hw, hw2 = u16(p), u16(p + 2) if p + 2 < END else 0
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            bf6.append(p)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
            for b in range(2, 20, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
                    p2.append(p); break
        if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) == 0:
            for b in range(2, 12, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2 and ((h >> 8) & 7) == (hw & 7):
                    p2.append(p); break
    return ints, p2, bf6

# Known string VAs from prior RO (tmp-eu-ready-path.out) — also re-find by content
needles = {
    "HotSwapInsertTimer state": b"[HOTSWAP] HotSwapInsertTimer state",
    "HotSwapInsertTimer hotswapstate": b"[HOTSWAP] HotSwapInsertTimer hotswapstate",
    "HotSwapRemoveTimer": b"[HOTSWAP] HotSwapRemoveTimer state",
    "SIM insertion diag": b"SIM insertion resulted into new diag trigger",
    "SIM removal diag": b"SIM removal resulted into new diag trigger",
    "SIM_PRESENT_IND CARD_PRESENT": b"Sending SIM_PRESENT_IND to PBM from USIM_CARD_PRESENT",
    "SIM_PRESENT_IND WAIT_INIT": b"Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ",
    "USIM_CARD_PRESENT enum": b"USIM_CARD_PRESENT",
    "USIM_CARD_ABSENT enum": b"USIM_CARD_ABSENT",
    "sitTxSimSetCardPower": b"sitTxSimSetCardPower",
    "DS_TCS_GV_FCN_SIM_HOT_SWAP": b"DS_TCS_GV_FCN_SIM_HOT_SWAP",
    "SADR_MEASURE_RSP": b"MMCIF_L1LC_DSL1C_SADR_MEASURE_RSP",
    "No CDMA SupportedRatMap": b"No CDMA in SupportedRatMap",
    "No CDMA InitRapMap": b"No CDMA in InitRapMap",
    "SupportedRatMap hex": b"SupportedRatMap(0x%X)",
}

print("\n=== HotSwap / USIM INSERT xrefs ===")
for name, needle in needles.items():
    so = img.find(needle)
    if so < 0:
        print(f"\n## {name}: NOT FOUND"); continue
    v = va(so)
    xr = xrefs_va(v)
    print(f"\n## {name}")
    print(f"  str@{hex(so)} VA={hex(v)} xrefs n={len(xr)} {list(map(hex, xr[:10]))}")
    for xo in xr[:4]:
        fn = fn_start(xo)
        print(f"  xref@{hex(xo)} fn~{hex(fn) if fn else None}")
        if not fn: continue
        ints, p2, bf6 = scan_fn(fn)
        print(f"    BLs: {[(hex(a), hex(b)) for a,b in ints[:16]]}")
        print(f"    STRB#0=2: {list(map(hex, p2[:8]))}  STRB#BF6: {list(map(hex, bf6[:4]))}")
        # sample dump around xref
        print(dump(xo - 0x10, xo + 0x40))

print("\n=== SET#1 callers (BL->0x146a99c) — dual pass narrow? use movw unused; scan BL once ===")
# Single-pass BL to SET1
set1_cs = []
helper_cs = []
getobj10 = []
for o in range(MAIN, END - 4, 2):
    bt = bl_target(o)
    if bt == SET1: set1_cs.append(o)
    if bt == HELPER: helper_cs.append(o)
    if bt == GETOBJ:
        # lookback MOVS r1,#0x10
        for b in range(2, 32, 2):
            if u16(o - b) == 0x2110:
                getobj10.append(o); break
print(f"SET#1 callers: {list(map(hex, set1_cs))}")
for c in set1_cs:
    fn = fn_start(c)
    print(f"\n-- SET#1 caller {hex(c)} fn~{hex(fn) if fn else None}")
    print(dump(c - 0x30, c + 0x20))
    if fn:
        ints, p2, bf6 = scan_fn(fn, 0x800)
        print(f"  BLs: {[(hex(a), hex(b)) for a,b in ints[:20]]}")
        print(f"  STRB#0=2: {list(map(hex, p2))} BF6: {list(map(hex, bf6))}")

print(f"\nHelper 0x1991838 callers n={len(helper_cs)}: {list(map(hex, helper_cs[:20]))}")
print(f"getobj(#0x10) sites n={len(getobj10)}: {list(map(hex, getobj10[:20]))}")

print("\n=== getobj#0x10 + STRB#2 + helper pattern detail ===")
for s in getobj10:
    # after getobj: MOVS#2 STRB [r1,#0] BL helper?
    has2 = False
    for p in range(s, min(END, s + 0x40), 2):
        h = u16(p)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
            has2 = True
        if bl_target(p) == HELPER and has2:
            print(f"\nSITE {hex(s)} (SET#1-side pattern)")
            print(dump(s - 0x28, p + 8))
            # nearby path/file string
            for q in range(s - 0x40, s + 0x20, 2):
                r = movw(q)
                if not r: continue
                for tpos in range(q, q + 20, 2):
                    t = movt(tpos)
                    if t and t[1] == r[1]:
                        ss = cstr((t[0] << 16) | r[0])
                        if ss and ("/" in ss or "L1" in ss or "SIM" in ss or "Msg" in ss):
                            print(f"  str: {ss[:90]}")
            break

print("\n=== Helper body prologue (arg meaning) ===")
print(dump(HELPER, HELPER + 0x60))
# SET_APP is nearby at 0x19916d2 — confirm helper != SET_APP
print(f"SET_APP@{hex(SET_APP)} HELPER@{hex(HELPER)} delta={HELPER-SET_APP}")

print("\n=== RatMap: No-CDMA log sites + preceding LDR of map value ===")
for name in ("No CDMA SupportedRatMap", "No CDMA InitRapMap", "SupportedRatMap hex"):
    so = img.find(needles[name])
    if so < 0: continue
    xr = xrefs_va(va(so))
    print(f"\n{name} xrefs={list(map(hex, xr[:6]))}")
    for xo in xr[:2]:
        print(dump(xo - 0x50, xo + 0x30))

# Check if image embeds a nonzero CDMA capability constant near TCS_CDMA_SUPPORT dump format
so = img.find(b"[[TCS_CDMA_SUPPORT]]")
print(f"\nTCS_CDMA_SUPPORT str@{hex(so) if so>=0 else -1}")
if so >= 0:
    xr = xrefs_va(va(so))
    print(f"  xrefs n={len(xr)} {list(map(hex, xr[:8]))}")
    for xo in xr[:2]:
        print(dump(xo - 0x40, xo + 0x50))

print("\n=== SADR: any Build*/sitTx* measure SIT? ===")
for n in [b"BuildSadr", b"sitTxSadr", b"SADR_MEASURE_REQ", b"DSL1C_SADR_MEASURE",
          b"sitTxL1", b"BuildLteMeasure", b"MEASURE_RSP"]:
    i = 0; hits = []
    while len(hits) < 5:
        j = img.find(n, i)
        if j < 0: break
        hits.append(j); i = j + 1
    if hits:
        print(f"  {n!r}: {[hex(h) for h in hits]} -> {[cstr(va(h))[:60] if False else img[h:h+40] for h in hits[:2]]}")

# STATUS_WRAP only 2 callers — already known; confirm no third via this pass
sw = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == STATUS_WRAP]
print(f"\nSTATUS_WRAP callers n={len(sw)}: {list(map(hex, sw))}")

print("\nDONE")
