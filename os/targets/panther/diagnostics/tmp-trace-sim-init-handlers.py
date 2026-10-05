#!/usr/bin/env python3
"""Trace SIM_INIT_REQ / START_STACK handlers for Present=2 (non-FN_A).

Uses known string VAs from prior eu-ready-path scan. No live I/O. No secrets.
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


def off_va(v):
    return MAIN + (v - VA)


def cstr(off, n=120):
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
            if (hw & 0xFFF0) == 0xF8A0:
                extra += f" ;STRH.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                extra += f" ;STR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
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
                    extra += f" lit={hex(lit)}"
                    so = lit - VA + MAIN
                    if 0 <= so < len(img):
                        extra += f" '{cstr(so, 50)}'"
            log(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


def find_ptr_refs(target_va, limit=50):
    needle = struct.pack("<I", target_va)
    refs = []
    start = 0
    while len(refs) < limit:
        i = img.find(needle, start)
        if i < 0:
            break
        refs.append(i)
        start = i + 1
    return refs


def code_near_litpool(lit_off):
    hits = []
    base = lit_off & ~3
    for o in range(max(0, lit_off - 0x400), lit_off, 2):
        hw = u16(o)
        if (hw & 0xF800) == 0x4800:
            imm = (hw & 0xFF) << 2
            pc = (o + 4) & ~3
            if pc + imm == base:
                hits.append((o, (hw >> 8) & 7, "t1"))
        if o + 4 <= len(img) and u16(o) == 0xF8DF:
            hw2 = u16(o + 2)
            pc = (o + 4) & ~3
            if pc + (hw2 & 0xFFF) == base:
                hits.append((o, (hw2 >> 12) & 0xF, "w"))
    return hits


def touches_near(lo, span=0x200):
    touches = []
    for q in range(max(0, lo - span), min(len(img) - 4, lo + span), 2):
        mw = movw(q)
        if mw and mw[0] == PRESENT_ID:
            touches.append(f"#636c@{hex(q)}")
        b = bl(q)
        if b in (FN_A, SET_APP, STATUS, STATUS_WRAP):
            touches.append(f"BL->{hex(b)}@{hex(q)}")
        hw, hw2 = u16(q), u16(q + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            touches.append(f"STRB+BF6@{hex(q)}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for r in range(q + 2, min(q + 14, len(img) - 2), 2):
                h2 = u16(r)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == ((hw >> 8) & 7):
                    touches.append(f"MOVS2+STRB0@{hex(q)}->{hex(r)}")
                    break
    return touches


log(f"MAIN={PATH} size={len(img)}")

# Known string file offsets from eu-ready-path.out
KNOWN = {
    "USIM <== SIM_INIT_REQ": 0x10371FA,
    "USIM not INITIALISED prop (1)": 0x4CF3CB7,
    "USIM not INITIALISED prop (2)": 0x4D049D7,
    "USIM <== SIM_START_STACK_SERVICES_REQ": 0x1037131,
    "GMC ==> SIM__ [START_STACK_SERVICES_REQ]": 0x104D7B4,
    "ASIM_START_STACK_SERVICES_REQ": 0x4E0C307,
    "USIM_WAIT_FOR_INIT_REQ": 0x10348AA,
    "USIM_CARD_PRESENT": 0x103490C,
    "Sending SIM_PRESENT_IND from WAIT": 0x4D095A7,
    "Sending SIM_PRESENT_IND from CARD_PRESENT": 0x4D098C3,
    "Waiting for SIM_INIT_REQ": img.find(b"Waiting for SIM_INIT_REQ"),
    "sitInformSimInit": img.find(b"sitInformSimInit"),
    "Update system property on SIM_INIT_REQ": img.find(b"Update system property on SIM_INIT_REQ"),
}

log("\n=== Known strings verify ===")
for name, off in KNOWN.items():
    if off is None or off < 0:
        log(f"  {name}: MISSING")
        continue
    log(f"  {name}: off={hex(off)} VA={hex(va_of(off))} '{cstr(off, 90)}'")

# Trace each via pointer refs + loaders
log("\n=== Trace handlers via litpool ===")
for name, off in KNOWN.items():
    if off is None or off < 0:
        continue
    # Prefer exact string start; for A[...] logs, search the printable start
    va = va_of(off)
    # Some offs point mid-string (A prefix). Prefer find of unique substring.
    prefs = [p for p in find_ptr_refs(va, limit=40) if abs(p - off) > 8]
    log(f"\n## {name} VA={hex(va)} ptrs={len(prefs)}")
    if not prefs:
        # try VA of actual string start if off points after 'A'
        s = cstr(off, 8)
        if s and s.startswith("A["):
            # find real start - already at A
            pass
        # Also try scanning MOVW/MOVT for this VA (narrow: only lo16 matches near hi)
        lo, hi = va & 0xFFFF, (va >> 16) & 0xFFFF
        mov_refs = []
        o = 0x1000000
        while o < 0x3C00000 - 8 and len(mov_refs) < 20:
            r = movw(o)
            if r and r[0] == lo:
                t = movt(o + 4)
                if t and t[0] == hi and t[1] == r[1]:
                    mov_refs.append(o)
            o += 2
        log(f"  MOVW/MOVT refs={len(mov_refs)} {[hex(x) for x in mov_refs[:10]]}")
        for r in mov_refs[:3]:
            dump(r - 0x30, r + 0x80, f"movref@{hex(r)}")
            log(f"  touches: {touches_near(r)}")
        continue
    for p in prefs[:8]:
        loaders = code_near_litpool(p)
        log(f"  lit@{hex(p)} loaders={[(hex(a), b, c) for a,b,c in loaders[:6]]}")
        for lo, rt, kind in loaders[:2]:
            dump(lo - 0x40, lo + 0xA0, f"{name}@{hex(lo)}")
            log(f"  touches: {touches_near(lo)}")

# Special: "Update system property on SIM_INIT_REQ" — what property? dump xref fn
prop = img.find(b"Update system property on SIM_INIT_REQ")
if prop >= 0:
    log("\n=== System property on SIM_INIT_REQ — surrounding log strings ===")
    log(f"  ctx: {cstr(prop - 100, 250)!r}")
    # Find property key strings nearby in same function via lit loads — scan +/-4KB for
    # "persist." / "vendor." / "ril." / "gsm." / "sim." property-like strings in litpools
    # Better: find ptr refs to this log and dump larger window
    va = va_of(prop)
    prefs = [p for p in find_ptr_refs(va, limit=20) if abs(p - prop) > 8]
    log(f"  ptrs={len(prefs)}")
    for p in prefs[:4]:
        for lo, rt, kind in code_near_litpool(p)[:1]:
            dump(lo - 0x100, lo + 0x180, f"PROP_FN@{hex(lo)}")
            # collect all PC-literal strings in this window
            strs = []
            for q in range(lo - 0x100, lo + 0x180, 2):
                hw = u16(q)
                if (hw & 0xF800) == 0x4800:
                    imm = (hw & 0xFF) << 2
                    pc = (q + 4) & ~3
                    tgt = pc + imm
                    if tgt + 4 <= len(img):
                        lit = u32(tgt)
                        so = lit - VA + MAIN
                        if 0 <= so < len(img):
                            s = cstr(so, 80)
                            if s and len(s) > 4:
                                strs.append(s)
            log(f"  lit_strings: {strs[:30]}")
            log(f"  touches: {touches_near(lo, 0x300)}")

# Who sends SIM_INIT_REQ TO USIM? Search producers of the message id
log("\n=== Message id names near SIM_INIT_REQ ===")
for key in (
    b"SIM_INIT_REQ",
    b"SIM_INIT_CNF",
    b"SIM_START_STACK_SERVICES_REQ",
    b"SIM_START_STACK_SERVICES_CNF",
    b"SIM_PRESENT_IND",
    b"SIM_START_IND",
    b"SIM_PIN_STATUS_IND",
):
    pos = 0
    n = 0
    while n < 15:
        i = img.find(key, pos)
        if i < 0:
            break
        s = cstr(max(0, i - 24), 100)
        log(f"  @{hex(i)}: {s!r}")
        pos = i + len(key)
        n += 1

# STRH +0xBF6 quick classify
log("\n=== STRH +0xBF6 sites classify ===")
for site in (0x299E87A, 0x33CF69E, 0x33CFB5C, 0x3ACD9EC):
    dump(site - 0x30, site + 0x20, f"BF6STRH@{hex(site)}")
    # look for #636c or SET_APP in +/-0x100
    log(f"  touches: {touches_near(site, 0x100)}")

# Final Present=2 inventory
log("\n=== Present=2 inventory (#636c + MOVS#2 + STRB#0) ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append(o)
    o += 2
p2 = []
for s in sites:
    for p in range(s, min(s + 0xC0, len(img) - 2), 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for q in range(p + 2, min(p + 16, len(img) - 2), 2):
                h2 = u16(q)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    p2.append((s, p, q))
log(f"n={len(p2)}")
for x in p2:
    log(f"  #636c@{hex(x[0])} MOVS2@{hex(x[1])} STRB@{hex(x[2])} FN_A={0x14F692C<=x[2]<=0x14F6B00}")

log("\nDONE")
