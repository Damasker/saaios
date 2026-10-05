#!/usr/bin/env python3
"""Trace 0x3708650 packer + Tx SIM Status log sites + PinSkip gate + RatMap CDMA."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


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
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0xB000 and (hw & 0xFF00) == 0xB500:
                extra = " ;PUSH"
            if hw == 0xB510 or hw == 0xB530 or hw == 0xB570 or hw == 0xB5F0 or hw == 0xB5B0:
                extra = " ;PUSH"
            if hw in (0xBD10, 0xBD30, 0xBD70, 0xBDF0, 0xBD00, 0x4770):
                extra = " ;RET"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Find function start before 0x3708648
fn = 0x3708600
while fn > 0x3708000:
    hw = u16(fn)
    if hw in (0xB510, 0xB530, 0xB570, 0xB5F0, 0xB5B0, 0xB5F8, 0xE92D):
        # likely prologue
        break
    fn -= 2
print(f"packer fn candidate start={hex(fn)}")
dump(fn, 0x3708700, "PACKER_STATUS_FIELDS")

# Who calls 0x3708648 / nearby? Search BL targets into 0x3708600-0x3708700
print("\n=== callers of packer range ===")
targets = set()
o = 0x1000000
while o < 0x3800000 - 4:
    t = bl(o)
    if t is not None and 0x3708600 <= t < 0x3708700:
        targets.add((o, t))
    o += 2
    if o % 0x400000 == 0:
        pass
print(f"callers: {len(targets)}")
for c, t in sorted(targets)[:20]:
    print(f"  BL @{hex(c)} -> {hex(t)}")
    dump(c - 0x20, c + 0x10, f"caller@{hex(c)}")

# Tx SIM Status - find via MOVW of string low
sva = VA + (img.find(b"Tx SIM Status(app_state=%d") - MAIN)
print(f"\nTxSIMStatus VA={hex(sva)}")
lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
hits = []
o = 0x1000000
while o < 0x4000000 - 8:
    r = movw(o)
    if r and r[0] == lo:
        t = movt(o + 4)
        if t and t[0] == hi and t[1] == r[1]:
            hits.append(o)
        # also MOVT before MOVW
        t2 = movt(o - 4) if o >= 4 else None
        if t2 and t2[0] == hi and t2[1] == r[1]:
            hits.append(o)
    o += 2
print(f"MOVW/MOVT hits for TxSIMStatus: {len(hits)} {[hex(h) for h in hits[:10]]}")
for h in hits[:5]:
    dump(h - 0x60, h + 0x40, f"TxSIM@{hex(h)}")

# Also try string ID tables: search for "sitSendSimStatus" near code via Decode tables
j = img.find(b"sitSendSimStatusChangeInd")
print(f"\nsitSendSimStatusChangeInd str={hex(j)}")
# Search C++ mangled / symbol-like in sit libs for GetSimStatus response packing
stream = Path("sit-stream.so").read_bytes()
ril = Path("libsitril.so").read_bytes()
for n in [
    b"GetSimStatus",
    b"SendSimStatus",
    b"SimStatus",
    b"app_state",
    b"APPSTATE",
    b"RIL_AppStatus",
    b"BuildGetSimStatus",
    b"OnGetSimStatus",
]:
    print(f"stream {n}: count={stream.count(n)} first={stream.find(n)}")
    print(f"ril {n}: count={ril.count(n)} first={ril.find(n)}")

# PinSkip: find NOT eSIM string VA and MOVW hits
ps = img.find(b"PIN SKIP FAILED: NOT eSIM")
psva = VA + (ps - MAIN)
print(f"\nNOT_eSIM VA={hex(psva)}")
plo, phi = psva & 0xFFFF, (psva >> 16) & 0xFFFF
phits = []
o = 0x1000000
while o < 0x4000000 - 8:
    r = movw(o)
    if r and r[0] == plo:
        t = movt(o + 4)
        if t and t[0] == phi and t[1] == r[1]:
            phits.append(o)
        t2 = movt(o - 4) if o >= 4 else None
        if t2 and t2[0] == phi and t2[1] == r[1]:
            phits.append(o)
    o += 2
print(f"NOT_eSIM MOVW hits: {len(phits)} {[hex(h) for h in phits[:8]]}")
for h in phits[:3]:
    dump(h - 0x80, h + 0x30, f"PinSkip@{hex(h)}")

# Invalid SimState similarly
ps2 = img.find(b"PIN SKIP FAILED: Invalid SimState")
ps2va = VA + (ps2 - MAIN)
print(f"\nInvalidSimState VA={hex(ps2va)}")
illo, ilhi = ps2va & 0xFFFF, (ps2va >> 16) & 0xFFFF
ihits = []
o = 0x1000000
while o < 0x4000000 - 8:
    r = movw(o)
    if r and r[0] == illo:
        t = movt(o + 4)
        if t and t[0] == ilhi and t[1] == r[1]:
            ihits.append(o)
        t2 = movt(o - 4) if o >= 4 else None
        if t2 and t2[0] == ilhi and t2[1] == r[1]:
            ihits.append(o)
    o += 2
print(f"InvalidSimState MOVW: {len(ihits)} {[hex(h) for h in ihits[:6]]}")
for h in ihits[:2]:
    dump(h - 0xA0, h + 0x20, f"InvState@{hex(h)}")
