#!/usr/bin/env python3
"""Trace real USIM SIM_INIT_REQ (0x2f50) senders + consumer switch for Present.

No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
FN_A = 0x14F692C
STATUS = 0x14FB322
STATUS_WRAP = 0x14C6626
SET_APP = 0x19916D2
PRESENT_ID = 0x636C

PATH = next(
    p
    for p in (
        Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
        Path("/mnt/c/Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
    )
    if p.exists()
)
img = PATH.read_bytes()


def log(*a):
    print(*a, flush=True)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def bl(o):
    if o + 4 > len(img):
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


def movw(o):
    if o + 4 > len(img):
        return None
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    if o + 4 > len(img):
        return None
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def va_of(o):
    return VA + (o - MAIN)


def cstr(off, n=80):
    if off < 0 or off >= len(img):
        return None
    s = bytearray()
    for i in range(off, min(len(img), off + n)):
        c = img[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def dump(start, end, lab):
    log(f"\n--- {lab} {hex(start)}..{hex(end)} ---")
    o = max(0, start)
    end = min(end, len(img) - 2)
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
            if (hw & 0xF800) == 0x4800:
                pass
            log(f" {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x4800:
                imm = (hw & 0xFF) << 2
                rt = (hw >> 8) & 7
                pc = (o + 4) & ~3
                tgt = pc + imm
                extra = f" ;LDR r{rt},[PC,#0x{imm:x}]->{hex(tgt)}"
                if tgt + 4 <= len(img):
                    lit = u32(tgt)
                    so = lit - VA + MAIN
                    if 0 <= so < len(img):
                        s = cstr(so, 55)
                        if s:
                            extra += f" '{s}'"
            log(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


def lit_strings(start, end):
    strs = []
    for q in range(start, end, 2):
        hw = u16(q)
        if (hw & 0xF800) != 0x4800:
            continue
        imm = (hw & 0xFF) << 2
        pc = (q + 4) & ~3
        tgt = pc + imm
        if tgt + 4 > len(img):
            continue
        lit = u32(tgt)
        so = lit - VA + MAIN
        if 0 <= so < len(img):
            s = cstr(so, 70)
            if s and len(s) > 5:
                strs.append((q, s))
    return strs


def touches(start, end):
    out = []
    o = start
    while o < end - 4:
        mw = movw(o)
        if mw and mw[0] == PRESENT_ID:
            out.append(f"#636c@{hex(o)}")
        b = bl(o)
        if b in (FN_A, SET_APP, STATUS, STATUS_WRAP):
            out.append(f"BL->{hex(b)}@{hex(o)}")
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            out.append(f"STRB+BF6@{hex(o)}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for r in range(o + 2, min(o + 14, end - 2), 2):
                h2 = u16(r)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    out.append(f"MOVS2+STRB0@{hex(o)}")
                    break
        o += 2
    return out


log(f"MAIN size={len(img)}")

# Focus send/consume sites from prior that had USIM-band BLs
FOCUS = [
    ("sendish_0x1417218", 0x1417218, 0x180),
    ("sendish_0x17d9eae", 0x17D9EAE, 0x180),
    ("sendish_0x18108d4", 0x18108D4, 0x100),
    ("sendish_0x1998828", 0x1998828, 0x100),
    ("sendish_0x1a33668", 0x1A33668, 0x120),
    ("sendish_0x1a6ac76", 0x1A6AC76, 0x100),
    ("sendish_0x19e66c2", 0x19E66C2, 0x100),
]

for lab, site, span in FOCUS:
    dump(site - 0x60, site + span, lab)
    log(f"  lit_strings: {[s for _, s in lit_strings(site - 0x80, site + span)][:20]}")
    log(f"  touches: {touches(site - 0x100, site + span)}")

# CMP/MOVW #0x2f50 in USIM task band 0x14f0000-0x1600000 (consumer)
log("\n=== Consumer: #0x2f50 in USIM band 0x14f0000..0x1600000 ===")
o = 0x14F0000
hits = []
while o < 0x1600000 - 4:
    r = movw(o)
    if r and r[0] == 0x2F50:
        hits.append(o)
    # also CMP Rn, #imm12 via TEQ/CMP.W — skip for now
    o += 2
log(f"n={len(hits)} {[hex(x) for x in hits]}")
for h in hits:
    dump(h - 0x40, h + 0xC0, f"usim_cmp_2f50@{hex(h)}")
    log(f"  lit: {[s for _, s in lit_strings(h - 0x80, h + 0x100)][:15]}")
    log(f"  touches: {touches(h - 0x200, h + 0x200)}")

# SCHEDULE_SIM_INIT_TIMER
log("\n=== USIM_SCHEDULE_SIM_INIT_TIMER ===")
off = img.find(b"USIM_SCHEDULE_SIM_INIT_TIMER")
log(f"off={hex(off)} VA={hex(va_of(off))}")
# state name table already has it; find code refs via ptr
sva = va_of(off)
needle = struct.pack("<I", sva)
prefs = []
start = 0
while True:
    i = img.find(needle, start)
    if i < 0:
        break
    if abs(i - off) > 8:
        prefs.append(i)
    start = i + 1
log(f"ptr_refs={len(prefs)} {[hex(x) for x in prefs[:10]]}")

# Waiting for SIM_INIT_REQ — find via relative log id search in USIM band:
# string at 0x4d061dc; search for unique nearby completed init strings as lit PC loads
log("\n=== Code refs to 'Waiting for SIM_INIT_REQ' via PC lit in 0x14-0x1b ===")
wait_va = va_of(0x4D061DC)
# Actually A-logs may not be PC-lit. Search MOVW lo16 of wait_va in USIM band only (fast)
lo, hi = wait_va & 0xFFFF, (wait_va >> 16) & 0xFFFF
refs = []
o = 0x14F0000
while o < 0x1B00000 - 8:
    r = movw(o)
    if r and r[0] == lo:
        t = movt(o + 4)
        if t and t[0] == hi and t[1] == r[1]:
            refs.append(o)
    o += 2
log(f"MOVW/MOVT wait_va refs in 0x14-1b: {len(refs)} {[hex(x) for x in refs]}")

# Broader wait string - maybe without A prefix at 0x4d061dc is mid-string
# Full: find "INITIALISED. Waiting for SIM_INIT_REQ"
full = img.find(b"Waiting for SIM_INIT_REQ")
log(f"Waiting off={hex(full)} ctx={cstr(full-30, 80)!r}")

# usim_PalInitUicc — after SIM_INIT
log("\n=== usim_PalInitUicc ===")
for key in (b"usim_PalInitUicc", b"PalInitUicc", b"USIM Powering UP - COMPLETED", b"USIM Powering UP - START"):
    off = img.find(key)
    if off >= 0:
        log(f"  {key!r}: {hex(off)} {cstr(off, 70)!r}")
    else:
        log(f"  {key!r}: MISSING")

# Does PalInit or Powering UP path write Present?
# Search #636c in 0x3900000 band (near codec) and also find functions that log Powering UP
log("\n=== #636c sites — classify by proximity to known FN_A / ctor / others ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append(o)
    o += 2
for s in sites:
    # nearby BL SET_APP / FN_A / strings via lit
    near = []
    for p in range(s, min(s + 0x80, len(img) - 4), 2):
        b = bl(p)
        if b == FN_A:
            near.append("FN_A")
        if b == SET_APP:
            near.append("SET_APP")
        if b == STATUS:
            near.append("STATUS")
    strs = [s for _, s in lit_strings(s - 0x40, s + 0xC0)]
    # any MOVS #2 STRB #0?
    p2 = False
    for p in range(s, min(s + 0x100, len(img) - 2), 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for q in range(p + 2, min(p + 16, len(img) - 2), 2):
                h2 = u16(q)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    p2 = True
    # stores of #0
    p0 = False
    for p in range(s, min(s + 0x80, len(img) - 2), 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 0:
            rt = (hw >> 8) & 7
            for q in range(p + 2, min(p + 12, len(img) - 2), 2):
                h2 = u16(q)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    p0 = True
    log(f"  {hex(s)} p2={p2} p0={p0} near={near} strs={strs[:4]}")

log("\nDONE")
