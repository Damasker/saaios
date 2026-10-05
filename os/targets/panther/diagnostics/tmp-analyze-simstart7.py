#!/usr/bin/env python3
"""INIT_REQ / PIN_STATUS / SET_APP near USIM; A[] log-id packing for START strings."""
import struct
from pathlib import Path
from collections import defaultdict

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A, FN_B, WRAP_SIM, SET_APP = 0x14F692C, 0x14F9108, 0x14F6D02, 0x19916D2
TARGETS = {FN_A, FN_B, WRAP_SIM, SET_APP, 0x14C380E, 0x14C3986}


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


def scan(lo, hi):
    hits = []
    for o in range(lo, min(hi, END - 4), 2):
        bt = bl_target(o)
        if bt in TARGETS:
            hits.append((o, f"BL->{hex(bt)}"))
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) in (0xBF4, 0xBF5, 0xBF6, 0x18E):
            hits.append((o, f"STRB.W #{hex(hw2 & 0xFFF)}"))
    return hits


def dump(o, before=0x40, after=0x60):
    lines = []
    for p in range(o - before, o + after, 2):
        if p < MAIN_OFF or p >= END - 4:
            continue
        note = []
        r, t, bt = movw(p), movt(p), bl_target(p)
        hw = u16(p)
        if r:
            note.append(f"MOVW r{r[1]},#{hex(r[0])}")
        if t:
            note.append(f"MOVT r{t[1]},#{hex(t[0])}")
        if bt is not None:
            note.append(f"BL->{hex(bt)}" + (" *" if bt in TARGETS else ""))
        if (hw & 0xFF00) == 0x2000:
            note.append(f"MOVS r{(hw>>8)&7},#{hw&0xFF}")
        if (hw & 0xFFF0) == 0xF880:
            note.append(f"STRB.W #{hex(u16(p+2)&0xFFF)}")
        if note:
            lines.append(f"  {hex(p)}: " + " | ".join(note))
    return "\n".join(lines)


# Shannon A[] strings often preceded by a 2-byte or 4-byte log id in a table.
# Search for pointer tables: many A[USIM] strings live at 0x4d0xxxx — find .word VA
# But earlier VA xrefs failed. Alternative: log id is embedded as short before string
# or in parallel table. Try: find 0x105a sites we know, see pattern of how format is bound.

print("=== Known 0x105a USIM sites dump (PIN path?) ===")
for site, ctx, reg in real_log_sites(0x105A):
    if ctx != 0x4107:
        continue
    print(f"\n## {hex(site)} reg=r{reg}")
    print(dump(site, 0x30, 0x50))
    print("hits", scan(site - 0x100, site + 0x200))

# Find log ids used near SET_APP callers in USIM
print("\n=== SET_APP callers (all) ===")
set_callers = []
for o in range(MAIN_OFF, END - 4, 2):
    if bl_target(o) == SET_APP:
        set_callers.append(o)
print([hex(x) for x in set_callers])
for o in set_callers:
    # collect nearby MOVW immediates that look like log ids / states
    imms = []
    for p in range(o - 0x40, o + 0x20, 2):
        r = movw(p)
        if r:
            imms.append((hex(p), hex(r[0]), f"r{r[1]}"))
    print(f"\nSET_APP @{hex(o)}")
    print(dump(o, 0x50, 0x30))
    print("hits±0x200", scan(o - 0x200, o + 0x100))

# SIM_INIT_REQ — find related log strings with A[USIM] and their sites via 0x5xx style
print("\n=== SIM_INIT / PIN_STATUS A[USIM] strings ===")
for needle in (
    b"SIM_INIT_REQ",
    b"PIN_STATUS",
    b"PrepareSimPin",
    b"SimPinStatus",
    b"Pin1Status",
    b"SIM_START_IND",
    b"START_STACK_SERVICES",
    b"sitInformSimInit",
    b"InformSimInit",
    b"SimStartInd",
    b"SendStartInd",
    b"SIM START IND",
):
    for off, s in find_str(needle):
        if s.startswith("A[") or "USIM" in s or "SIT" in s or "SIM START" in s:
            print(f"  @{hex(off)}: {s[:100]}")

