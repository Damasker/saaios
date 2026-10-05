#!/usr/bin/env python3
"""Deeper: who produces USIM SIM_INIT_REQ — GMC parallel, schedule timer, send helpers.

Follow START_STACK GMC path as template; hunt SIM_INIT equivalents.
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


def off_of(va):
    return va - VA + MAIN


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
            if (hw & 0xF800) == 0x4800:
                pass
            log(f" {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
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


def find_movw_movt_va(target_va, band=(0x1000000, 0x3C00000)):
    lo, hi = target_va & 0xFFFF, (target_va >> 16) & 0xFFFF
    refs = []
    o = band[0]
    while o < band[1] - 8:
        r = movw(o)
        if r and r[0] == lo:
            # MOVT may be immediate next or within 0x20
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


log(f"size={len(img)}")

# 1) GMC START_STACK string @ ptr 0x19f7e8 — dump table + code xrefs to that ptr slot / string VA
gmc_off = img.find(b"GMC ==> SIM__ [START_STACK_SERVICES_REQ]")
gmc_va = va_of(gmc_off)
log(f"\n=== GMC START_STACK str off={hex(gmc_off)} VA={hex(gmc_va)} ===")
dump(0x19F7C0, 0x19F820, "gmc_ptr_table")
# code that loads this VA
gmc_refs = find_movw_movt_va(gmc_va, (0x1800000, 0x1C00000))
log(f"MOVW/MOVT GMC str in 0x18-0x1c: n={len(gmc_refs)} {[hex(x) for x in gmc_refs[:20]]}")
for r in gmc_refs[:6]:
    dump(r - 0x40, r + 0x80, f"gmc_log@{hex(r)}")
    # nearby bare 0x2f58?
    for p in range(r - 0x100, r + 0x100, 2):
        mw = movw(p)
        if mw and mw[0] in (0x2F50, 0x2F58, 0x2F57, 0x2FA7):
            log(f"  nearby msgid MOVW #{hex(mw[0])}@{hex(p)} r{mw[1]}")

# Also LDR PC of ptr table entry 0x19f7e8
log("\n=== Who LDRs ptr@0x19f7e8 ===")
# scan for MOVW/MOVT of 0x4019f7e8? table is file-off; VA of table = va_of(0x19f7e8)
tbl_va = va_of(0x19F7E8)
tbl_refs = find_movw_movt_va(tbl_va, (0x1400000, 0x2200000))
log(f"table VA={hex(tbl_va)} refs={len(tbl_refs)} {[hex(x) for x in tbl_refs[:15]]}")

# 2) USIM_SCHEDULE_SIM_INIT_TIMER ptr @0x1104580
sched_off = img.find(b"USIM_SCHEDULE_SIM_INIT_TIMER")
sched_va = va_of(sched_off)
log(f"\n=== SCHEDULE_SIM_INIT_TIMER off={hex(sched_off)} VA={hex(sched_va)} ===")
dump(0x1104540, 0x11045C0, "sched_state_table")
# surrounding state names
for o in range(0x1104500, 0x1104600, 4):
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 50) if 0 <= so < len(img) else None
    if s:
        log(f"  {hex(o)}: {s}")

# 3) SIM_INIT_REQ ptr @0x6de744
log("\n=== SIM_INIT_REQ extra ptr @0x6de744 ===")
dump(0x6DE700, 0x6DE780, "sim_init_ptr_nbhd")
for o in range(0x6DE700, 0x6DE780, 4):
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 60) if 0 <= so < len(img) else None
    log(f"  {hex(o)}: {hex(w)} {s!r}")

# 4) Search all strings containing SIM_INIT
log("\n=== All SIM_INIT* strings ===")
pos = 0
while True:
    i = img.find(b"SIM_INIT", pos)
    if i < 0:
        break
    log(f"  {hex(i)} VA={hex(va_of(i))}: {cstr(max(0,i-20), 90)!r}")
    pos = i + 1

# 5) Search "==> " near SIM / USIM init messaging
log("\n=== '==> SIM' / '<== SIM' / 'SIM__' strings ===")
for key in (b"==> SIM", b"<== SIM", b"SIM__", b"SIM_INIT", b"InitReq", b"INIT_REQ"):
    pos = 0
    n = 0
    while n < 40:
        i = img.find(key, pos)
        if i < 0:
            break
        ctx = cstr(max(0, i - 25), 100)
        if ctx and ("SIM" in ctx or "USIM" in ctx or "GMC" in ctx or "INIT" in ctx):
            log(f"  {hex(i)}: {ctx!r}")
            n += 1
        pos = i + 1

# 6) Dump interesting bare sites 0x17af936 / 0x1bab5aa fully
for site in (0x17AF936, 0x1BAB5AA, 0x2EC25F0):
    dump(site - 0x60, site + 0x80, f"bare_focus@{hex(site)}")

# 7) Generic send: find functions that take msgid and destination task USIM
# Look for string "USIM_TASK" / "USIM" task id send wrappers
log("\n=== Task / send related strings ===")
for key in (
    b"USIM_TASK",
    b"SendMessageToUsim",
    b"usim_Send",
    b"PalMsgSend",
    b"OsSendMsg",
    b"sa_msg_send",
    b"MSG_SEND",
    b"SendToUsim",
    b"gmc_send",
    b"GMC_Send",
    b"SIM_INIT_REQ",
    b"STACK_SERVICES",
):
    off = img.find(key)
    if off >= 0:
        log(f"  {key!r}: {hex(off)} {cstr(off, 60)!r}")
    else:
        log(f"  {key!r}: MISSING")

# 8) Soft CPIF: existing SIT that might cause CP to emit INIT
# CardPower / GetSimStatus / VerifyPin already live-neg; look for sit builders mentioning Init
log("\n=== sit*Init* / SimInit strings in MAIN ===")
pos = 0
while True:
    i = img.find(b"sit", pos)
    if i < 0 or i > 0x5000000:
        break
    s = cstr(i, 50)
    if s and ("Init" in s or "init" in s) and ("Sim" in s or "SIM" in s or "Usim" in s or "USIM" in s):
        log(f"  {hex(i)}: {s!r}")
    pos = i + 1
    if pos > 0x5000000:
        break

# 9) Non-STRB Present wide hunt: STM of {2,?,?} or STR.W of 0x00000002 to [getobj+0]
# Search MOVS#2 + STR.W [Rn,#0] anywhere in USIM band near getobj calls
log("\n=== USIM band 0x14f0000-0x1b00000: MOVS#2 + STR.W #0 (word store Present?) ===")
hits = []
o = 0x14F0000
while o < 0x1B00000 - 8:
    hw = u16(o)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        rt = (hw >> 8) & 7
        for q in range(o + 2, min(o + 12, 0x1B00000 - 4), 2):
            h2 = u16(q)
            # STR.W Rt,[Rn,#imm12] encoding F8C0
            if (h2 & 0xFFF0) == 0xF8C0:
                h22 = u16(q + 2)
                if ((h22 >> 12) & 0xF) == rt and (h22 & 0xFFF) == 0:
                    hits.append((o, q))
            # STR Rt,[Rn,#0] word T2
            if (h2 & 0xF800) == 0x6000 and ((h2 >> 6) & 0x1F) == 0 and (h2 & 7) == rt:
                hits.append((o, q))
    o += 2
log(f"n={len(hits)}")
for a, b in hits[:40]:
    # check #636c within ±0x100
    has636 = False
    for p in range(max(0x14F0000, a - 0x100), min(a + 0x100, 0x1B00000 - 4), 2):
        mw = movw(p)
        if mw and mw[0] == 0x636C:
            has636 = True
    log(f"  MOVS2@ {hex(a)} STR@{hex(b)} near#636c={has636}")

# 10) memcpy template: MOVS r1,#2 / MOVS r2,#1 BL memcpy near #636c — rare; scan BL targets named
log("\n=== Near #636c: MOVS#2 within 0x40 of BL (possible fill/memcpy) outside FN_A ===")
FN_A = 0x14F692C
sites = []
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == 0x636C:
        sites.append(o)
    o += 2
for s in sites:
    for p in range(s, min(s + 0x100, len(img) - 4), 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            # any BL in next 0x20?
            for q in range(p, min(p + 0x20, len(img) - 4), 2):
                b = bl(q)
                if b is not None and not (FN_A <= p <= FN_A + 0x200):
                    log(f"  #636c@{hex(s)} MOVS2@{hex(p)} BL@{hex(q)}->{hex(b)}")

log("\nDONE")
