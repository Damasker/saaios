#!/usr/bin/env python3
"""Find GMC-style msg descriptors with msgid 0x2f50; resolve producer task.

Template from START_STACK @0x19f7dc: ... 0x2f58, flags, ..., name_va ...
No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
PATH = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin")
img = PATH.read_bytes()


def log(*a):
    print(*a, flush=True)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


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


def find_movw_movt_va(target_va, band=(0x1000000, 0x3C00000)):
    lo, hi = target_va & 0xFFFF, (target_va >> 16) & 0xFFFF
    refs = []
    o = band[0]
    while o < band[1] - 8:
        r = movw(o)
        if r and r[0] == lo:
            p = o + 4
            end = min(o + 0x28, band[1] - 4)
            while p < end:
                t = movt(p)
                if t and t[1] == r[1] and t[0] == hi:
                    refs.append(o)
                    break
                hw = u16(p)
                p += 4 if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) else 2
        o += 2
    return refs


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
            log(f" {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
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


# Decode START_STACK descriptor layout at 0x19f7c0 area more carefully as u32s
log("=== START_STACK descriptor words @0x19f7c0 ===")
for o in range(0x19F7C0, 0x19F820, 4):
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 55) if 0 <= so < len(img) else None
    log(f"  {hex(o)}: {hex(w)} {s!r}")

# Hunt data words == 0x2f50 aligned, with nearby string VA pointing to SIM/USIM/GMC
log("\n=== Data u16/u32 0x2f50 with nearby name-like VA (descriptor hunt) ===")
needle = struct.pack("<H", 0x2F50)
start = 0x1000000
cands = []
while True:
    i = img.find(needle, start, 0x2000000)  # focus likely data/code lower MAIN
    if i < 0:
        break
    # skip if this is inside Thumb MOVW encoding F642 7x50 — those are code
    # MOVW encoding: look at prior halfword
    if i >= 2:
        prev = u16(i - 2)
        if (prev & 0xFBF0) == 0xF240:
            start = i + 2
            continue
    # window: look for string VAs in ±0x20
    names = []
    for o in range(max(0, i - 0x20), min(len(img) - 4, i + 0x30), 4):
        w = u32(o)
        so = w - VA + MAIN
        if 0 <= so < len(img):
            s = cstr(so, 60)
            if s and len(s) > 6 and any(k in s for k in ("SIM", "USIM", "GMC", "INIT", "STACK")):
                names.append((hex(o), s))
    if names:
        cands.append((i, names))
    start = i + 2

log(f"descriptor-like n={len(cands)}")
for i, names in cands[:60]:
    log(f"  msgid@{hex(i)} names={names}")
    # dump surrounding u32
    base = i & ~3
    for o in range(base - 0x10, base + 0x20, 4):
        w = u32(o)
        so = w - VA + MAIN
        s = cstr(so, 40) if 0 <= so < len(img) else None
        log(f"    {hex(o)}: {hex(w)} {s!r}")

# Broader: any 0x2f50 data (not MOVW) in 0x19xxxx GMC band
log("\n=== raw 0x2f50 halfwords in 0x1900000-0x1b00000 excluding MOVW ===")
o = 0x1900000
hits = []
while o < 0x1B00000 - 2:
    if u16(o) == 0x2F50:
        prev = u16(o - 2) if o >= 2 else 0
        if (prev & 0xFBF0) != 0xF240:
            hits.append(o)
    o += 2
log(f"n={len(hits)} {[hex(x) for x in hits[:40]]}")
for h in hits[:20]:
    names = []
    for o in range(h - 0x30, h + 0x40, 4):
        w = u32(o)
        so = w - VA + MAIN
        if 0 <= so < len(img):
            s = cstr(so, 50)
            if s and len(s) > 5:
                names.append(s)
    log(f"  {hex(h)} nearby_strs={names[:6]}")

# Who references the START_STACK descriptor block? Search for ADR of 0x19f7dc / 0x19f7c8
# File offset 0x19f7dc -> VA
desc_va = va_of(0x19F7DC)
log(f"\n=== Refs to START_STACK msgid slot VA={hex(desc_va)} and block base ===")
for label, off in (("msgid_slot", 0x19F7DC), ("block", 0x19F7C8), ("name_slot", 0x19F7E8)):
    tva = va_of(off)
    refs = find_movw_movt_va(tva, (0x1400000, 0x2200000))
    log(f"  {label} VA={hex(tva)} refs={len(refs)} {[hex(x) for x in refs[:12]]}")
    for r in refs[:3]:
        dump(r - 0x30, r + 0x60, f"ref_{label}@{hex(r)}")

# Search LDR [PC] literals equal to desc_va
log("\n=== PC-lit == START_STACK block VAs ===")
for want_off in (0x19F7C0, 0x19F7DC, 0x19F7E8, 0x19F700, 0x19F800):
    want = va_of(want_off)
    needle = struct.pack("<I", want)
    prefs = []
    start = 0x1400000
    while True:
        i = img.find(needle, start, 0x2200000)
        if i < 0:
            break
        prefs.append(i)
        start = i + 1
    log(f"  lit {hex(want_off)} VA={hex(want)} at {[hex(x) for x in prefs[:10]]}")

# Parallel: USIM msgtable entry at 0x10fb078 — search PC-lit of that
log("\n=== PC-lit / MOVW of USIM msgtable SIM_INIT entry ===")
for want_off in (0x10FB078, 0x10FB07C, 0x10FB080, 0x10FA000):
    want = va_of(want_off)
    needle = struct.pack("<I", want)
    prefs = []
    start = 0x1000000
    while True:
        i = img.find(needle, start, 0x2200000)
        if i < 0:
            break
        prefs.append(i)
        start = i + 1
    refs = find_movw_movt_va(want, (0x1400000, 0x2200000))
    log(f"  {hex(want_off)} VA={hex(want)} lit={len(prefs)} movw={len(refs)}")

# Soft: list SIT opcodes related to SIM that soft CPIF already has — from prior stock builders
log("\n=== sitInformSimInit neighborhood strings (A-log bank) ===")
sit = img.find(b"sitInformSimInit")
for o in range(sit - 0x200, sit + 0x200):
    if img[o] == 0:
        continue
    # start of C string?
    if o > 0 and 32 <= img[o - 1] < 127:
        continue
    s = cstr(o, 70)
    if s and s.startswith("sit") and len(s) > 5:
        log(f"  {hex(o)}: {s!r}")

# Conclusion aids: search 'Waiting for SIM_INIT' consumer path for who it expects
wait = img.find(b"Waiting for SIM_INIT_REQ")
log(f"\nWaiting str off={hex(wait)} VA={hex(va_of(wait))}")
# A-log ids often referenced differently; search unique neighboring completed string
for key in (
    b"USIM is not INITIALISED",
    b"USIM_WAIT_FOR_INIT_REQ",
    b"USIM_INIT_REQ_RCVD",
    b"SIM_INIT_REQ_RCVD",
    b"Received SIM_INIT",
    b"Process SIM_INIT",
    b"Handle SIM_INIT",
):
    off = img.find(key)
    log(f"  {key!r}: {hex(off) if off>=0 else 'MISSING'}")

log("\nDONE")
