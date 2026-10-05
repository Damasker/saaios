#!/usr/bin/env python3
"""Stock EU Present/READY path hunt beyond FN_A.

Focus:
- all +0xBF6 writers (any encoding)
- all PresentObj(#636c) [0] writers (any encoding)
- STATUS callers / r5 provenance (is PresentObj the only arg?)
- SET#1 insert callers vs CardPower / ABSENT paths
- pin1=VERIFIED(2) / Pin1Verified branches toward SET_APP#5
- USIM insert/ATR/hotplug strings near Present/SET_APP
"""
from __future__ import annotations

import struct
from pathlib import Path

PATH = Path(__file__).resolve().parent / "fw" / "saaios-probe-b-modem.bin"
MAIN = 0x16C10
END = MAIN + 0x05917ACC
VA0 = 0x40010000
img = PATH.read_bytes()

STATUS = 0x14FB322
SET_APP = 0x19916D2
GET_APP = 0x18EC8C0
FN_A = 0x14F692C
GETOBJ = 0x20EA040
BF6_STORE = 0x14FB380
READY_SET = 0x14FB5C6
SET1 = 0x146A99C


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def va(o: int) -> int:
    return VA0 + (o - MAIN)


def off_va(v: int) -> int:
    return MAIN + (v - VA0)


def movw(o: int):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o: int):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def bl_target(o: int):
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


def cstr(v: int, lim: int = 100) -> str | None:
    o = off_va(v)
    if o < 0 or o >= len(img):
        return None
    b = o
    while b < o + lim and 32 <= img[b] < 127:
        b += 1
    if b == o:
        return None
    return img[o:b].decode("ascii", "replace")


def find_cstr(needle: bytes) -> list[int]:
    out = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        out.append(j)
        i = j + 1
    return out


def dump(start: int, end: int) -> str:
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
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
            if (hw & 0xFFF0) == 0xF8A0:
                extra = f" ;STRH.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            elif (hw & 0xF800) == 0x4800:
                extra = f" ;LDR r{(hw>>8)&7},[PC,#{(hw&0xff)*4}]"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


print("=== 1) All +0xBF6 stores (STRB.W / STRH.W / Thumb16) ===")
bf6_stores = []
for o in range(MAIN, END - 4, 2):
    hw, hw2 = u16(o), u16(o + 2)
    # STRB.W Rt,[Rn,#imm12] imm==0xBF6
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
        bf6_stores.append((o, f"STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#0xBF6]"))
    # STRH.W covering BF6 (imm 0xBF6 or 0xBF4/BF5 half)
    if (hw & 0xFFF0) == 0xF8A0 and (hw2 & 0xFFF) in (0xBF4, 0xBF5, 0xBF6):
        bf6_stores.append((o, f"STRH.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"))
    # STR.W word covering
    if (hw & 0xFFF0) == 0xF8C0 and (hw2 & 0xFFF) in (0xBF4, 0xBF0):
        bf6_stores.append((o, f"STR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"))
print(f"count={len(bf6_stores)}")
for o, s in bf6_stores[:40]:
    print(f"  {hex(o)}: {s}")

print("\n=== 2) All BL->STATUS / BL->SET_APP#5 site context ===")
status_callers = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == STATUS]
print(f"BL STATUS count={len(status_callers)}: {list(map(hex, status_callers[:30]))}")
ready_callers = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == READY_SET]
# READY_SET is not a function entry; find BLs to SET_APP with nearby MOVS#5
set_app_bls = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == SET_APP]
print(f"BL SET_APP count={len(set_app_bls)}")
for o in set_app_bls:
    imm = None
    for b in range(2, 24, 2):
        h = u16(o - b)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) <= 7:
            imm = h & 0xFF
            break
    print(f"  {hex(o)} imm~{imm}")

print("\n=== 3) STATUS arg provenance near sole caller wrap ===")
# Prior: STATUS_WRAP 0x14c6626
print(dump(0x14C65F0, 0x14C66B0))
print("\n--- STATUS entry + Present copy ---")
print(dump(0x14FB322, 0x14FB3B0))
print("\n--- STATUS READY gate ---")
print(dump(0x14FB580, 0x14FB5E0))

print("\n=== 4) getobj(#0x636c) sites + nearby STRB [*,#0] vals ===")
sites_636c = []
for o in range(MAIN, END - 8, 2):
    r = movw(o)
    if not r or r[0] != 0x636C:
        continue
    # look ahead for BL getobj within 32B
    for p in range(o, min(END - 4, o + 0x30), 2):
        if bl_target(p) == GETOBJ:
            sites_636c.append((o, p))
            break
print(f"getobj#636c sites={len(sites_636c)}")
for mov, bl in sites_636c:
    print(f"\n-- MOVW@ {hex(mov)} BL@{hex(bl)} --")
    print(dump(mov - 0x10, bl + 0x80))

print("\n=== 5) PresentObj[0] Thumb16 STRB rX,[rY,#0] with MOVS #2 nearby (SIM+L1 bands) ===")
bands = [(0x14F0000, 0x1508000), (0x14C0000, 0x14D0000), (0x1910000, 0x1950000),
         (0x1450000, 0x1470000), (0x1A20000, 0x1A50000), (0x1F00000, 0x1F20000)]
