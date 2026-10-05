#!/usr/bin/env python3
"""Identify 0x6de740 SIM_* catalog owner; contrast GMC 0x2f58 vs absent 0x2f50.

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


def cstr(off, n=90):
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


# Walk catalog entries around 0x6de700 — record record size / pattern
log("=== Catalog walk 0x6de600..0x6de900 ===")
o = 0x6DE600
while o < 0x6DE900:
    # pattern seen: u16 flags?, u16 msgid, u32 name_va, u32 meta, ...
    # try parse as 0x20-byte records starting when we see name-like VA
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 50) if 0 <= so < len(img) else None
    if s and s.startswith("SIM_"):
        # back up to find msgid halfword
        msgid = u16(o - 2) if o >= 2 else 0
        flags = u16(o - 4) if o >= 4 else 0
        meta = u32(o + 4)
        log(f"  @{hex(o-4)}: flags={hex(flags)} msgid={hex(msgid)} name={s!r} meta={hex(meta)}")
    o += 4

# Refs to catalog base / SIM_INIT_REQ entry
log("\n=== Refs to catalog VAs ===")
for off in (0x6DE600, 0x6DE700, 0x6DE740, 0x6DE744, 0x6DE000, 0x6DF000):
    tva = va_of(off)
    refs = find_movw_movt_va(tva, (0x600000, 0x2000000))
    needle = struct.pack("<I", tva)
    lits = []
    start = 0x600000
    while True:
        i = img.find(needle, start, 0x2000000)
        if i < 0:
            break
        lits.append(i)
        start = i + 1
    log(f"  {hex(off)} VA={hex(tva)} movw={len(refs)} {[hex(x) for x in refs[:8]]} lit={len(lits)} {[hex(x) for x in lits[:8]]}")

# Strings near catalog in file (owner hint)
log("\n=== C-strings in 0x6d8000..0x6e2000 ===")
pos = 0x6D8000
n = 0
while pos < 0x6E2000 and n < 80:
    if img[pos] == 0 or not (32 <= img[pos] < 127):
        pos += 1
        continue
    if pos > 0 and 32 <= img[pos - 1] < 127:
        pos += 1
        continue
    s = cstr(pos, 70)
    if s and len(s) >= 6:
        log(f"  {hex(pos)}: {s!r}")
        n += 1
        pos += len(s)
    else:
        pos += 1

# GMC descriptors: list all msgid halfwords in GMC table band that look like 0x2fxx
log("\n=== GMC descriptor band 0x19f000..0x1a2000: u16 in 0x2f00..0x3000 with nearby GMC/SIM str ===")
o = 0x19F0000
found = []
while o < 0x1A20000 - 2:
    v = u16(o)
    if 0x2F00 <= v <= 0x3000:
        # skip MOVW
        prev = u16(o - 2) if o >= 2 else 0
        if (prev & 0xFBF0) == 0xF240:
            o += 2
            continue
        names = []
        for p in range(o - 0x20, o + 0x30, 4):
            w = u32(p)
            so = w - VA + MAIN
            if 0 <= so < len(img):
                s = cstr(so, 55)
                if s and ("GMC" in s or "SIM" in s or "USIM" in s):
                    names.append(s)
        if names:
            found.append((o, v, names))
    o += 2
for o, v, names in found:
    log(f"  {hex(o)} msgid={hex(v)} {names[:3]}")

# Does any GMC entry list 0x2f50?
has_2f50 = [x for x in found if x[1] == 0x2F50]
log(f"\nGMC-band descriptor 0x2f50 count={len(has_2f50)}")

# Directionality: SIM_INIT only as USIM <== (into USIM). Who are other <== SIM_* REQ senders logged?
log("\n=== 'USIM <== SIM_' REQ strings (inbound to USIM) ===")
pos = 0
while True:
    i = img.find(b"USIM <== SIM_", pos)
    if i < 0:
        break
    s = cstr(i, 70)
    log(f"  {hex(i)}: {s!r}")
    pos = i + 1

log("\n=== 'GMC ==> SIM__' strings (GMC as sender into SIM) ===")
pos = 0
while True:
    i = img.find(b"GMC ==> SIM__", pos)
    if i < 0:
        break
    s = cstr(i, 80)
    log(f"  {hex(i)}: {s!r}")
    pos = i + 1

# Soft CPIF sit builders inventory cross-check from known paths in repo docs — scan MAIN for Build*Sim*
log("\n=== Build*Sim* / sit*Sim*Init* code-ish strings ===")
for key in (b"BuildSim", b"sitSim", b"SimInit", b"SIM_INIT", b"StartStack", b"STACK_SERVICES"):
    pos = 0
    n = 0
    while n < 15:
        i = img.find(key, pos)
        if i < 0:
            break
        s = cstr(i, 60)
        if s:
            log(f"  {key!r}@{hex(i)}: {s!r}")
            n += 1
        pos = i + 1

log("\nDONE")
