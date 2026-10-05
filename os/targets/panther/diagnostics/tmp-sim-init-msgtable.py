#!/usr/bin/env python3
"""Resolve SIM_INIT_REQ message-table entry + dispatch; hunt Present writes in handler.

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


def off_va(v):
    return MAIN + (v - VA)


def cstr(off, n=100):
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
            log(f" {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            log(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


def touches(lo, span=0x280):
    out = []
    for q in range(max(0, lo - span), min(len(img) - 4, lo + span), 2):
        mw = movw(q)
        if mw and mw[0] == PRESENT_ID:
            out.append(f"#636c@{hex(q)}")
        b = bl(q)
        if b in (FN_A, SET_APP, STATUS, STATUS_WRAP):
            out.append(f"BL->{hex(b)}@{hex(q)}")
        hw, hw2 = u16(q), u16(q + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            out.append(f"STRB+BF6@{hex(q)}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for r in range(q + 2, min(q + 14, len(img) - 2), 2):
                h2 = u16(r)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    out.append(f"MOVS2+STRB0@{hex(q)}")
                    break
    return out


log(f"MAIN size={len(img)}")

# 1) Message name table around ptr to SIM_INIT_REQ
INIT_STR = 0x10371FA
INIT_VA = va_of(INIT_STR)
STACK_STR = 0x1037131
STACK_VA = va_of(STACK_STR)
WAIT_STR = 0x10348AA
WAIT_VA = va_of(WAIT_STR)

log(f"\nINIT_VA={hex(INIT_VA)} STACK_VA={hex(STACK_VA)} WAIT_VA={hex(WAIT_VA)}")

# Dump table neighborhood at known ptr sites
for label, ptr_off in (
    ("SIM_INIT_REQ ptr", 0x10FB07C),
    ("START_STACK ptr", 0x10FB88C),
    ("WAIT_FOR_INIT ptr", 0x1104540),
    ("CARD_PRESENT ptr", 0x1104544),
    ("GMC START_STACK ptr", 0x19F7E8),
):
    log(f"\n=== Table around {label} @{hex(ptr_off)} ===")
    # dump 16 entries before/after as words, decode if look like string VAs
    base = ptr_off - 0x40
    for o in range(base, ptr_off + 0x80, 4):
        w = u32(o)
        mark = " <--" if o == ptr_off else ""
        so = w - VA + MAIN
        s = cstr(so, 60) if 0 <= so < len(img) else None
        if s:
            log(f"  {hex(o)}: {hex(w)} '{s}'{mark}")
        else:
            log(f"  {hex(o)}: {hex(w)}{mark}")

# 2) Find refs TO the table base (who uses the name table)
# Often table base loaded via MOVW/MOVT of table VA
log("\n=== Refs to message-name table pages ===")
for name, ptr_off in (("init_ptr", 0x10FB07C), ("wait_ptr", 0x1104540)):
    tva = va_of(ptr_off)
    lo, hi = tva & 0xFFFF, (tva >> 16) & 0xFFFF
    # also try page-aligned base
    for cand_va in (tva, tva & ~0xFF, tva & ~0xFFF):
        lo, hi = cand_va & 0xFFFF, (cand_va >> 16) & 0xFFFF
        refs = []
        o = 0x1000000
        while o < 0x3C00000 - 8 and len(refs) < 15:
            r = movw(o)
            if r and r[0] == lo:
                t = movt(o + 4)
                if t and t[0] == hi and t[1] == r[1]:
                    refs.append(o)
            o += 2
        if refs:
            log(f"  {name} cand_va={hex(cand_va)} refs={len(refs)} {[hex(x) for x in refs[:8]]}")

# 3) Search for message ID constants near SIM_INIT string in enum tables
# Look backward from INIT_STR for numeric ID patterns in adjacent struct
# Many Shannon builds: struct { u16 id; u16 pad; char *name; } or { char *name; u32 id }
log("\n=== Struct decode at INIT ptr slot ===")
# At 0x10fb07c we have pointer to name. Check neighbors for IDs.
slot = 0x10FB07C
for stride in (4, 8, 12, 16):
    log(f" stride={stride}:")
    for i in range(-4, 5):
        o = slot + i * stride
        w0 = u32(o)
        so = w0 - VA + MAIN
        s = cstr(so, 40) if 0 <= so < len(img) else None
        w1 = u32(o + 4) if stride >= 8 else 0
        log(f"  [{i}] @{hex(o)} w0={hex(w0)} s={s!r} w1={hex(w1)}")

# 4) Find code that compares to likely SIM_INIT message ids
# Search strings "SIM_INIT_REQ" used as case labels — also search for
# handler log "Waiting for SIM_INIT_REQ" via ADR of high VA using MOVW of low 16 of file-relative?
# Samsung A-logs often: MOVW r1, #imm; BL log — imm is string id not pointer.
# Try: find BL targets called near MOVW of distinctive constants from the A-string offset low16.

log("\n=== A-log id refs for INITIALISED / Waiting (low16 of off) ===")
for name, off in (
    ("INITIALISED_1", 0x4CF3CB7),
    ("Waiting", 0x4D061DC),
    ("PRESENT_IND_WAIT", 0x4D095A7),
    ("sitInformSimInit", 0x4C2F154),
):
    # try low16 of offset, of VA, and of (off - some base)
    candidates = {
        "off_lo": off & 0xFFFF,
        "va_lo": va_of(off) & 0xFFFF,
        "off_hiish": (off >> 8) & 0xFFFF,
    }
    # Also common: string pool base 0x4c00000-ish; id = off - pool_base
    for pool in (0x4C00000, 0x4C20000, 0x4CF0000, 0x4D00000, 0x4400000, 0x44C0000):
        if off >= pool:
            candidates[f"rel_{hex(pool)}"] = (off - pool) & 0xFFFF
    log(f"\n  {name} off={hex(off)}")
    for k, imm in candidates.items():
        hits = []
        o = 0x1400000  # focus USIM-ish code band first
        while o < 0x1B00000 - 4 and len(hits) < 8:
            r = movw(o)
            if r and r[0] == imm:
                hits.append((o, r[1]))
            o += 2
        if hits:
            log(f"    {k}=#{hex(imm)} hits_in_0x14-1b={len(hits)} e.g. {[hex(h[0]) for h in hits[:5]]}")
            for h, rd in hits[:2]:
                dump(h - 0x20, h + 0x60, f"alog_{name}_{k}@{hex(h)}")
                log(f"    touches: {touches(h, 0x180)}")

# 5) Broader: scan USIM module for #636c OR SET_APP OR MOVS2+STRB0 near SIM_INIT-ish
# Find functions containing both a BL to a log and Present store — by scanning
# for Present=2 outside FN_A again with STRB.W encoding
log("\n=== Present=2 via STRB.W [reg,#0] after MOVS/MOVW #2 near #636c ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append((o, r[1]))
    o += 2
for s, rd in sites:
    for p in range(s, min(s + 0x100, len(img) - 4), 2):
        hw = u16(p)
        imm2 = None
        rt = None
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            imm2, rt = 2, (hw >> 8) & 7
        mw = movw(p)
        if mw and mw[0] == 2:
            imm2, rt = 2, mw[1]
        if imm2 is None:
            continue
        for q in range(p + 2, min(p + 20, len(img) - 4), 2):
            h2, h2b = u16(q), u16(q + 2)
            if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                log(f"  STRB #0 @{hex(q)} after #2 @{hex(p)} #636c@{hex(s)} FN_A={0x14F692C<=q<=0x14F6B00}")
            if (h2 & 0xFFF0) == 0xF880 and (h2b & 0xFFF) == 0 and ((h2b >> 12) & 0xF) == rt:
                log(f"  STRB.W #0 @{hex(q)} after #2 @{hex(p)} #636c@{hex(s)}")

# 6) Who calls SET_APP with #5 besides STATUS? already known. Who with values that could skip?
log("\n=== All SET_APP call sites with inferred imm ===")
o = 0x1000000
set_sites = []
while o < 0x3C00000 - 4:
    if bl(o) == SET_APP:
        set_sites.append(o)
    o += 2
for c in set_sites:
    imm = None
    for p in range(c - 2, max(0, c - 0x60), -2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) == 0:
            imm = hw & 0xFF
            break
        mw = movw(p)
        if mw and mw[1] == 0:
            imm = mw[0]
            break
        # MOV r0, rN after MOVS rN,#imm
        if (hw & 0xFFC0) == 0x4600:  # MOV low
            pass
    log(f"  SET_APP BL @{hex(c)} imm~{imm}")

# 7) Soft-path gap: does sitInformSimInit get called from SIT path we skip?
log("\n=== sitInformSimInit string + nearby code via pool-rel search ===")
sit = 0x4C2F154
log(f"sitInformSimInit @ {hex(sit)} '{cstr(sit, 40)}'")
# Search code for the distinctive substring as ASCII in MOV? Unlikely.
# Find xrefs by scanning for ADRP-like: MOVW/MOVT of VA 0x44c28544
sva = va_of(sit)
lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
refs = []
o = 0x1000000
while o < 0x3C00000 - 8:
    r = movw(o)
    if r and r[0] == lo:
        t = movt(o + 4)
        if t and t[0] == hi and t[1] == r[1]:
            refs.append(o)
    o += 2
log(f"MOVW/MOVT of sitInform VA: {len(refs)} {[hex(x) for x in refs]}")
# ptr refs
needle = struct.pack("<I", sva)
prefs = []
start = 0
while True:
    i = img.find(needle, start)
    if i < 0:
        break
    if abs(i - sit) > 8:
        prefs.append(i)
    start = i + 1
log(f"ptr refs: {len(prefs)} {[hex(x) for x in prefs[:10]]}")

# 8) Critical: search "USIM is not INITIALISED" handler by finding unique nearby
# instruction pattern — the log at 0x4cf3cb7. Samsung MLOG often:
# look for XREF via compressed ID table. Alternative: search code that
# contains both SIM_INIT handling and property update strings' relative IDs.
log("\n=== Property strings near INITIALISED log pool ===")
# dump printable strings in 0x4cf3c00..0x4cf4000
off = 0x4CF3C00
while off < 0x4CF4000:
    s = cstr(off, 120)
    if s and len(s) >= 8:
        log(f"  {hex(off)}: {s}")
        off += len(s) + 1
    else:
        off += 1

log("\nDONE")
