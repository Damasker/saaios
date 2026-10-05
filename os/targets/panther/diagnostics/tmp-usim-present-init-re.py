#!/usr/bin/env python3
"""Hunt USIM SIM_INIT / PresentObj writers beyond FN_A (No-CDMA READY model)."""
from __future__ import annotations

import struct
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
FN_A = 0x14F692C
STATUS_WRAP = 0x14C6626
PRESENT_ID = 0x636C

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def bl(o):
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


def off_va(v):
    return MAIN + (v - VA)


def cstr(va, n=100):
    o = off_va(va)
    if o < 0 or o >= len(img):
        return None
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def scan_bl(target, start=0x1000000, end=0x3C00000):
    hits = []
    o = start
    while o < end - 4:
        if bl(o) == target:
            hits.append(o)
        o += 2
    return hits


def dump(start, end, lab):
    print(f"\n=== {lab} {hex(start)}..{hex(end)} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, b = movw(o), movt(o), bl(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                extra += f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra += f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


# 1) All MOVW #636c sites — any store to [obj+0] with imm or reg
print("=== ALL MOVW #0x636c sites + nearby stores ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append((o, r[1]))
    o += 2
print(f"n={len(sites)}")
for s, rd in sites:
    stores = []
    for p in range(s, min(s + 0xC0, len(img) - 4), 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xF800) == 0x7000:  # STRB T1
            stores.append(f"STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]@{hex(p)}")
        if (hw & 0xFFF0) == 0xF880:
            stores.append(
                f"STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]@{hex(p)}"
            )
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1, 2, 3):
            stores.append(f"MOVS r{(hw>>8)&7},#{hw&0xff}@{hex(p)}")
        b = bl(p)
        if b in (FN_A, STATUS_WRAP):
            stores.append(f"BL->{hex(b)}@{hex(p)}")
    print(f"  {hex(s)} r{rd}: {stores[:12]}")

# 2) FN_A callers
print("\n=== BL -> FN_A ===")
for c in scan_bl(FN_A):
    print(f"  {hex(c)}")
    dump(c - 0x30, c + 8, f"caller@{hex(c)}")

# 3) SIM_INIT_REQ / Waiting / START_STACK xref via string VA in MOVW/MOVT
needles = {
    "Waiting for SIM_INIT_REQ": img.find(b"Waiting for SIM_INIT_REQ"),
    "USIM <== SIM_INIT_REQ": img.find(b"USIM <== SIM_INIT_REQ"),
    "SIM_START_STACK_SERVICES": img.find(b"START_STACK_SERVICES"),
    "SIM_PRESENT_IND": img.find(b"SIM_PRESENT_IND"),
    "SIM_START_IND": img.find(b"USIM ==> SIM_START_IND"),
    "SIM_PIN_STATUS_IND": img.find(b"SIM_PIN_STATUS_IND"),
}
print("\n=== key USIM strings ===")
for k, off in needles.items():
    if off < 0:
        print(f"  {k}: MISSING")
        continue
    va = VA + (off - MAIN)
    print(f"  {k}: off={hex(off)} VA={hex(va)}")

# Find code refs: MOVW lo16 + MOVT hi16 of each VA
print("\n=== code refs to Waiting for SIM_INIT_REQ ===")
wait_off = needles["Waiting for SIM_INIT_REQ"]
if wait_off >= 0:
    wva = VA + (wait_off - MAIN)
    lo, hi = wva & 0xFFFF, (wva >> 16) & 0xFFFF
    refs = []
    o = 0x1000000
    while o < 0x3C00000 - 8:
        r = movw(o)
        if r and r[0] == lo:
            t = movt(o + 4)
            if t and t[0] == hi and t[1] == r[1]:
                refs.append(o)
        o += 2
    print(f"  refs={len(refs)} {[hex(x) for x in refs[:20]]}")
    for r in refs[:6]:
        dump(r - 0x40, r + 0x60, f"waitref@{hex(r)}")

# 4) Who BLs STATUS_WRAP
print("\n=== BL -> STATUS_WRAP ===")
for c in scan_bl(STATUS_WRAP):
    print(f"  {hex(c)}")
    # nearby strings
    chunk = img[max(0, c - 0x200) : c + 0x40]
    tags = []
    for s in (
        b"SADR",
        b"CDMA",
        b"L1TUNNEL",
        b"USIM",
        b"SIM",
        b"MEASURE",
        b"STATUS",
    ):
        if s in chunk:
            tags.append(s.decode())
    print(f"    tags_near={tags}")

# 5) Broad: MOVS #2; STRB [rN,#0] within 8 insn, with getobj nearby OR in USIM band
print("\n=== MOVS#2 + STRB[*,#0] candidates (USIM/SIM band 0x14f0000-0x1a00000) ===")
cands = []
o = 0x14F0000
while o < 0x1A00000 - 8:
    hw = u16(o)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        rt = (hw >> 8) & 7
        for p in range(o + 2, o + 16, 2):
            h2 = u16(p)
            if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                # look back 0x40 for 636c
                has = False
                for q in range(max(0x14F0000, o - 0x60), o, 2):
                    r = movw(q)
                    if r and r[0] == PRESENT_ID:
                        has = True
                        break
                cands.append((o, p, has))
                break
    o += 2
print(f"n={len(cands)} with_636c={sum(1 for c in cands if c[2])}")
for c in cands:
    if c[2] or (0x14F6900 <= c[0] <= 0x14F6B00):
        print(f"  MOVS2@{hex(c[0])} STRB@{hex(c[1])} has636c={c[2]}")

# 6) sit-stream: any Build* mentioning Init/Stack/Refresh near SIM
sit = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
if not sit.exists():
    sit = Path(
        "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/sit-stream.so"
    )
if sit.exists():
    s = sit.read_bytes()
    print("\n=== sit-stream SIM-ish Build* / Refresh strings ===")
    for needle in (
        b"SIM_INIT",
        b"StartStack",
        b"START_STACK",
        b"SimRefresh",
        b"BuildSimRefresh",
        b"BuildSimGetATR",
        b"BuildSimStatus",
        b"BuildSetUicc",
        b"BuildSimCardPower",
        b"BuildSimOpenChannel",
        b"FileUpdate",
        b"SimFile",
    ):
        print(f"  {needle!r}: {s.find(needle)>=0} off={hex(s.find(needle)) if s.find(needle)>=0 else None}")
else:
    print("sit-stream.so missing")

print("\nDONE")
