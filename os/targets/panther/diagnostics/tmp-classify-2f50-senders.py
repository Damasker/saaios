#!/usr/bin/env python3
"""Classify MOVW #0x2f50: bare msgid (send/cmp) vs string-VA (MOVT same Rd).

Also: litpool 0x2f50, STRH msgid, GMC/USIM string xrefs, soft SIT cross-check.
No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
FN_A = 0x14F692C
SET_APP = 0x19916D2
PRESENT_ID = 0x636C

PATH = next(
    p
    for p in (
        Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
        Path("/mnt/c/Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
        Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin"),
        Path(r"C:\Users\Admin\Projects\saaios-som\fw-saaios-probe-b-modem-PATCHED-ready.bin"),
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


def decode_str_va(full_va):
    so = full_va - VA + MAIN
    return cstr(so, 70) if 0 <= so < len(img) else None


def next_movt_same_rd(o, rd, window=0x30):
    p = o + 4
    end = min(o + window, len(img) - 4)
    while p < end:
        t = movt(p)
        if t and t[1] == rd:
            return p, t[0]
        # skip 2/4
        hw = u16(p)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            p += 4
        else:
            p += 2
    return None


def dump_brief(o, span=0x40):
    lines = []
    p, end = max(0, o - 0x20), min(len(img) - 2, o + span)
    while p < end:
        hw = u16(p)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(p + 2)
            extra = ""
            r, t, b = movw(p), movt(p), bl(p)
            if r:
                extra = f" MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" BL->{hex(b)}"
            lines.append(f"  {hex(p)}: {hw:04x} {hw2:04x}{extra}")
            p += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            lines.append(f"  {hex(p)}: {hw:04x}{extra}")
            p += 2
    return "\n".join(lines)


log(f"MAIN={PATH.name} size={len(img)}")

# 1) Classify all MOVW #0x2f50 / #0x2f58
for msgid, lab in ((0x2F50, "SIM_INIT_REQ"), (0x2F58, "START_STACK")):
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        r = movw(o)
        if r and r[0] == msgid:
            hits.append((o, r[1]))
        o += 2
    bare, strva, other = [], [], []
    for o, rd in hits:
        mt = next_movt_same_rd(o, rd)
        if mt:
            full = (mt[1] << 16) | msgid
            s = decode_str_va(full)
            strva.append((o, rd, hex(full), s))
        else:
            # look for nearby BL / CMP patterns
            bls = []
            for p in range(o, min(o + 0x40, len(img) - 4), 2):
                b = bl(p)
                if b is not None:
                    bls.append((hex(p), hex(b)))
            bare.append((o, rd, bls[:8]))
    log(f"\n=== {lab} #{hex(msgid)} MOVW n={len(hits)} bare={len(bare)} strVA={len(strva)} ===")
    log(f"  bare sites:")
    for o, rd, bls in bare:
        log(f"    {hex(o)} r{rd} BLs={bls}")
        log(dump_brief(o, 0x50))
    # sample a few strVA with SIM/INIT in string
    simish = [x for x in strva if x[3] and ("SIM" in x[3] or "INIT" in x[3] or "USIM" in x[3] or "GMC" in x[3])]
    log(f"  strVA with SIM/INIT/USIM/GMC: n={len(simish)}")
    for o, rd, full, s in simish[:25]:
        log(f"    {hex(o)} {full} {s!r}")

# 2) Literal pool words == 0x2f50 (msgid as 32-bit imm)
log("\n=== litpool u32 == 0x2f50 / 0x2f58 (code-band LDR PC refs) ===")
needle50 = struct.pack("<I", 0x2F50)
needle58 = struct.pack("<I", 0x2F58)
for needle, lab in ((needle50, "0x2f50"), (needle58, "0x2f58")):
    refs = []
    start = 0x1000000
    while True:
        i = img.find(needle, start, 0x3C00000)
        if i < 0:
            break
        if i % 4 == 0:
            # find LDR Rt,[PC,#imm] -> this literal within ±0x400
            for q in range(max(0x1000000, i - 0x400), i, 2):
                hw = u16(q)
                if (hw & 0xF800) != 0x4800:
                    continue
                imm = (hw & 0xFF) << 2
                pc = (q + 4) & ~3
                if pc + imm == i:
                    refs.append((q, (hw >> 8) & 7, i))
        start = i + 4
    log(f"  {lab}: lit_LDR_refs={len(refs)}")
    for q, rt, lit in refs[:30]:
        bls = []
        for p in range(q, min(q + 0x30, len(img) - 4), 2):
            b = bl(p)
            if b is not None:
                bls.append(hex(b))
        log(f"    LDR r{rt}@{hex(q)} lit@{hex(lit)} BLs={bls[:6]}")
        log(dump_brief(q, 0x40))

# 3) STRH immediate / MOVW then STRH of msgid into msg header
log("\n=== MOVW #0x2f50 then STRH same Rd within 0x20 (header store) ===")
o = 0x1000000
hdr = []
while o < 0x3C00000 - 8:
    r = movw(o)
    if r and r[0] == 0x2F50:
        rd = r[1]
        # skip if MOVT same Rd (string)
        if next_movt_same_rd(o, rd):
            o += 2
            continue
        for p in range(o + 4, min(o + 0x20, len(img) - 2), 2):
            hw = u16(p)
            # STRH Rt,[Rn,#imm5] T1: 1000 0 imm5 Rn Rt
            if (hw & 0xF800) == 0x8000 and (hw & 7) == rd:
                hdr.append((o, p, hw))
            # STRH.W
            if (hw & 0xFFF0) == 0xF8A0:
                hw2 = u16(p + 2)
                if ((hw2 >> 12) & 0xF) == rd:
                    hdr.append((o, p, (hw << 16) | hw2))
        # also: MOV Rd,Rm then use — skip
    o += 2
log(f"  header-store candidates n={len(hdr)}")
for a, b, hw in hdr[:40]:
    log(f"    MOVW@{hex(a)} STRH@{hex(b)} hw={hw if isinstance(hw,int) and hw<0x10000 else hex(hw) if isinstance(hw,int) else hw}")
    log(dump_brief(a, 0x40))

# 4) Message table entry 0x2f50 @ 0x10fb078 — who indexes it?
log("\n=== msgtable 0x2f50 neighborhood + ptr refs ===")
for o in range(0x10FB060, 0x10FB0A0, 4):
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 50) if 0 <= so < len(img) else None
    log(f"  {hex(o)}: {hex(w)} {s!r}")

# 5) GMC START_STACK string producers — parallel for SIM_INIT
log("\n=== String producers: GMC/USIM INIT / START_STACK ===")
for key in (
    b"GMC ==> SIM__ [START_STACK_SERVICES_REQ]",
    b"USIM <== SIM_INIT_REQ",
    b"USIM ==> SIM_INIT",
    b"GMC ==> SIM__ [SIM_INIT",
    b"MMC ==> USIM",
    b"sitInformSimInit",
    b"USIM_SCHEDULE_SIM_INIT_TIMER",
    b"SIM_INIT_REQ",
):
    off = img.find(key)
    if off < 0:
        log(f"  {key!r}: MISSING")
        continue
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
    log(f"  {key!r}: off={hex(off)} VA={hex(sva)} ptrs={len(prefs)} {[hex(x) for x in prefs[:8]]}")

# 6) Soft CPIF / sit-stream: any builder emitting 0x2f50?
log("\n=== sit-stream / soft builders near 0x02xx SIM ops — 0x2f50 emit? ===")
# Prior: sit builders live near sitInformSimInit; scan ±0x2000 for MOVW #0x2f50 bare
sit = img.find(b"sitInformSimInit")
if sit >= 0:
    band0, band1 = max(0x1000000, sit - 0x80000), min(len(img), sit + 0x80000)
    # Actually sit string is in log bank; find code that logs it via MOVW/MOVT of its VA
    sva = va_of(sit)
    lo, hi = sva & 0xFFFF, (sva >> 16) & 0xFFFF
    code_refs = []
    o = 0x1400000
    while o < 0x2200000 - 8:
        r = movw(o)
        if r and r[0] == lo:
            t = movt(o + 4)
            if t and t[0] == hi and t[1] == r[1]:
                code_refs.append(o)
            # MOVT may be a few instr later
            mt = next_movt_same_rd(o, r[1], 0x20)
            if mt and mt[1] == hi:
                code_refs.append(o)
        o += 2
    log(f"  sitInformSimInit VA={hex(sva)} code_MOVW/MOVT refs={len(code_refs)} {[hex(x) for x in code_refs[:12]]}")
    for r in code_refs[:8]:
        # any bare 0x2f50 in ±0x200?
        nearby = []
        for p in range(max(0, r - 0x200), min(len(img) - 4, r + 0x200), 2):
            mw = movw(p)
            if mw and mw[0] == 0x2F50 and not next_movt_same_rd(p, mw[1]):
                nearby.append(hex(p))
        log(f"    ref@{hex(r)} bare_2f50_nearby={nearby}")
        log(dump_brief(r, 0x60))

# 7) Non-STRB PresentObj[0]=2 hunt
log("\n=== Non-STRB PresentObj[0]=2 hunt (#636c getobj vicinity) ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append(o)
    o += 2
log(f"  #636c MOVW sites n={len(sites)}")

def scan_present2_patterns(start, end, lab):
    hits = []
    o = start
    while o < end - 4:
        hw, hw2 = u16(o), u16(o + 2) if o + 2 < len(img) else 0
        # MOVS #2 + STRB [Rn,#0]
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for q in range(o + 2, min(o + 16, end - 2), 2):
                h2 = u16(q)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    hits.append(("MOVS2+STRB0", o, q))
                # STRB.W Rt,[Rn,#0]
                if (h2 & 0xFFF0) == 0xF880:
                    h22 = u16(q + 2)
                    if ((h22 >> 12) & 0xF) == rt and (h22 & 0xFFF) == 0:
                        hits.append(("MOVS2+STRB.W0", o, q))
        # MOVW #2 then STRB
        mw = movw(o)
        if mw and mw[0] == 2:
            rd = mw[1]
            for q in range(o + 4, min(o + 20, end - 2), 2):
                h2 = u16(q)
                if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == (rd & 7):
                    hits.append(("MOVW2+STRB0", o, q))
                if (h2 & 0xFFF0) == 0xF880:
                    h22 = u16(q + 2)
                    if ((h22 >> 12) & 0xF) == rd and (h22 & 0xFFF) == 0:
                        hits.append(("MOVW2+STRB.W0", o, q))
        # STR #2 via STM / STR.W of word with low byte 2 — look for MOVS#2 + STR (word)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            rt = (hw >> 8) & 7
            for q in range(o + 2, min(o + 16, end - 2), 2):
                h2 = u16(q)
                # STR Rt,[Rn,#0] T2: 0110 0 imm5 Rn Rt
                if (h2 & 0xF800) == 0x6000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    hits.append(("MOVS2+STR0_word", o, q))
                # STRH
                if (h2 & 0xF800) == 0x8000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                    hits.append(("MOVS2+STRH0", o, q))
                # STMIA / STMDB containing rt — rough: STMIA Rn!,{...} 
                if (h2 & 0xFF80) == 0xC000 and ((h2 >> 0) & (1 << rt)):
                    hits.append(("MOVS2+STM_incl", o, q))
        # memcpy-ish: MOVW #2 into r1/r2 as fill byte near getobj — flag BL after MOVS#2
        o += 2
    if hits:
        log(f"  {lab}: {hits}")
    return hits

all_p2 = []
for s in sites:
    # window after getobj id load
    h = scan_present2_patterns(s, min(s + 0x120, len(img)), f"#{hex(s)}")
    all_p2.extend((s, x) for x in h)
    # also check FN_A membership
fn_a_only = []
other = []
for s, h in all_p2:
    kind, a, b = h
    in_fna = FN_A <= a <= FN_A + 0x200 or FN_A <= s <= FN_A + 0x200
    (fn_a_only if in_fna else other).append((hex(s), kind, hex(a), hex(b)))

log(f"\n  Present=2 pattern near #636c: total={len(all_p2)} in_FN_A={len(fn_a_only)} OTHER={len(other)}")
for row in fn_a_only:
    log(f"    FN_A {row}")
for row in other:
    log(f"    OTHER {row}")

# Wide: STR.W [Rn,#0] with const 2 constructed differently near #636c — already covered
# Template: search memcpy size patterns with src pointing to const byte 2 — hard; skip blind

# 8) Who sends: look for task names near bare sites via lit strings in ±0x200
log("\n=== Lit strings near bare 0x2f50 (task ID) ===")
# recompute bare for 0x2f50 only
bare50 = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == 0x2F50 and not next_movt_same_rd(o, r[1]):
        bare50.append(o)
    o += 2
for site in bare50:
    strs = []
    for q in range(max(0x1000000, site - 0x200), min(site + 0x200, len(img) - 2), 2):
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
            s = cstr(so, 60)
            if s and len(s) > 4:
                strs.append(s)
    log(f"  bare@{hex(site)} strs={strs[:12]}")

log("\nDONE")
