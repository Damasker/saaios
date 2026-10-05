#!/usr/bin/env python3
"""Fast follow-up: literal-pool SIM_INIT xrefs + STRH +0xBF6 dumps + init trigger.

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
            if (hw & 0xFFF0) == 0xF8D0:
                extra += f" ;LDR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            log(f" {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x4800:
                # LDR Rt,[PC,#imm]
                imm = (hw & 0xFF) << 2
                rt = (hw >> 8) & 7
                # PC aligned
                pc = (o + 4) & ~3
                tgt = pc + imm
                extra = f" ;LDR r{rt},[PC,#0x{imm:x}] -> {hex(tgt)}"
                if tgt + 4 <= len(img):
                    lit = u32(tgt)
                    extra += f" lit={hex(lit)}"
                    if MAIN <= off_va(lit) < len(img) if False else True:
                        so = lit - VA + MAIN if VA <= lit < VA + (len(img) - MAIN) else -1
                        if so >= 0:
                            extra += f" str={cstr(so, 60)!r}"
            log(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


def find_ptr_refs(target_va, limit=40):
    """Find little-endian 32-bit pointer occurrences of target_va."""
    needle = struct.pack("<I", target_va)
    refs = []
    start = 0
    while len(refs) < limit:
        i = img.find(needle, start)
        if i < 0:
            break
        # skip the string itself region
        refs.append(i)
        start = i + 1
    return refs


def code_near_litpool(lit_off, back=0x80, fwd=0x20):
    """Find Thumb LDR [PC,#imm] that load this literal pool slot."""
    hits = []
    # LDR T1: PC-relative, pool at aligned PC+imm
    for o in range(max(0, lit_off - 0x400), lit_off, 2):
        hw = u16(o)
        if (hw & 0xF800) != 0x4800:
            continue
        imm = (hw & 0xFF) << 2
        rt = (hw >> 8) & 7
        pc = (o + 4) & ~3
        if pc + imm == (lit_off & ~3) or pc + imm == lit_off:
            hits.append((o, rt))
        # also allow lit_off aligned down
        if (pc + imm) == (lit_off & ~3):
            hits.append((o, rt))
    # LDR.W PC-relative (F8DF / F85F)
    for o in range(max(0, lit_off - 0x800), lit_off, 2):
        hw, hw2 = u16(o), u16(o + 2) if o + 4 <= len(img) else 0
        # LDR.W Rt,[PC,#imm] encoding: 0xF8DF / 0xF85F with U bit
        if hw in (0xF8DF, 0xF85F) or (hw & 0xFF7F) == 0xF85F:
            imm = hw2 & 0xFFF
            add = 1 if (hw & 0x0080) or hw == 0xF8DF else 0
            # simplify: F8DF = LDR.W Rt,[PC,#imm12] always add
            if hw == 0xF8DF:
                pc = (o + 4) & ~3
                if pc + (hw2 & 0xFFF) == (lit_off & ~3) or abs((pc + (hw2 & 0xFFF)) - lit_off) <= 3:
                    hits.append((o, (hw2 >> 12) & 0xF))
    return hits


log(f"MAIN={PATH} size={len(img)}")

# ------------------------------------------------------------------
# 1) STRH/STR +0xBF6 sites — are they SIM status object?
# ------------------------------------------------------------------
log("\n=== 1) Dump STRH/STR sites touching +0xBF6 / +0xBF4 ===")
for site in (0x14FB380, 0x299E87A, 0x33CF69E, 0x33CFB5C, 0x3ACD9EC,
             0x1991734, 0x19A539C, 0x14FB432, 0x18FE446):
    dump(site - 0x40, site + 0x30, f"store@{hex(site)}")
    # string search nearby in +/- 0x200 bytes
    chunk = img[max(0, site - 0x100): site + 0x80]
    for n in (b"Present", b"SIM", b"USIM", b"STATUS", b"PIN", b"BF6", b"app_state", b"SET_APP"):
        if n in chunk:
            log(f"  nearby_bytes contain {n!r}")

# ------------------------------------------------------------------
# 2) Literal-pool / pointer refs to key SIM_INIT strings
# ------------------------------------------------------------------
log("\n=== 2) Pointer + litpool refs to SIM_INIT strings ===")
keys = [
    b"Waiting for SIM_INIT_REQ",
    b"USIM <== SIM_INIT_REQ",
    b"USIM ==> SIM_INIT_REQ",
    b"START_STACK_SERVICES",
    b"SIM_START_STACK_SERVICES",
    b"USIM ==> SIM_START_IND",
    b"SIM_PRESENT_IND",
    b"SIM_PIN_STATUS_IND",
    b"USIM_WAIT_FOR_INIT_REQ",
    b"USIM_CARD_PRESENT",
    b"sitInformSimInit",
    b"GMC ==> SIM__",
    b"SIM__ [START_STACK",
    b"Waiting for",
]
for key in keys:
    off = img.find(key)
    if off < 0:
        # try partial
        log(f"  {key!r}: MISSING")
        continue
    va = va_of(off)
    prefs = find_ptr_refs(va, limit=30)
    # filter out the string's own location if somehow packed
    prefs = [p for p in prefs if abs(p - off) > len(key)]
    log(f"\n  {key!r}: off={hex(off)} VA={hex(va)} ptr_refs={len(prefs)}")
    for p in prefs[:12]:
        log(f"    ptr@{hex(p)} context={cstr(p - 16, 48)!r}")
        # who loads this literal?
        loaders = code_near_litpool(p)
        log(f"      LDR-near n={len(loaders)} {[hex(x[0]) for x in loaders[:8]]}")
        for lo, rt in loaders[:2]:
            dump(lo - 0x30, lo + 0x60, f"loader@{hex(lo)}")
            # Present / SET_APP / FN_A in +/- 0x180
            touches = []
            for q in range(max(0, lo - 0x180), min(len(img) - 4, lo + 0x180), 2):
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
                    # MOVS #2 near Present store?
                    for r in range(q + 2, min(q + 12, len(img) - 2), 2):
                        h2 = u16(r)
                        if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0:
                            touches.append(f"MOVS2+STRB0@{hex(q)}")
                            break
            log(f"      touches: {touches[:15]}")

# Also search GMC SIM message format strings
log("\n=== 2b) GMC/USIM message arrow strings containing SIM ===")
pos = 0
count = 0
while count < 60:
    i = img.find(b"SIM", pos)
    if i < 0:
        break
    s = cstr(max(0, i - 20), 100)
    if s and ("USIM" in s or "GMC" in s or "SIM_" in s or "SIM <" in s or "SIM >" in s or "SIM =" in s):
        if any(k in s for k in ("INIT", "START", "PRESENT", "STACK", "POWER", "ATR", "READY", "WAIT")):
            log(f"  @{hex(i)}: {s!r}")
            count += 1
    pos = i + 1

# ------------------------------------------------------------------
# 3) Who sends SIM_INIT_REQ internally? Search '==> SIM_INIT' / '<== SIM_INIT'
# ------------------------------------------------------------------
log("\n=== 3) Directional SIM_INIT / START_STACK message strings ===")
for key in (
    b"USIM <== SIM_INIT_REQ",
    b"USIM ==> SIM_INIT_CNF",
    b"USIM ==> SIM_INIT_REQ",
    b"GMC ==> SIM__ [START_STACK_SERVICES_REQ]",
    b"START_STACK_SERVICES_REQ",
    b"SIM_START_STACK_SERVICES_REQ",
    b"USIM <== SIM_START_STACK",
    b"USIM ==> SIM_PRESENT_IND",
    b"PBM <== SIM_PRESENT_IND",
    b"USIM ==> SIM_START_IND",
    b"USIM ==> SIM_PIN_STATUS_IND",
):
    off = img.find(key)
    if off < 0:
        # fuzzy: find unique substring
        log(f"  exact {key!r}: MISSING — fuzzy:")
        # try without brackets
        sub = key.split(b"[")[0].strip() if b"[" in key else key
        off2 = img.find(sub)
        if off2 >= 0:
            log(f"    fuzzy {sub!r} @{hex(off2)}: {cstr(off2, 100)!r}")
            off = off2
        else:
            continue
    else:
        log(f"  {key!r} @{hex(off)}: {cstr(off, 100)!r}")
    va = va_of(off)
    prefs = [p for p in find_ptr_refs(va, limit=20) if abs(p - off) > 4]
    log(f"    ptr_refs={len(prefs)} {[hex(p) for p in prefs[:8]]}")
    for p in prefs[:4]:
        loaders = code_near_litpool(p)
        log(f"    lit@{hex(p)} loaders={[hex(x[0]) for x in loaders[:6]]}")
        for lo, rt in loaders[:1]:
            dump(lo - 0x20, lo + 0x80, f"msglog@{hex(lo)}")
            touches = []
            for q in range(max(0, lo - 0x200), min(len(img) - 4, lo + 0x200), 2):
                mw = movw(q)
                if mw and mw[0] == PRESENT_ID:
                    touches.append(f"#636c@{hex(q)}")
                b = bl(q)
                if b in (FN_A, SET_APP, STATUS, STATUS_WRAP):
                    touches.append(f"BL->{hex(b)}@{hex(q)}")
            log(f"    touches: {touches}")

# ------------------------------------------------------------------
# 4) sitInformSimInit + factory sit path
# ------------------------------------------------------------------
log("\n=== 4) sitInformSimInit / InformSim ===")
for key in (b"sitInformSimInit", b"InformSimInit", b"InformSim", b"SimInitComplete", b"SIM_INIT_COMPLETE"):
    off = img.find(key)
    if off >= 0:
        log(f"  {key!r}: {hex(off)} {cstr(off,80)!r}")
    else:
        log(f"  {key!r}: MISSING")
    if off >= 0:
        prefs = [p for p in find_ptr_refs(va_of(off), limit=15) if abs(p - off) > 4]
        log(f"    ptr_refs={len(prefs)} {[hex(p) for p in prefs[:8]]}")

# ------------------------------------------------------------------
# 5) Alternate Present identity: does STATUS use getobj other than #636c?
# ------------------------------------------------------------------
log("\n=== 5) STATUS getobj / Present source ===")
dump(STATUS, STATUS + 0x100, "STATUS_HEAD")
# find MOVW near STATUS for object ids
ids = []
for q in range(STATUS - 0x40, STATUS + 0x2C0, 2):
    mw = movw(q)
    if mw:
        ids.append((q, mw[0], mw[1]))
log(f"MOVW immediates in STATUS window: {[(hex(a), hex(b), c) for a,b,c in ids[:40]]}")

# ------------------------------------------------------------------
# 6) Soft-CPIF skip hypothesis: does CP wait forever for SIM_INIT from AP?
#    Look for state machine enum values near USIM_WAIT string
# ------------------------------------------------------------------
log("\n=== 6) USIM state name table ===")
for key in (
    b"USIM_WAIT_FOR_INIT_REQ",
    b"USIM_CARD_PRESENT",
    b"USIM_WAIT_FOR_CARD",
    b"USIM_APP_SELECTED",
    b"USIM_PIN_REQUIRED",
    b"USIM_READY",
    b"USIM_INIT_REQ_RCVD",
    b"USIM_NULL",
    b"USIM_OFF",
):
    off = img.find(key)
    if off < 0:
        log(f"  {key!r}: MISSING")
        continue
    log(f"  {key!r} @{hex(off)}")
    # dump surrounding string table
    log(f"    table_window: {cstr(off - 80, 200)!r}")

# ------------------------------------------------------------------
# 7) FN_B Present=2 recheck with #636c (docs said unreachable / not PresentObj)
# ------------------------------------------------------------------
log("\n=== 7) FN_B region 0x14f9108..0x14f9800 Present stores ===")
dump(0x14F9108, 0x14F9600, "FN_B")
has636 = any(movw(q) and movw(q)[0] == PRESENT_ID for q in range(0x14F9108, 0x14F9800, 2))
log(f"FN_B has #636c: {has636}")

# ------------------------------------------------------------------
# 8) Non-STRB Present=2: STR.W of value containing 2 at [obj,#0]
# ------------------------------------------------------------------
log("\n=== 8) Near #636c: STR.W [reg,#0] after MOV #2 ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append(o)
    o += 2
log(f"#636c sites={len(sites)}")
for s in sites:
    for p in range(s, min(s + 0x100, len(img) - 4), 2):
        hw, hw2 = u16(p), u16(p + 2)
        # MOVW/MOVS #2 then STR.W #0
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for q in range(p + 2, min(p + 20, len(img) - 4), 2):
                h2, h2b = u16(q), u16(q + 2)
                if (h2 & 0xFFF0) == 0xF8C0 and (h2b & 0xFFF) == 0 and ((h2b >> 12) & 0xF) == rt:
                    log(f"  STR.W #2 via MOVS at #636c@{hex(s)} STR@{hex(q)}")
        mw = movw(p)
        if mw and mw[0] == 2:
            for q in range(p + 4, min(p + 24, len(img) - 4), 2):
                h2, h2b = u16(q), u16(q + 2)
                if (h2 & 0xFFF0) == 0xF8C0 and (h2b & 0xFFF) == 0 and ((h2b >> 12) & 0xF) == mw[1]:
                    log(f"  STR.W #2 via MOVW at #636c@{hex(s)} STR@{hex(q)}")

log("\nDONE")
