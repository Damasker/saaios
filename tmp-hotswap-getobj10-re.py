#!/usr/bin/env python3
"""RO: HotSwap INSERT chain, getobj#0x10 role, RatMap CDMA bits, SADR producers."""
from __future__ import annotations

import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
if not PATH.exists():
    PATH = Path(
        r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin"
    )
MAIN = 0x16C10
END = MAIN + 0x05917ACC
VA0 = 0x40010000
img = PATH.read_bytes()

SET_APP = 0x19916D2
STATUS = 0x14FB322
FN_A = 0x14F692C
GET_APP = 0x18EC8C0
STATUS_WRAP = 0x14C6626
SET1 = 0x146A99C
HELPER = 0x1991838
GETOBJ = 0x20EA040


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA0 + (o - MAIN)


def off_va(v):
    return MAIN + (v - VA0)


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


def cstr(o, lim=100):
    if o < 0 or o >= len(img):
        return None
    b = o
    while b < o + lim and 32 <= img[b] < 127:
        b += 1
    if b == o:
        return None
    return img[o:b].decode("ascii", "replace")


def dump(start, end):
    o = start
    lines = []
    while o < end and o + 2 <= len(img):
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) and o + 4 <= len(img):
            hw2 = u16(o + 2)
            extra = ""
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                extra = f" ;BL->{hex(bt)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xFF00) == 0x2100:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


def find_str(needle: bytes):
    hits = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        hits.append(j)
        i = j + 1
    return hits


def xrefs_to_off(soff: int, band=None):
    """MOVW/MOVT pairs loading VA of string at file offset soff."""
    v = va(soff)
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    lo_s, hi_s = (band or (MAIN, END))
    hits = []
    for o in range(lo_s, hi_s - 8, 2):
        r = movw(o)
        if not r or r[0] != lo:
            continue
        for p in range(max(lo_s, o - 24), min(hi_s, o + 28), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == r[1]:
                hits.append(o)
                break
    return hits


def fn_start(xo, max_back=0xC00):
    for p in range(xo, max(MAIN, xo - max_back), -2):
        h = u16(p)
        if (h & 0xFFF0) == 0xE92D or h in (0xB5F0, 0xB570, 0xB5B0, 0xB580, 0xB5C0):
            return p
    return None


def scan_interesting(fn, span=0x800):
    interesting = []
    present2 = []
    for p in range(fn, min(END, fn + span), 2):
        bt = bl_target(p)
        if bt in (SET_APP, STATUS, FN_A, STATUS_WRAP, GET_APP, SET1, HELPER, GETOBJ):
            interesting.append((p, bt))
        hw, hw2 = u16(p), u16(p + 2) if p + 2 < END else 0
        # STRB.W [rn,#0] after MOVS #2; or STRB [rn,#0]
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
            for b in range(2, 20, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
                    present2.append(p)
                    break
        if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) == 0:
            for b in range(2, 12, 2):
                h = u16(p - b)
                if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2 and (h >> 8) & 7 == (hw & 7):
                    present2.append(p)
                    break
        # STRB.W #0xBF6
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            interesting.append((p, 0xBF6))
    return interesting, present2


print("=== 1) HotSwap / INSERT string inventory + code xrefs ===")
needles = [
    b"[HOTSWAP] HotSwapInsertTimer state",
    b"HotSwapInsertTimer state",
    b"HotSwapInsertTimer",
    b"SIM_HOT_SWAP",
    b"DS_TCS_GV_FCN_SIM_HOT_SWAP",
    b"SIM insertion resulted into new diag trigger",
    b"SIM removal resulted into new diag trigger",
    b"Sending SIM_PRESENT_IND to PBM from USIM_CARD_PRESENT",
    b"Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ",
    b"USIM_CARD_PRESENT",
    b"USIM_CARD_ABSENT",
    b"USIM_WAIT_FOR_INIT_REQ",
    b"Card Detected",
    b"CARD_INSERTED",
    b"SIM_PRESENT_IND",
    b"sitTxSimSetCardPower",
    b"HotSwapRemove",
    b"hot_swap",
    b"HOT_SWAP",
]
for n in needles:
    offs = find_str(n)
    if not offs:
        continue
    print(f"\n## {n.decode('ascii','replace')[:70]}")
    print(f"  str offs={[hex(o) for o in offs[:4]]} VA={[hex(va(o)) for o in offs[:4]]}")
    for so in offs[:2]:
        xr = xrefs_to_off(so)
        print(f"  xrefs n={len(xr)} first={list(map(hex, xr[:8]))}")
        for xo in xr[:3]:
            fn = fn_start(xo)
            print(f"  -- xref@{hex(xo)} fn~{hex(fn) if fn else None}")
            if fn:
                ints, p2 = scan_interesting(fn, 0xA00)
                print(f"     BLs/stores: {[(hex(a), hex(b) if isinstance(b,int) and b>0xff else b) for a,b in ints[:20]]}")
                print(f"     Presentish STRB#0=2: {list(map(hex, p2[:10]))}")
                # nearby strings via MOVW/MOVT in fn
                names = []
                for p in range(fn, min(END, fn + 0x200), 2):
                    r = movw(p)
                    if not r:
                        continue
                    for q in range(p, min(END, p + 24), 2):
                        t = movt(q)
                        if not t or t[1] != r[1]:
                            continue
                        s = cstr(off_va((t[0] << 16) | r[0]))
                        if s and len(s) > 6 and s not in names:
                            names.append(s[:80])
                if names:
                    print(f"     strings: {names[:8]}")

print("\n=== 2) SET#1 callers + getobj#0x10 role ===")
cs = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == SET1]
print(f"SET#1 callers n={len(cs)}: {list(map(hex, cs))}")
for c in cs:
    fn = fn_start(c)
    print(f"\n-- caller {hex(c)} in fn~{hex(fn) if fn else None}")
    print(dump(c - 0x20, c + 0x30))
    if fn:
        ints, p2 = scan_interesting(fn, 0x600)
        print(f"  interesting: {[(hex(a), hex(b) if isinstance(b,int) and b>0xff else b) for a,b in ints[:15]]}")

