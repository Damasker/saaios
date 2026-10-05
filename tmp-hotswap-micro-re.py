#!/usr/bin/env python3
"""Micro RE: HotSwap string-start xrefs + PresentObj stores in USIM band + RatMap ADR."""
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
INTEREST = {SET_APP, STATUS, FN_A, STATUS_WRAP, GET_APP, SET1, HELPER, GETOBJ, 0x14F6A16}

def u16(o): return struct.unpack_from("<H", img, o)[0]
def u32(o): return struct.unpack_from("<I", img, o)[0]
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

def cstr(v, lim=120):
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

print("index MOVW...")
movw_idx = defaultdict(list)
for o in range(MAIN, END - 4, 2):
    r = movw(o)
    if r: movw_idx[r[0]].append((o, r[1]))

def xrefs_va(v):
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    hits = []
    for o, rd in movw_idx.get(lo, ()):
        for p in range(max(MAIN, o - 24), min(END, o + 28), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == rd:
                hits.append(o); break
    return hits

def scan_fn(fn, span=0x1000):
    ints, p2, bf6, po = [], [], [], []
    for p in range(fn, min(END, fn + span), 2):
        bt = bl_target(p)
        if bt in INTEREST: ints.append((p, bt))
        hw, hw2 = u16(p), u16(p + 2) if p + 2 < END else 0
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6: bf6.append(p)
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
        r = movw(p)
        if r and r[0] == 0x636C: po.append(p)
        if r and r[0] == 0x7B7C:
            for q in range(p, p + 16, 2):
                t = movt(q)
                if t and t[0] == 0x48B8 and t[1] == r[1]:
                    po.append(p)
    return ints, p2, bf6, po

# Use FULL format string starts (prefix A[USIM_%d] / A[SIM_HOT_SWAP])
needles = [
    b"A[USIM_%d] [HOTSWAP] HotSwapInsertTimer state:%s",
    b"A[USIM_%d] [HOTSWAP] HotSwapInsertTimer hotswapstate:%s usimstate:%s",
    b"A[USIM_%d] [HOTSWAP] HotSwapRemoveTimer state:%s",
    b"A[SIM_HOT_SWAP] SIM insertion resulted into new diag trigger",
    b"A[SIM_HOT_SWAP] SIM removal resulted into new diag trigger",
    b"A[USIM_%d] Sending SIM_PRESENT_IND to PBM from USIM_CARD_PRESENT case",
    b"A[USIM_%d] Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ case",
    b"A[USIM_%d] Activation is OK.USIM state:USIM_CARD_ABSENT.Change it to USIM_CARD_ABSENT->USIM_WAIT_FOR_INIT_REQ",
    b"A[USIM_%d] Cold Reset Failed, move to CARD_ABSENT",
    b"A[SIT_1_SIM] sitTxSimSetCardPower",
    b"A[SIT_0_SIM] sitTxSimSetCardPower",
    b"ASIM STATUS update: Present:%d, Pin1Verified: %d, MePerVerified:%d",
    b"@QM_MM_INIT_REQ_Handler: No CDMA in InitRapMap!",
    b"@QM_MM_STOP_REQ_Handler: No CDMA in SupportedRatMap",
    b"@QM_MM_INIT_REQ_Handler: SupportedRatMap(0x%X)",
]

print("\n=== STRING-START xrefs ===")
for n in needles:
    so = img.find(n)
    if so < 0:
        # try without trailing punctuation variants
        print(f"MISS {n[:60]!r}")
        continue
    v = va(so)
    xr = xrefs_va(v)
    print(f"\n## {n[:70].decode('ascii','replace')}")
    print(f"  VA={hex(v)} xrefs n={len(xr)} {list(map(hex, xr[:12]))}")
    for xo in xr[:3]:
        fn = fn_start(xo)
        print(f"  xref@{hex(xo)} fn~{hex(fn) if fn else None}")
        if not fn: 
            print(dump(xo - 0x20, xo + 0x40)); continue
        ints, p2, bf6, po = scan_fn(fn)
        print(f"    BLs interest: {[(hex(a), hex(b)) for a,b in ints[:20]]}")
        print(f"    STRB#0=2: {list(map(hex, p2[:8]))}  BF6: {list(map(hex, bf6[:4]))}  PresentObj refs: {list(map(hex, po[:8]))}")
        print(dump(xo - 0x18, xo + 0x28))

# Also: ADR.W / literal pool pointers to HotSwapInsert VA
print("\n=== Literal u32 pointers to HotSwapInsertTimer VA ===")
hot_va = va(img.find(b"A[USIM_%d] [HOTSWAP] HotSwapInsertTimer state:%s"))
print(f"hot_va={hex(hot_va)}")
# search aligned words
hits = []
target = struct.pack("<I", hot_va)
i = MAIN
while True:
    j = img.find(target, i, END)
    if j < 0: break
    if j % 4 == 0: hits.append(j)
    i = j + 1
print(f"  pool hits n={len(hits)} {list(map(hex, hits[:20]))}")

# PresentObj getobj #636c in HotSwap/USIM file band — scan code near hotswap xrefs if any
print("\n=== All PresentObj #636c getobj sites (confirm no HotSwap writer) ===")
# MOVW #0x636c near BL getobj
sites = []
for o, rd in movw_idx.get(0x636C, ()):
    for p in range(o, min(END, o + 0x30), 2):
        if bl_target(p) == GETOBJ:
            sites.append((o, p)); break
print(f"getobj#636c n={len(sites)}: {[(hex(a), hex(b)) for a,b in sites]}")

# For each, check STRB #2 to [obj,#0] within 0x80
for so, go in sites:
    for p in range(go, min(END, go + 0x80), 2):
        h = u16(p)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
            for q in range(p + 2, p + 10, 2):
                hh = u16(q)
                if (hh & 0xF800) == 0x7000 and ((hh >> 6) & 0x1F) == 0:
                    print(f"  Present=2 candidate getobj@{hex(go)} STRB@{hex(q)}")
                    print(dump(go - 0x10, q + 0x10))

print("\n=== RatMap: ADR/pool to No-CDMA strings ===")
for label, needle in [
    ("NoCDMA Init", b"@QM_MM_INIT_REQ_Handler: No CDMA in InitRapMap!"),
    ("NoCDMA Supp", b"@QM_MM_STOP_REQ_Handler: No CDMA in SupportedRatMap"),
    ("Supp hex", b"@QM_MM_INIT_REQ_Handler: SupportedRatMap(0x%X)"),
]:
    so = img.find(needle)
    if so < 0:
        print(f"{label}: miss"); continue
    v = va(so)
    xr = xrefs_va(v)
    print(f"{label} VA={hex(v)} movw_xrefs={list(map(hex, xr[:8]))}")
    # pool
    target = struct.pack("<I", v)
    ph = []
    i = MAIN
    while len(ph) < 10:
        j = img.find(target, i, END)
        if j < 0: break
        if j % 4 == 0: ph.append(j)
        i = j + 1
    print(f"  pool={list(map(hex, ph[:8]))}")
    for xo in xr[:2]:
        print(dump(xo - 0x60, xo + 0x40))

print("\nDONE")
