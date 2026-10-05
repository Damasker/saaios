#!/usr/bin/env python3
"""Fast RO: SIM START / PIN_STATUS builders vs Present=2; INIT_REQ order."""
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
TARGETS = {FN_A, FN_B, WRAP_SIM, SET_APP, 0x14C380E, 0x14C3986, 0x14F6A14, 0x14F9578}


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


print("Indexing MOVW by imm (MAIN)...", flush=True)
movw_index = defaultdict(list)
movt_index = defaultdict(list)
bl_index = defaultdict(list)
for o in range(MAIN_OFF, END - 4, 2):
    r = movw(o)
    if r:
        movw_index[r[0]].append((o, r[1]))
    t = movt(o)
    if t:
        movt_index[t[0]].append((o, t[1]))
    bt = bl_target(o)
    if bt in TARGETS or bt == 0x20E184E:
        bl_index[bt].append(o)
print(
    f"done. movw keys={len(movw_index)} target BLs={ {hex(k):len(v) for k,v in bl_index.items()} }",
    flush=True,
)


def va_xrefs(soff):
    """MOVW+MOVT pairs building VA(soff)."""
    v = va(soff)
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    xrefs = []
    for o, reg in movw_index.get(lo, []):
        # look for matching MOVT nearby
        for p, r2 in movt_index.get(hi, []):
            if r2 == reg and abs(p - o) <= 16:
                xrefs.append(o)
                break
    return xrefs, lo, hi


def scan_range(lo, hi):
    hits = []
    for o in range(lo, min(hi, END - 4), 2):
        bt = bl_target(o)
        if bt in TARGETS:
            hits.append((o, f"BL->{hex(bt)}"))
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) in (0xBF4, 0xBF5, 0xBF6, 0x18E):
            hits.append((o, f"STRB.W #{hex(hw2 & 0xFFF)}"))
        if hw == 0x2002:  # movs r0,#2
            for p in range(o + 2, min(o + 20, END - 2), 2):
                h = u16(p)
                if (h & 0xF800) == 0x7000:
                    imm5 = (h >> 6) & 0x1F
                    if imm5 <= 24:
                        hits.append((o, f"MOVS#2 STRB [r{(h>>3)&7},#{imm5}]"))
                    break
                if (h & 0xFFF0) == 0xF880 and (u16(p + 2) & 0xFFF) in (0xBF6, 0xBF5):
                    hits.append((o, f"MOVS#2 STRB.W #{hex(u16(p+2)&0xFFF)}"))
                    break
                if bl_target(p) in TARGETS:
                    hits.append((o, f"MOVS#2 BL->{hex(bl_target(p))}"))
                    break
    return hits


def dump(o, before=0x28, after=0x50):
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


# 1) Format string for SIM START IND payload
print("\n=== SIM START IND (SimPresent=...) ===", flush=True)
for off, s in find_str(b"SIM START IND (SimPresent"):
    print(f"@{hex(off)}: {s}")
    xrefs, lo, hi = va_xrefs(off)
    print(f"  VA={hex(va(off))} xrefs={list(map(hex, xrefs))}")
    for xo in xrefs:
        print(f"\n-- builder xref {hex(xo)} --")
        print(dump(xo))
        print("  hits:", [(hex(a), b) for a, b in scan_range(xo - 0x400, xo + 0x400)])

# 2) PrepareSimStartIndParameters
print("\n=== PrepareSimStartIndParameters ===", flush=True)
for off, s in find_str(b"PrepareSimStartIndParameters"):
    xrefs, lo, hi = va_xrefs(off)
    print(f"@{hex(off)} xrefs={list(map(hex, xrefs))}")
    for xo in xrefs[:6]:
        print(f"\n-- {hex(xo)} --")
        print(dump(xo, 0x20, 0x60))
        print("  hits:", [(hex(a), b) for a, b in scan_range(xo - 0x200, xo + 0x300)])

# 3) PIN_STATUS / START name strings via litpool neighbors + LDR
print("\n=== Msg-name litpools ===", flush=True)
for label, needle in (
    ("PIN_STATUS", b"USIM ==> SIM_PIN_STATUS_IND"),
    ("START", b"USIM ==> SIM_START_IND"),
    ("PRESENT", b"USIM ==> SIM_PRESENT_IND"),
):
    off = find_str(needle)[0][0]
    target = va(off)
    pools = []
    i = 0
    nb = struct.pack("<I", target)
    while True:
        j = img.find(nb, i)
        if j < 0:
            break
        pools.append(j)
        i = j + 1
    print(f"\n{label} @{hex(off)} pools={list(map(hex, pools))}")
    for po in pools:
        for k in range(-6, 7):
            w = u32(po + k * 4)
            if 0x41020000 <= w <= 0x41040000:
                fo = MAIN_OFF + (w - VA_BASE)
                if 0 <= fo < len(img):
                    b = fo
                    while b < fo + 70 and 32 <= img[b] < 127:
                        b += 1
                    s2 = img[fo:b].decode("ascii", "replace")
                    if "SIM_" in s2:
                        print(f"  [{k:+d}] {s2}")

