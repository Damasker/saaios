#!/usr/bin/env python3
"""Focused non-FN_A Present=2 + SIM_INIT/USIM bring-up hunt (fast).

Writes progress with flush. No live I/O. No secrets.
"""
from __future__ import annotations

import struct
import sys
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
FN_A = 0x14F692C
STATUS = 0x14FB322
STATUS_WRAP = 0x14C6626
SET_APP = 0x19916D2
PRESENT_ID = 0x636C

_CANDS = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    Path("/mnt/c/Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]
PATH = next((p for p in _CANDS if p.exists()), None)
if PATH is None:
    raise SystemExit("MAIN missing")
img = PATH.read_bytes()


def log(*a):
    print(*a, flush=True)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


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


def cstr(off, n=140):
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


def scan_bl(target, start=0x1000000, end=0x3C00000):
    hits = []
    o = start
    while o < end - 4:
        if bl(o) == target:
            hits.append(o)
        o += 2
    return hits


def refs_va(target_va, start=0x1000000, end=0x3C00000, limit=30):
    lo, hi = target_va & 0xFFFF, (target_va >> 16) & 0xFFFF
    refs = []
    o = start
    while o < end - 8:
        r = movw(o)
        if r and r[0] == lo:
            t = movt(o + 4)
            if t and t[0] == hi and t[1] == r[1]:
                refs.append(o)
                if len(refs) >= limit:
                    break
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
            log(f" {hex(o)}: {hw:04x}{extra}")
            o += 2


log(f"MAIN={PATH} size={len(img)}")

# A) collect #636c sites once
log("\n=== A) MOVW #0x636c sites + stores ===")
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == PRESENT_ID:
        sites.append((o, r[1]))
    o += 2
log(f"n={len(sites)}")
present2_sites = []
for s, rd in sites:
    notes = []
    for p in range(s, min(s + 0x140, len(img) - 4), 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xF800) == 0x7000:
            notes.append(f"STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]@{hex(p)}")
        if (hw & 0xFFF0) == 0xF880:
            notes.append(f"STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]@{hex(p)}")
        if (hw & 0xFFF0) == 0xF8C0:
            notes.append(f"STR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]@{hex(p)}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1, 2, 3):
            notes.append(f"MOVS r{(hw>>8)&7},#{hw&0xff}@{hex(p)}")
            if (hw & 0xFF) == 2:
                # check following STRB #0
                for q in range(p + 2, min(p + 16, len(img) - 2), 2):
                    h2 = u16(q)
                    if (h2 & 0xF800) == 0x7000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == ((hw >> 8) & 7):
                        present2_sites.append((s, p, q))
                    if (h2 & 0xFFF0) == 0xF880:
                        h2b = u16(q + 2)
                        if (h2b & 0xFFF) == 0 and ((h2b >> 12) & 0xF) == ((hw >> 8) & 7):
                            present2_sites.append((s, p, q))
        b = bl(p)
        if b in (FN_A, STATUS_WRAP, SET_APP, STATUS):
            notes.append(f"BL->{hex(b)}@{hex(p)}")
    log(f"  {hex(s)} r{rd}: {notes[:18]}")

log(f"\nPresent=2 pattern (#636c + MOVS#2 + STRB#0): n={len(present2_sites)}")
for x in present2_sites:
    log(f"  #636c@{hex(x[0])} MOVS2@{hex(x[1])} STRB@{hex(x[2])} in_FN_A={0x14F692C<=x[2]<=0x14F6B00}")

# B) +0xBF6 / BF4 / BF5 writers
log("\n=== B) STR*.W +0xBF4/BF5/BF6 ===")
for name, off in (("BF4", 0xBF4), ("BF5", 0xBF5), ("BF6", 0xBF6)):
    hits = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) in (0xF880, 0xF8A0, 0xF8C0) and (hw2 & 0xFFF) == off:
            kind = {0xF880: "STRB.W", 0xF8A0: "STRH.W", 0xF8C0: "STR.W"}[hw & 0xFFF0]
            hits.append((o, kind, hw & 0xF, (hw2 >> 12) & 0xF))
        o += 2
    log(f"  +0x{name}: n={len(hits)}")
    for h in hits:
        log(f"    {hex(h[0])} {h[1]} r{h[3]},[r{h[2]},#0x{name}]")

# C) BL FN_A / STATUS_WRAP
log("\n=== C) BL->FN_A / STATUS_WRAP ===")
for lab, tgt in (("FN_A", FN_A), ("STATUS_WRAP", STATUS_WRAP)):
    hits = scan_bl(tgt)
    log(f"  {lab}: n={len(hits)} {[hex(x) for x in hits]}")

# D) SIM_INIT strings + refs + dump
log("\n=== D) SIM_INIT / USIM bring-up strings ===")
needles = [
    b"Waiting for SIM_INIT_REQ",
    b"USIM <== SIM_INIT_REQ",
    b"USIM ==> SIM_INIT_REQ",
    b"SIM_INIT_REQ",
    b"USIM_WAIT_FOR_INIT_REQ",
    b"START_STACK_SERVICES",
    b"SIM_START_STACK_SERVICES",
    b"USIM ==> SIM_START_IND",
    b"SIM_PRESENT_IND",
    b"SIM_PIN_STATUS_IND",
    b"USIM_CARD_PRESENT",
    b"SIM_INIT_CNF",
    b"SIM_INIT_COMPLETE",
    b"NS_SIM_INIT",
    b"SimInitReq",
    b"CARD_POWER_ON",
    b"SIM_POWER_ON",
    b"PowerOnCard",
    b"USIM_ATR",
    b"SELECT_USIM",
    b"SimAppInit",
    b"SIM_APP_INIT",
    b"InitSimApplication",
    b"USIM_APP_READY",
    b"USIM_READY",
    b"USIM_INIT_REQ_RCVD",
    b"USIM_APP_DETECTED",
    b"USIM_PIN_REQUIRED",
    b"USIM_READY_STATE",
    b"HotSwap",
    b"HOT_SWAP",
    b"PHONE_START",
    b"STACK_START",
    b"StartPhone",
]
for n in needles:
    off = img.find(n)
    if off < 0:
        log(f"  {n!r}: MISSING")
        continue
    va = va_of(off)
    refs = refs_va(va, limit=16)
    log(f"  {n!r}: off={hex(off)} VA={hex(va)} refs={len(refs)} {[hex(x) for x in refs[:10]]}")

# Detailed dumps for key wait / init strings
for key in (b"Waiting for SIM_INIT_REQ", b"USIM <== SIM_INIT_REQ", b"START_STACK_SERVICES", b"SIM_PRESENT_IND"):
    off = img.find(key)
    if off < 0:
        continue
    refs = refs_va(va_of(off), limit=8)
    for r in refs[:3]:
        dump(r - 0x40, r + 0xA0, f"{key.decode()}@{hex(r)}")
        # Does this function touch Present (#636c) or SET_APP / FN_A?
        window = range(max(0, r - 0x200), min(len(img) - 4, r + 0x200), 2)
        touches = []
        for p in window:
            mw = movw(p)
            if mw and mw[0] == PRESENT_ID:
                touches.append(f"#636c@{hex(p)}")
            b = bl(p)
            if b in (FN_A, SET_APP, STATUS, STATUS_WRAP):
                touches.append(f"BL->{hex(b)}@{hex(p)}")
            if (u16(p) & 0xFFF0) == 0xF880 and (u16(p + 2) & 0xFFF) == 0xBF6:
                touches.append(f"STRB +BF6@{hex(p)}")
        log(f"  touches_near: {touches[:20]}")

# E) All SIM_INIT context strings
log("\n=== E) All 'SIM_INIT' occurrences (context) ===")
pos = 0
n = 0
while n < 80:
    i = img.find(b"SIM_INIT", pos)
    if i < 0:
        break
    log(f"  @{hex(i)}: {cstr(max(0, i - 32), 120)!r}")
    pos = i + 8
    n += 1
log(f"  total_shown={n}")

# F) SET#1 DETECTED — Present nearby?
log("\n=== F) SET_APP callers with imm#1 (DETECTED) ===")
for c in scan_bl(SET_APP):
    imm = None
    for p in range(c - 2, max(0, c - 0x50), -2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000:
            # only if into r0-ish; MOVS encodes Rd in bits
            imm = hw & 0xFF
            rd = (hw >> 8) & 7
            if rd == 0:
                break
            imm = None
        mw = movw(p)
        if mw and mw[1] == 0:
            imm = mw[0]
            break
    if imm == 1:
        log(f"  SET#1 @{hex(c)}")
        has636 = any(
            (movw(q) and movw(q)[0] == PRESENT_ID)
            for q in range(max(0, c - 0x200), c + 0x80, 2)
        )
        log(f"    #636c nearby={has636}")
        dump(c - 0x50, c + 0x10, f"SET1@{hex(c)}")

# G) STATUS Present load (confirm)
log("\n=== G) STATUS Present path (snippet) ===")
dump(0x14FB360, 0x14FB5E0, "STATUS_READY_GATE")

# H) memcpy-like: BL after getobj#636c with size small (heuristic MOVS #1/#4)
log("\n=== H) After #636c: BL targets (possible memcpy/helper) ===")
# Find getobj by scanning BL after MOVW #636c within 0x20
bl_targets = {}
for s, rd in sites:
    for p in range(s, min(s + 0x40, len(img) - 4), 2):
        b = bl(p)
        if b is not None:
            bl_targets.setdefault(b, []).append(p)
for tgt, callers in sorted(bl_targets.items(), key=lambda x: -len(x[1]))[:25]:
    log(f"  BL->{hex(tgt)} from #636c-window n={len(callers)} e.g. {[hex(x) for x in callers[:4]]}")

# I) sit-stream builders for SIM init-ish
log("\n=== I) sit-stream SIM init-ish ===")
for sitp in (
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"),
    Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/sit-stream.so"),
):
    if sitp.exists():
        s = sitp.read_bytes()
        log(f"sit={sitp} size={len(s)}")
        for needle in (
            b"SIM_INIT",
            b"StartStack",
            b"START_STACK",
            b"SimRefresh",
            b"BuildSimRefresh",
            b"BuildSimGetATR",
            b"BuildSimStatus",
            b"BuildSimCardPower",
            b"BuildSetUiccSubscription",
            b"BuildSimOpenChannel",
            b"BuildSimCloseChannel",
            b"BuildGetSimAuth",
            b"BuildSimTransmitApdu",
            b"FileUpdate",
            b"SimFile",
            b"PhoneStart",
            b"NetStart",
        ):
            off = s.find(needle)
            log(f"  {needle!r}: {off>=0} {hex(off) if off>=0 else ''}")
        break
else:
    log("sit-stream.so missing")

# J) Soft-path skip: what does CP log when waiting — is SIM_INIT from FMT/IPC?
log("\n=== J) Nearby IPC/FMT/MBX strings within 0x100 of Waiting for SIM_INIT_REQ ===")
wait = img.find(b"Waiting for SIM_INIT_REQ")
if wait >= 0:
    window = img[max(0, wait - 0x200) : wait + 0x200]
    for n in (b"FMT", b"IPC", b"MBX", b"mailbox", b"SIT", b"RFS", b"NV", b"cbd", b"AP", b"REQ"):
        if n in window:
            log(f"  found {n!r} in +/-0x200 of Waiting")

log("\nDONE")
