#!/usr/bin/env python3
"""RO: SIM START IND / SIM_PIN_STATUS_IND handlers → Present=2 / FN_A / Pin1Verified."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A = 0x14F692C
FN_B = 0x14F9108
WRAP_A = 0x14C380E
SET_APP = 0x19916D2
TARGETS = {FN_A, FN_B, WRAP_A, 0x14F6D02, 0x14C3986, SET_APP}


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


def find_str(needle):
    hits = []
    idx = 0
    while True:
        j = img.find(needle, idx)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        hits.append((a, img[a:b].decode(errors="replace")))
        idx = j + 1
    return hits


# Shannon log hdr: often byte 0x44, id in >>8, then string
def log_id_at_string(soff):
    for back in range(4, 16, 4):
        w = u32(soff - back)
        if (w & 0xFF) == 0x44:
            return (w >> 8) & 0xFFFF, soff - back
        if (w & 0xFF) == 0x41:  # sometimes 'A' prefix consumed
            return (w >> 8) & 0xFFFF, soff - back
    return None, None


def real_movw_sites(log_id, limit=30):
    hits = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != log_id:
            continue
        # skip dense false positives
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
        hits.append((o, ctx))
        if len(hits) >= limit:
            break
    return hits


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = ""
                if r[0] in (0x7AB, 0x18E, 0x106A, 0x1068, 0xC6B, 0xC5B, 0xBDA, 0x2C5E):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                t = bl_target(o)
                tag = ""
                if t in TARGETS:
                    tag = " <<TARGET"
                if t == FN_A:
                    tag = " <<FN_A"
                if t == FN_B:
                    tag = " <<FN_B"
                extra = f" ;BL->{t:#x}{tag}"
            elif (hw & 0xFFF0) == 0xF880:
                imm = hw2 & 0xFFF
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{imm:#x}]"
                if imm == 0xBF6:
                    extra += " <<+BF6"
            elif (hw & 0xF800) == 0x7000:
                pass
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if ((hw >> 6) & 0x1F) == 20:
                extra += " <<#20?"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


print("=== String hits ===")
needles = [
    b"SIM START IND",
    b"SIM_PIN_STATUS_IND",
    b"SIM_PRESENT_IND",
    b"USIM ==> SIM_PIN_STATUS_IND",
    b"SimPresent=%d, SimState=%d, Pin1Status",
    b"Pin1Status=%d",
    b"sitTxSimStatus",
    b"Tx SIM Status",
    b"SIM START",
]
for n in needles:
    for off, s in find_str(n)[:4]:
        lid, hdr = log_id_at_string(off if img[off] != 0x41 else off + 1)
        # try off and off-1 for leading A
        for cand in (off, off - 1, off + 1):
            if cand > 0:
                lid2, hdr2 = log_id_at_string(cand)
                if lid2:
                    lid, hdr = lid2, hdr2
                    break
        lid_s = f"{lid:#x}" if lid else "None"
        print(f"  @{off:#x} lid={lid_s}: {s[:100]}")

# Known from prior: SIM START IND string at 0x4d4888b
print("\n=== Resolve log IDs for key strings ===")
for label, soff in [
    ("SIM_START_IND", 0x4D4888B),
    ("SIM_PIN_STATUS_IND_sym", 0x1035939),
    ("SIM_PRESENT_IND_sym", 0xC308D5),
]:
    # find nearest shannon hdr before string
    for back in range(0, 32):
        o = soff - back
        if o < 4:
            break
        w = u32(o)
        if (w & 0xFF) == 0x44:
            print(f"  {label}: hdr@{o:#x} id={(w>>8)&0xFFFF:#x} str={img[soff:soff+80]!r}")
            break
    else:
        # symbol name may not have log hdr — search MOVW of address
        print(f"  {label}: no hdr near string")

# Broader: find all strings containing START IND or PIN_STATUS
print("\n=== More IND strings ===")
for n in [b"START IND", b"PIN_STATUS", b"PIN STATUS IND", b"Pin Status Ind", b"simStart", b"SimStart"]:
    for off, s in find_str(n)[:6]:
        if b"SIM" in s.encode() or b"USIM" in s.encode() or b"Pin" in s or b"PIN" in s:
            print(f"  @{off:#x}: {s[:110]}")