# 4) LDR to pools
print("\n=== LDR to name pools ===", flush=True)
for label, po in (("PIN", 0x10FB2BC), ("START", 0x10FB87C), ("PRESENT", 0x10FB2E8)):
    found = []
    for o in range(MAIN_OFF, END - 4, 2):
        hw, hw2 = u16(o), u16(o + 2)
        if hw == 0xF8DF:
            imm12 = hw2 & 0xFFF
            if ((o + 4) & ~3) + imm12 == po:
                found.append(o)
        if (hw & 0xF800) == 0x4800:
            if ((o + 4) & ~3) + (hw & 0xFF) * 4 == po:
                found.append(o)
    print(f"{label}: {list(map(hex, found))} n={len(found)}")
    for fo in found[:4]:
        print(dump(fo, 0x30, 0x60))
        print("  hits:", [(hex(a), b) for a, b in scan_range(fo - 0x200, fo + 0x200)])

# 5) INIT_REQ / PRESENT send order strings
print("\n=== INIT / PRESENT / START order strings + xrefs ===", flush=True)
for needle in (
    b"USIM_WAIT_FOR_INIT_REQ",
    b"sitInformSimInit",
    b"Sending SIM_PRESENT_IND to PBM from USIM_WAIT_FOR_INIT_REQ case",
    b"Sending SIM_PRESENT_IND to PBM from USIM_CARD_PRESENT case",
    b"Sending SIM_PRESENT_IND to PBM",
    b"SIM_INIT_REQ",
    b"USIM <== SIM_INIT",
    b"USIM ==> SIM_START_IND",
    b"USIM ==> SIM_PIN_STATUS_IND",
):
    hits = find_str(needle)
    for off, s in hits[:2]:
        xrefs, lo, hi = va_xrefs(off)
        print(f"\n{s[:80]}")
        print(f"  @{hex(off)} xrefs({len(xrefs)})={list(map(hex, xrefs[:10]))}")
        for xo in xrefs[:3]:
            h = scan_range(xo - 0x180, xo + 0x180)
            if h:
                print(f"  {hex(xo)} hits={[(hex(a),b) for a,b in h]}")

# 6) USIM region full Present scan
print("\n=== USIM 0x1916000..0x191a000 Present scan ===", flush=True)
print([(hex(a), b) for a, b in scan_range(0x1916000, 0x191A000)])

# 7) BL 0x20e184e with r0=#2 near START — annotate
print("\n=== 0x20e184e callers with MOVS r0,#2 (sample near USIM) ===", flush=True)
c2 = []
for o in bl_index.get(0x20E184E, []):
    arg = None
    for p in range(o - 12, o, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) == 0:
            arg = hw & 0xFF
    if arg == 2:
        c2.append(o)
print(f"count r0=#2: {len(c2)}")
usim_c2 = [o for o in c2 if 0x1910000 <= o <= 0x1A80000 or 0x1A6C000 <= o <= 0x1A70000]
print("USIM-ish:", list(map(hex, usim_c2[:20])))
for o in usim_c2[:8]:
    print(f"\n{hex(o)}")
    print(dump(o, 0x20, 0x10))
    print(" hits", [(hex(a), b) for a, b in scan_range(o - 0x100, o + 0x40)])

# 8) Any FN_A caller in 0x1910000-0x1b00000?
print("\n=== FN_A/FN_B/WRAP_SIM callers in USIM-ish range ===", flush=True)
for tname, t in (("FN_A", FN_A), ("FN_B", FN_B), ("WRAP_SIM", WRAP_SIM), ("SET_APP", SET_APP)):
    cs = [o for o in bl_index.get(t, []) if 0x1900000 <= o <= 0x1C00000]
    print(f"{tname}: {list(map(hex, cs[:20]))} n={len(cs)}")

# 9) Pin1Verified writers near START?
print("\n=== Pin1V sites proximity to START logs ===", flush=True)
for site in (0x1F04576, 0x1F046A4):
    print(hex(site), "hits in ±0x200:", scan_range(site - 0x200, site + 0x200))

print("\nDONE", flush=True)
