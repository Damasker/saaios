#!/usr/bin/env python3
"""Locate SIM START / PIN_STATUS builders via format-string log IDs; INIT_REQ order."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A, FN_B, WRAP_SIM, SET_APP = 0x14F692C, 0x14F9108, 0x14F6D02, 0x19916D2
TARGETS = {FN_A, FN_B, WRAP_SIM, SET_APP, 0x14C380E, 0x14C3986, 0x14F6A14, 0x14F9578, 0x1991734}


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def bl_target(o):
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


def find_str(needle: bytes):
    out = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        out.append((a, img[a:b].decode("ascii", "replace")))
        i = j + 1
    return out


def log_id_for_format_string(soff):
    """Shannon often packs: log table entry points at format string.
    Heuristic: search for .word VA(soff) in MAIN, then nearby MOVW small ids.
    Also: string may be in a table indexed by log id — search litpools near code.
    """
    target = va(soff)
    needle = struct.pack("<I", target)
    pools = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        pools.append(j)
        i = j + 1
    return pools


def real_log_sites(log_id):
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != log_id:
            continue
        dense = False
        for p in range(o + 4, o + 28, 2):
            r2 = movw(p)
            if r2 and abs(r2[0] - log_id) <= 2:
                dense = True
                break
        if dense:
            continue
        ctx = None
        for p in range(o - 24, o + 28, 2):
            t = movt(p)
            if t and 0x4000 <= t[0] <= 0x45FF:
                ctx = t[0]
                break
        if ctx is None:
            continue
        hits.append((o, ctx, r[1]))
    return hits


def scan_fn(lo, hi):
    hits = []
    bls = {}
    for o in range(lo, hi - 4, 2):
        t = bl_target(o)
        if t:
            bls[t] = bls.get(t, 0) + 1
            if t in TARGETS:
                hits.append((o, f"BL->{hex(t)}"))
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880:
            imm = hw2 & 0xFFF
            if imm in (0xBF4, 0xBF5, 0xBF6, 0x18E):
                hits.append((o, f"STRB.W #{hex(imm)}"))
        # MOVS r0,#2 followed by STRB somewhere
        if hw == 0x2002:  # movs r0,#2
            for p in range(o + 2, min(o + 24, hi - 2), 2):
                h = u16(p)
                if (h & 0xF800) == 0x7000:
                    hits.append((o, f"MOVS#2 STRB [r{(h>>3)&7},#{(h>>6)&0x1f}] @{hex(p)}"))
                    break
                if (h & 0xFFF0) == 0xF880 and (u16(p + 2) & 0xFFF) in (0xBF6, 0xBF5, 0, 1, 20):
                    hits.append((o, f"MOVS#2 STRB.W #{hex(u16(p+2)&0xFFF)} @{hex(p)}"))
                    break
                bt = bl_target(p)
                if bt in TARGETS:
                    hits.append((o, f"MOVS#2 .. BL {hex(bt)} @{hex(p)}"))
                    break
    return hits, bls


def dump_window(o, before=0x30, after=0x60):
    lines = []
    for p in range(o - before, o + after, 2):
        if p < MAIN_OFF or p >= END - 4:
            continue
        note = []
        r = movw(p)
        t = movt(p)
        bt = bl_target(p)
        hw = u16(p)
        if r:
            note.append(f"MOVW r{r[1]},#{hex(r[0])}")
        if t:
            note.append(f"MOVT r{t[1]},#{hex(t[0])}")
        if bt is not None:
            tag = " ***" if bt in TARGETS else ""
            note.append(f"BL->{hex(bt)}{tag}")
        if (hw & 0xFF00) == 0x2000:
            note.append(f"MOVS r{(hw>>8)&7},#{hw&0xFF}")
        if (hw & 0xFFF0) == 0xF880:
            note.append(f"STRB.W #{hex(u16(p+2)&0xFFF)}")
        if note:
            lines.append(f"  {hex(p)}: " + " | ".join(note))
    return "\n".join(lines)


# --- 1. Format string SIM START IND (SimPresent=...) ---
print("=== SIM START IND format strings → log sites ===")
for off, s in find_str(b"SIM START IND (SimPresent"):
    print(f"\n@{hex(off)} VA={hex(va(off))}: {s}")
    pools = log_id_for_format_string(off)
    print(f"  litpools: {list(map(hex, pools))}")
    # Shannon A[ ] logs often: the format string VA is passed via MOVW/MOVT directly
    lo, hi = va(off) & 0xFFFF, (va(off) >> 16) & 0xFFFF
    print(f"  VA lo={hex(lo)} hi={hex(hi)}")
    # Find MOVW #lo + MOVT #hi same register within 16 bytes
    xrefs = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != lo:
            continue
        reg = r[1]
        for p in range(max(MAIN_OFF, o - 16), min(END - 4, o + 20), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == reg:
                xrefs.append(o)
                break
    print(f"  MOVW+MOVT xrefs ({len(xrefs)}): {list(map(hex, xrefs[:20]))}")
    for xo in xrefs[:8]:
        print(f"\n  --- xref {hex(xo)} ---")
        print(dump_window(xo, 0x40, 0x80))
        # scan ±0x300 for Present builders
        hits, _ = scan_fn(xo - 0x300, xo + 0x400)
        print(f"  present-ish hits: {[(hex(a),b) for a,b in hits]}")

# --- 2. PIN_STATUS format / prepare ---
print("\n=== PIN_STATUS related format / prepare strings ===")
for needle in (
    b"PIN_STATUS_IND",
    b"PinStatus",
    b"PrepareSimPin",
    b"SimPinStatus",
    b"PIN STATUS",
    b"Pin1Status",
    b"sitInformSimInit",
    b"USIM_WAIT_FOR_INIT_REQ",
    b"WAIT_FOR_INIT",
    b"INIT_REQ",
    b"SimInit",
    b"SIM_INIT_REQ",
    b"USIM_CARD_PRESENT",
    b"Sending SIM_START",
    b"SendSimStart",
    b"SIM_START_IND",
):
    for off, s in find_str(needle.encode() if isinstance(needle, str) else needle):
        if len(s) > 120:
            s = s[:120]
        print(f"  @{hex(off)}: {s}")

# --- 3. Follow name-table litpool for START/PIN_STATUS (msg dispatch) ---
print("\n=== Name-table pool consumers (wide search LDR.W) ===")
for label, soff in (
    ("START", find_str(b"USIM ==> SIM_START_IND")[0][0]),
    ("PIN_STATUS", find_str(b"USIM ==> SIM_PIN_STATUS_IND")[0][0]),
    ("PRESENT", find_str(b"USIM ==> SIM_PRESENT_IND")[0][0]),
):
    target = va(soff)
    needle = struct.pack("<I", target)
    pools = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        pools.append(j)
        i = j + 1
    print(f"\n{label} VA={hex(target)} pools={list(map(hex, pools))}")
    for po in pools:
        # dump 32 words around pool — often a message-id table
        print(f"  pool@{hex(po)} neighbors:")
        for k in range(-4, 5):
            w = u32(po + k * 4)
            # try as VA→string
            if 0x41000000 <= w <= 0x45000000:
                fo = MAIN_OFF + (w - VA_BASE)
                if 0 <= fo < len(img) - 8:
                    # read cstring
                    b = fo
                    while b < fo + 80 and 32 <= img[b] < 127:
                        b += 1
                    s = img[fo:b].decode("ascii", "replace")
                    if s:
                        print(f"    [{k}] {hex(w)} -> {s[:70]}")
                    else:
                        print(f"    [{k}] {hex(w)}")
                else:
                    print(f"    [{k}] {hex(w)}")
            else:
                print(f"    [{k}] {hex(w)}")

# --- 4. Log sites for PrepareSimStart (0x5b5) USIM ctx — full fn covering 0x1a6d ---
print("\n=== Deep dump Prepare @0x1a6d142 (MOVS#2 path) ===")
print(dump_window(0x1A6D19C, 0x80, 0x40))
hits, bls = scan_fn(0x1A6D100, 0x1A6D300)
print("hits", [(hex(a), b) for a, b in hits])
# What is BL 0x20e184e with r0=2?
print("\n=== 0x20e184e with args — is it SET state? ===")
# Look at callers that MOVS r0,#2 just before BL 0x20e184e
callers2 = []
callers5 = []
for o in range(MAIN_OFF, END - 4, 2):
    if bl_target(o) != 0x20E184E:
        continue
    # look back 12 bytes for movs r0,#imm
    arg = None
    for p in range(o - 12, o, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) == 0:
            arg = hw & 0xFF
    if arg == 2:
        callers2.append(o)
    if arg == 5:
        callers5.append(o)
print(f"BL 0x20e184e with r0=#2 count={len(callers2)} sample={list(map(hex, callers2[:15]))}")
print(f"BL 0x20e184e with r0=#5 count={len(callers5)} sample={list(map(hex, callers5[:15]))}")
# Check if any #2 caller is near FN_A or writes Present
for o in callers2[:20]:
    hits, _ = scan_fn(o - 0x80, o + 0x40)
    if hits:
        print(f"  {hex(o)} nearby: {[(hex(a),b) for a,b in hits]}")

# --- 5. INIT_REQ log sites ---
print("\n=== USIM_WAIT_FOR_INIT_REQ / sitInformSimInit log xrefs ===")
for needle in (
    b"USIM_WAIT_FOR_INIT_REQ",
    b"sitInformSimInit",
    b"Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ",
    b"Sending SIM_PRESENT_IND to PBM from USIM_CARD_PRESENT",
    b"PrepareSimStartIndParameters",
):
    for off, s in find_str(needle):
        lo, hi = va(off) & 0xFFFF, (va(off) >> 16) & 0xFFFF
        xrefs = []
        for o in range(MAIN_OFF, END - 8, 2):
            r = movw(o)
            if not r or r[0] != lo:
                continue
            reg = r[1]
            for p in range(max(MAIN_OFF, o - 16), min(END - 4, o + 20), 2):
                t = movt(p)
                if t and t[0] == hi and t[1] == reg:
                    xrefs.append(o)
                    break
        print(f"\n{s[:70]}")
        print(f"  @{hex(off)} VA={hex(va(off))} xrefs={list(map(hex, xrefs[:12]))}")
        for xo in xrefs[:4]:
            hits, _ = scan_fn(xo - 0x200, xo + 0x200)
            print(f"  xref {hex(xo)} present-hits={[(hex(a),b) for a,b in hits[:8]]}")

# --- 6. Wider USIM region 0x1916000-0x1919000 for FN_A/Present ---
print("\n=== Full scan USIM 0x1916000..0x1919000 ===")
hits, bls = scan_fn(0x1916000, 0x1919000)
print("hits", [(hex(a), b) for a, b in hits])
targ = {hex(t): bls[t] for t in TARGETS if t in bls}
print("target BLs", targ)

# --- 7. Does PIN_STATUS Tx path call Present=2? Search log id near PIN_STATUS name usage ---
# Message names often logged with a common helper taking name ptr — already have litpool.
# Try: find code that LDR from pool 0x10fb2bc / 0x10fb87c via ADR/LDR.W literal
print("\n=== LDR.W PC-rel to PIN_STATUS / START pools ===")
for label, po in (("PIN", 0x10FB2BC), ("START", 0x10FB87C), ("PRESENT", 0x10FB2E8)):
    found = []
    for o in range(MAIN_OFF, END - 4, 2):
        hw, hw2 = u16(o), u16(o + 2)
        # LDR.W Rt,[PC,#imm12] encoding T2: F8DF
        if hw == 0xF8DF:
            imm12 = hw2 & 0xFFF
            rt = (hw2 >> 12) & 0xF
            pc = (o + 4) & ~3
            if pc + imm12 == po:
                found.append(o)
        # LDR Rt,[PC,#imm8] T1
        if (hw & 0xF800) == 0x4800:
            imm8 = hw & 0xFF
            pc = (o + 4) & ~3
            if pc + imm8 * 4 == po:
                found.append(o)
    print(f"{label} pool@{hex(po)} LDR sites={list(map(hex, found[:20]))} count={len(found)}")
    for fo in found[:3]:
        print(dump_window(fo, 0x20, 0x50))
        hits, _ = scan_fn(fo - 0x100, fo + 0x200)
        print(f"  hits={[(hex(a),b) for a,b in hits]}")
