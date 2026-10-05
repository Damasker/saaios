#!/usr/bin/env python3
"""Dump SIM_INIT_REQ / START_STACK handlers (from msg table) for Present writes.

Table layout observed:
  u16/u32 id; char *name; void *handler; u32 flags/size  (approx)

SIM_INIT_REQ: id~0x2f50 name@0x410305ea handler@0x43909d49
START_STACK:  id~0x2f58 name@0x41030521 handler@0x4390ecf5

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
                    so = lit - VA + MAIN
                    if 0 <= so < len(img):
                        s = cstr(so, 50)
                        if s:
                            extra += f" '{s}'"
            log(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


def scan_touches(start, end):
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
                    out.append(f"MOVS2+STRB0@{hex(o)}->{hex(r)}")
                    break
        # collect interesting BLs (unique)
        if b is not None and 0x1400000 <= b <= 0x1B00000:
            out.append(f"BLusim->{hex(b)}@{hex(o)}")
        o += 2
    return out


log(f"MAIN={PATH} size={len(img)}")

# Resolve handlers — try Thumb bit clear
HANDLERS = {
    "SIM_INIT_REQ": 0x43909D49,
    "START_STACK_SERVICES_REQ": 0x4390ECF5,
    "SIM_INFO_RSP?": 0x43908D91,
    "SIM_START_IND?": 0x4390FDD9,
}

for name, hva in HANDLERS.items():
    thumb = hva & 1
    hva_clear = hva & ~1
    off = off_va(hva_clear)
    log(f"\n=== {name} handler VA={hex(hva)} thumb={thumb} off={hex(off)} valid={0<=off<len(img)} ===")
    if not (0 <= off < len(img)):
        continue
    dump(off, off + 0x200, f"{name}_handler")
    t = scan_touches(off, off + 0x400)
    # unique preserve order
    seen = set()
    uniq = []
    for x in t:
        if x not in seen:
            seen.add(x)
            uniq.append(x)
    log(f"touches/unique: {uniq[:40]}")

# Also try handler as offset directly if VA formula wrong (some tables store file offsets)
log("\n=== Alt: interpret handler words as code offsets ===")
for name, raw in (("INIT", 0x43909D49), ("STACK", 0x4390ECF5)):
    for cand in (raw, raw & 0xFFFFFF, raw - 0x40000000, raw - 0x41000000):
        if 0x1000000 <= cand < len(img) - 0x100:
            log(f"  {name} cand_off={hex(cand)}")
            # check if looks like push/prolog
            hw = u16(cand & ~1)
            log(f"    first_hw={hex(hw)}")

# Message id 0x2f50 / 0x2f58 producers (who SENDs to USIM)
log("\n=== MOVW #0x2f50 / #0x2f58 (SIM_INIT / START_STACK ids) ===")
for msgid, lab in ((0x2F50, "SIM_INIT_REQ"), (0x2F58, "START_STACK"), (0x2F57, "SIM_INFO"), (0x2FA7, "START_IND?")):
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4 and len(hits) < 40:
        r = movw(o)
        if r and r[0] == msgid:
            hits.append((o, r[1]))
        o += 2
    log(f"  {lab} #{hex(msgid)}: n={len(hits)}")
    for h, rd in hits[:12]:
        # classify: near send vs compare
        window = img[max(0, h - 0x40) : h + 0x40]
        tags = []
        for s in (b"USIM", b"GMC", b"SEND", b"SIM", b"INIT", b"STACK"):
            # can't easily; just dump
            pass
        dump(h - 0x30, h + 0x50, f"{lab}_id@{hex(h)}")
        t = scan_touches(h - 0x100, h + 0x100)
        seen = set()
        uniq = [x for x in t if not (x in seen or seen.add(x))]
        log(f"  touches: {uniq[:20]}")

# Who produces GMC ==> SIM__ [START_STACK_SERVICES_REQ]?
log("\n=== GMC START_STACK string producer ===")
gva = va_of(0x104D7B4)
# ptr at 0x19f7e8 from prior
log(f"GMC str VA={hex(gva)}")
# dump around 0x19f7e8 table
for o in range(0x19F7C0, 0x19F820, 4):
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 60) if 0 <= so < len(img) else None
    log(f"  {hex(o)}: {hex(w)} {s!r}")

# Search MOVW #0x2f50 in GMC band (often 0x19xxxx / 0x1axxxx / lower)
log("\n=== Send-side: BL patterns after MOVW msgid in 0x1800000-0x1c00000 ===")
for msgid in (0x2F50, 0x2F58):
    o = 0x1800000
    while o < 0x1C00000 - 4:
        r = movw(o)
        if r and r[0] == msgid:
            # look ahead for BL
            bls = []
            for p in range(o, min(o + 0x40, len(img) - 4), 2):
                b = bl(p)
                if b is not None:
                    bls.append((p, b))
            log(f"  #{hex(msgid)}@{hex(o)} r{r[1]} BLs={[(hex(a), hex(b)) for a,b in bls[:6]]}")
            dump(o - 0x20, o + 0x60, f"send_{hex(msgid)}@{hex(o)}")
        o += 2

# Soft-path: is there SIT opcode that emits 0x2f50?
log("\n=== SIT registration near SIM / 0x02xx that might map to INIT ===")
# Prior sitInformSimInit — find code referencing that log and see SIT id
# Search MOVW of sit ids 0x0200..0x0250 near USIM init path
sit = img.find(b"sitInformSimInit")
log(f"sitInformSimInit off={hex(sit) if sit>=0 else None}")

# Property pool strings
log("\n=== Strings containing 'property' near USIM init logs ===")
pos = 0x4CF0000
while pos < 0x4D20000:
    i = img.find(b"property", pos)
    if i < 0 or i >= 0x4D20000:
        break
    log(f"  {hex(i)}: {cstr(max(0,i-40), 120)!r}")
    pos = i + 8

log("\nDONE")