hits = []
for lo, hi in bands:
    for o in range(lo, hi - 2, 2):
        hw = u16(o)
        # Thumb16 STRB Rt,[Rn,#0] encoding 0x7000 | Rt | (Rn<<3)
        if (hw & 0xF800) != 0x7000 or ((hw >> 6) & 0x1F) != 0:
            continue
        rt, rn = hw & 7, (hw >> 3) & 7
        # look back 40B for MOVS rt,#2
        for b in range(2, 48, 2):
            h = u16(o - b)
            if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == rt and (h & 0xFF) == 2:
                hits.append((o, rn, o - b))
                break
print(f"Thumb16 STRB#0 MOVS#2 hits={len(hits)}")
for o, rn, mov in hits[:40]:
    print(f"  STRB@{hex(o)} rn=r{rn} MOVS@{hex(mov)}")
    # if near getobj 636c or FN_A
    near = ""
    for p in range(max(MAIN, o - 0x100), min(END, o + 0x40), 2):
        if bl_target(p) in (FN_A, GETOBJ, SET_APP, STATUS):
            near = f" nearBL {hex(p)}->{hex(bl_target(p))}"
            break
        r = movw(p)
        if r and r[0] == 0x636C:
            near = f" near#636c {hex(p)}"
            break
    if near:
        print(f"    {near}")

print("\n=== 6) SET#1 callers deep: insert vs absent ===")
set1_callers = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == SET1]
print(f"BL SET1FN={list(map(hex, set1_callers))}")
for c in set1_callers:
    print(f"\n-- caller {hex(c)} --")
    print(dump(c - 0x60, c + 0x20))
    # walk up function prolog
    fn = None
    for p in range(c, max(MAIN, c - 0x400), -2):
        h = u16(p)
        if h in (0xB570, 0xB5F0, 0xB5B0, 0xB580) or (h & 0xFFF0) == 0xE92D:
            fn = p
            break
    print(f"  fn~{hex(fn) if fn else None}")
    if fn:
        parents = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == fn]
        print(f"  parents={list(map(hex, parents[:12]))} n={len(parents)}")

print("\n=== 7) Strings: insert/hotplug/ATR/card detect near Present ===")
for needle in (
    b"SIM INSERT",
    b"CARD_INSERT",
    b"Card Insert",
    b"HOT_SWAP",
    b"hotplug",
    b"HotSwap",
    b"SIM_ABSENT",
    b"SIM_PRESENT",
    b"UICC_RESET",
    b"Cold Reset",
    b"cold reset",
    b"ATR received",
    b"ATR_",
    b"CardDetect",
    b"CARD_DETECT",
    b"tray",
    b"Tray",
    b"SIM STATUS update",
    b"Present/Pin1Verified",
    b"SET_APP",
    b"DETECTED",
    b"USIM_CARD_PRESENT",
    b"USIM_WAIT_FOR_INIT",
    b"SIM_INIT_REQ",
    b"START_STACK_SERVICES",
):
    hits = find_cstr(needle)
    if not hits:
        continue
    for j in hits[:3]:
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        print(f"  @{hex(a)} VA={hex(va(a))}: {img[a:b].decode('ascii','replace')[:120]}")

print("\n=== 8) Pin1Verified / pin1=2 path toward READY? ===")
# sites that STRB Pin1Verified (obj+20) and whether they BL SET_APP or STATUS after
# Known FirstPIN sites ~0x1f0456c / 0x1f0469a from docs
for start in (0x1F04500, 0x1F04650, 0x18E0000, 0x14FB3C0):
    print(f"\n-- window {hex(start)} --")
    print(dump(start, start + 0x120))

print("\n=== 9) Challenge: any non-FN_A path stores imm 2 then copies into +0xBF6 via STATUS? ===")
# Who BL STATUS_WRAP / STATUS with PresentObj freshly written?
# STATUS_WRAP = 0x14c6626 area
wrap_callers = []
# find function containing 0x14c6626
wrap_fn = None
for p in range(0x14C6626, 0x14C6626 - 0x200, -2):
    h = u16(p)
    if h in (0xB570, 0xB5F0, 0xB5B0) or (h & 0xFFF0) == 0xE92D:
        wrap_fn = p
        break
print(f"STATUS_WRAP fn={hex(wrap_fn) if wrap_fn else None}")
if wrap_fn:
    wrap_callers = [o for o in range(MAIN, END - 4, 2) if bl_target(o) == wrap_fn]
    print(f"callers n={len(wrap_callers)}: {list(map(hex, wrap_callers[:20]))}")
    for c in wrap_callers[:10]:
        names = []
        for p in range(c - 0x60, c + 8, 2):
            r = movw(p)
            if not r:
                continue
            for q in range(max(MAIN, p - 16), min(END, p + 20), 2):
                t = movt(q)
                if t and t[1] == r[1] and 0x4100 <= t[0] <= 0x4120:
                    s = cstr((t[0] << 16) | r[0])
                    if s:
                        names.append(s[:80])
        print(f"  {hex(c)} names={names[:4]}")

print("\n=== 10) CardPower / 0x024c handler vs ABSENT path ===")
for needle in (b"CardPower", b"CARD_POWER", b"SimCardPower", b"0x024c", b"SIM_CARD_POWER"):
    for j in find_cstr(needle)[:5]:
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        print(f"  @{hex(a)}: {img[a:b].decode('ascii','replace')[:100]}")

# SIT id 0x024c = 588 as MOVW?
print("\nMOVW #0x24c sites (CardPower id):")
n = 0
for o in range(MAIN, END - 4, 2):
    r = movw(o)
    if r and r[0] == 0x24C:
        print(f"  {hex(o)} r{r[1]}")
        n += 1
        if n >= 20:
            break

print("\nDONE")