# Try packing: A-string at 0x4d4888b — search for relative offset from a known base
# Many Shannon builds: log descriptor {id, fmt_ptr}. Search 8-byte structs with fmt VA.
print("\n=== Descriptor search for SIM START format / Prepare / INIT strings ===")
for off, s in [
    find_str(b"SIM START IND (SimPresent")[0],
    find_str(b"PrepareSimStartIndParameters")[0],
    find_str(b"sitInformSimInit()")[0],
    find_str(b"USIM <== SIM_INIT_REQ")[0],
    find_str(b"Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ")[0],
]:
    v = va(off)
    needle = struct.pack("<I", v)
    # also try file offset itself as some tables use file-relative
    hits = []
    i = 0
    while True:
        j = img.find(needle, i, min(len(img), END + 0x100000))
        if j < 0:
            break
        hits.append(j)
        i = j + 1
    print(f"\n{s[:60]}")
    print(f"  VA={hex(v)} desc_hits={list(map(hex, hits[:8]))} n={len(hits)}")
    for h in hits[:4]:
        # dump u16/u32 before pointer (possible log id)
        prev = [hex(u16(h - 4)), hex(u16(h - 2)), hex(u32(h - 8)), hex(u32(h - 4))]
        print(f"  @{hex(h)} prev16={prev[:2]} prev32={prev[2:]}")

# Present=2 STRB sites — confirm none in START builders by scanning all Present=2 writers again
print("\n=== All MOVS#2 + STRB Present-object patterns near known Present builders ===")
# Known Present STRB at 0x14f6a14 and 0x14f9578 — already documented
# Search STRB.W #0 / #1 / #2 / #3 to [rN,#0] where rN points at Present byte
# Broader: any BL FN_A from entire MAIN (confirm only 2)
print("FN_A BLs:", end=" ")
fa = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == FN_A]
print([hex(x) for x in fa])
print("FN_B BLs:", [hex(x) for x in range(MAIN_OFF, END - 4, 2) if bl_target(x) == FN_B][:20])
# Restrict: only count if previous 4 bytes look like valid thumb (heuristic already via bl)

# Rx path: USIM <== SIM_INIT_REQ — message name pool
print("\n=== INIT_REQ name pool + neighbors ===")
off = find_str(b"USIM <== SIM_INIT_REQ")[0][0]
v = va(off)
pools = []
i = 0
nb = struct.pack("<I", v)
while True:
    j = img.find(nb, i)
    if j < 0:
        break
    pools.append(j)
    i = j + 1
print(f"INIT_REQ pools={list(map(hex, pools))}")
for po in pools:
    for k in range(-8, 10):
        w = u32(po + k * 4)
        if 0x41020000 <= w <= 0x41080000:
            fo = MAIN_OFF + (w - VA_BASE)
            b = fo
            while b < fo + 70 and 32 <= img[b] < 127:
                b += 1
            s2 = img[fo:b].decode("ascii", "replace")
            if "SIM_" in s2 or "USIM" in s2:
                print(f"  [{k:+d}] {s2}")

# START_STACK_SERVICES_REQ
print("\n=== START_STACK_SERVICES / after-START AP REQs ===")
for needle in (
    b"START_STACK_SERVICES",
    b"SIM_START_PBM",
    b"STACK_SERVICES_REQ",
    b"USIM <== SIM_START",
    b"after SIM START",
    b"Before sending SIM_START",
    b"Send SIM_START",
    b"sending SIM_START",
    b"SIM_PIN_STATUS",
):
    for off, s in find_str(needle):
        print(f"  @{hex(off)}: {s[:110]}")

print("\nDONE")
