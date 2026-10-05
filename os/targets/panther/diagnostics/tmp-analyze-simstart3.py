#!/usr/bin/env python3
"""Deep: PrepareSimStartIndParameters, SIT Rx START/PIN_STATUS, Present writes."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A = 0x14F692C
FN_B = 0x14F9108
WRAP_SIM = 0x14F6D02
SET_APP = 0x19916D2


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


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


def real_sites(log_id):
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != log_id:
            continue
        dense = False
        for p in range(o + 4, o + 28, 2):
            r2 = movw(p)
            if r2 and abs(r2[0] - log_id) <= 2:
                dense = True
                break
        if dense:
            continue
        ctx = None
        for p in range(o - 24, o + 28, 2):
            t = movt(p)
            if t and 0x4000 <= t[0] <= 0x45FF:
                ctx = t[0]
                break
        if ctx is None:
            continue
        hits.append((o, ctx))
    return hits


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = ""
                if r[0] in (0x105A, 0x7AB, 0x18E, 0x106A, 0x1068, 0xC6B, 0x347, 0x2C5E):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                t = bl_target(o)
                tag = ""
                if t == FN_A:
                    tag = " <<FN_A"
                elif t == FN_B:
                    tag = " <<FN_B"
                elif t == WRAP_SIM:
                    tag = " <<WRAP_SIM"
                elif t == SET_APP:
                    tag = " <<SET_APP"
                extra = f" ;BL->{t:#x}{tag}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


def find_str(n):
    out = []
    idx = 0
    while True:
        j = img.find(n, idx)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        out.append((a, img[a:b].decode(errors="replace")))
        idx = j + 1
    return out


# 1) PrepareSimStartIndParameters log
print("=== PrepareSimStartIndParameters ===")
for off, s in find_str(b"PrepareSimStartIndParameters"):
    print(f"  @{off:#x}: {s[:100]}")
    # find log id
    for back in range(0, 16):
        w = u32(off - back)
        if (w & 0xFF) == 0x44:
            lid = (w >> 8) & 0xFFFF
            print(f"    lid={lid:#x}")
            sites = real_sites(lid)
            print(f"    sites={[(hex(o), hex(c)) for o,c in sites[:8]]}")
            for o, ctx in sites[:4]:
                print(f"\n    dump around {o:#x}:")
                print(dump(o - 0x40, o + 0x80))
            break

# 2) Dump USIM 0x105a sites ctx=0x4107 (most likely real)
print("\n=== USIM-ctx 0x105a dumps (0x1916xxx) ===")
for site in (0x191687A, 0x1916E70, 0x19170E6, 0x191724E):
    print(f"\n---- {site:#x} ----")
    print(dump(site - 0x60, site + 0xA0))
    # scan ±0x200 for Present=2 / FN_A / STRB #0 BF6 / MOVS#2 STRB
    hits = []
    for o in range(site - 0x200, site + 0x300, 2):
        b = bl_target(o)
        if b in (FN_A, FN_B, WRAP_SIM, SET_APP):
            hits.append(f"BL {hex(b)} @{hex(o)}")
        hw = u16(o)
        if (hw & 0xFFF0) == 0xF880 and (u16(o + 2) & 0xFFF) in (0xBF6, 0xBF5, 0, 1):
            hits.append(f"STRB.W #{u16(o+2)&0xfff:#x} @{hex(o)}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for q in range(o + 2, o + 10, 2):
                h = u16(q)
                if (h & 0xFFF0) == 0xF880 and (u16(q + 2) & 0xFFF) == 0:
                    hits.append(f"Present=2 @{hex(o)}")
                if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 0:
                    hits.append(f"MOVS#2 STRB#0 @{hex(o)}")
    print(f"  hits: {hits}")

# 3) SIT Rx SIM START / PIN STATUS
print("\n=== SIT Rx START / PIN STATUS strings ===")
for n in [
    b"Rx NS_SIM_START",
    b"Rx SIM_START",
    b"NS_SIM_START_IND",
    b"SIM_START_IND",
    b"Rx NS_USIM_START",
    b"USIM_START_IND",
    b"Rx.*PIN_STATUS",
    b"NS_SIM_PIN_STATUS",
    b"NS_USIM_PIN_STATUS",
    b"PIN_STATUS_IND",
    b"sitRxSimStart",
    b"SimStartInd",
    b"HandleSimStart",
    b"SimPinStatus",
]:
    if b".*" in n:
        continue
    for off, s in find_str(n)[:5]:
        print(f"  @{off:#x}: {s[:130]}")

# 4) Message name table entries USIM ==> SIM_START_IND / PIN_STATUS — find switch using those VAs
print("\n=== Xref message name VAs (MOVW+MOVT) ===")
for label, soff in [
    ("SIM_START_IND", 0x1035B4B),
    ("SIM_PIN_STATUS_IND", 0x1035939),
    ("SIM_ERROR_STATUS_IND", 0x1035955),  # approx next string
]:
    # string may start at soff; VA
    sva = VA_BASE + (soff - MAIN_OFF)
    lo = sva & 0xFFFF
    hi = sva >> 16
    print(f"  {label}: file@{soff:#x} VA={sva:#x} lo={lo:#x} hi={hi:#x}")
    xrefs = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != lo:
            continue
        # nearby MOVT same reg with hi
        for p in range(o - 8, o + 12, 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == r[1]:
                xrefs.append(o)
                break
    print(f"    xrefs ({len(xrefs)}): {[hex(x) for x in xrefs[:12]]}")

# 5) Search sitSetPin1 near SIM START path — CMP DISABLED #3 then SET ready?
print("\n=== In 0x1916000..0x1918000: MOVS#2/3/5 + SET_APP / Present ===")
for o in range(0x1916000, 0x1918000, 2):
    b = bl_target(o)
    if b == SET_APP:
        imm = None
        for back in range(2, 16, 2):
            hw = u16(o - back)
            if (hw & 0xFF00) == 0x2000:
                imm = hw & 0xFF
                break
        print(f"  SET_APP @{o:#x} imm={imm}")
    if b in (FN_A, FN_B, WRAP_SIM):
        print(f"  BL target @{o:#x}->{b:#x}")
    hw = u16(o)
    if (hw & 0xFFF0) == 0xF880 and (u16(o + 2) & 0xFFF) == 0xBF6:
        print(f"  STRB +0xBF6 @{o:#x}")