# getobj #0x10: r1=#0x10 before BL getobj near SET#1
print("\n=== getobj(#0x10) sites (MOVW/MOVS r1,#0x10 near BL getobj) ===")
sites = []
for o in range(MAIN, END - 8, 2):
    if bl_target(o) != GETOBJ:
        continue
    # lookback for r1 = 0x10 (MOVS r1,#16 = 0x2110) or MOVW
    got = False
    for b in range(2, 40, 2):
        h = u16(o - b)
        if h == 0x2110:  # MOVS r1,#0x10
            got = True
            break
        r = movw(o - b)
        if r and r[1] == 1 and r[0] == 0x10:
            got = True
            break
    if got:
        sites.append(o)
print(f"n={len(sites)}: {list(map(hex, sites[:30]))}")
for s in sites[:12]:
    # dump + nearby string
    print(f"\n-- getobj#0x10 @{hex(s)} --")
    print(dump(s - 0x30, s + 0x28))
    names = []
    for p in range(s - 0x60, s + 0x40, 2):
        r = movw(p)
        if not r:
            continue
        for q in range(p, min(END, p + 20), 2):
            t = movt(q)
            if t and t[1] == r[1]:
                ss = cstr(off_va((t[0] << 16) | r[0]))
                if ss and len(ss) > 5:
                    names.append(ss[:90])
    if names:
        print(f"  strings: {names[:5]}")
    # does this site STRB #2 and BL helper?
    for p in range(s, s + 0x30, 2):
        bt = bl_target(p)
        if bt == HELPER:
            print(f"  BL helper @{hex(p)}")
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            print(f"  MOVS#2 @{hex(p)}")

print("\n=== Helper 0x1991838: what does arg0 mean? ===")
print(dump(HELPER, HELPER + 0xA0))
# callers that pass MOVS #2 immediately before
print("\nCallers with MOVS #2 just before BL helper:")
for o in range(MAIN, END - 4, 2):
    if bl_target(o) != HELPER:
        continue
    for b in (2, 4, 6, 8):
        h = u16(o - b)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) == 2:
            print(f"  {hex(o)} (MOVS#2 @{hex(o-b)})")
            print(dump(o - 0x20, o + 8))
            break

print("\n=== 3) RatMap / CDMA bit evidence in image (RO strings + log formats) ===")
for n in [
    b"No CDMA in SupportedRatMap",
    b"No CDMA in InitRapMap",
    b"SupportedRatMap(0x%X)",
    b"SupportedRatMap(%d)",
    b"InitRapMap",
    b"TCS_CDMA_SUPPORT",
    b"DS_TCS_GV_CDMA_SUPPORT",
]:
    offs = find_str(n)
    print(f"{n!r}: offs={[hex(o) for o in offs[:3]]}")

# Search for literal RatMap masks with CDMA bit comments nearby
# Common CDMA RAT bits in Shannon: bit for CDMA2000 / 1x / EVDO often in multi-RAT masks
print("\nNearby code logging SupportedRatMap:")
for so in find_str(b"SupportedRatMap(0x%X)")[:2]:
    xr = xrefs_to_off(so)
    print(f"  fmt@{hex(so)} xrefs={list(map(hex, xr[:6]))}")
    for xo in xr[:2]:
        print(dump(xo - 0x40, xo + 0x60))

print("\n=== 4) SADR_MEASURE_RSP producers / signed AP path? ===")
for n in [
    b"MMCIF_L1LC_DSL1C_SADR_MEASURE_RSP",
    b"SADR_MEASURE",
    b"SADR_GAP_MEASURE",
    b"DSL1C_SADR",
]:
    offs = find_str(n)
    print(f"{n!r}: n={len(offs)} first={[hex(o) for o in offs[:4]]}")
    for so in offs[:1]:
        xr = xrefs_to_off(so)
        print(f"  xrefs n={len(xr)} {list(map(hex, xr[:8]))}")

# Any SIT Build* mentioning measure / sadr?
for n in [
    b"BuildSadr",
    b"sitTxSadr",
    b"SIT_SADR",
    b"MEASURE_REQ",
    b"BuildMeasure",
    b"sitTxLteMeasure",
]:
    offs = find_str(n)
    if offs:
        print(f"SIT-ish {n!r}: {[hex(o) for o in offs[:4]]}")

print("\nDONE")
