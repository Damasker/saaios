#!/usr/bin/env python3
"""SIM_INIT_REQ Rx / SIM_PRESENT_IND Tx vs Present=2; WAIT_FOR_INIT logs."""
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
TARGETS = {FN_A, FN_B, WRAP_SIM, SET_APP, 0x14C380E, 0x14C3986}


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


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
        hits.append((o, ctx))
    return hits


def scan(lo, hi):
    hits = []
    for o in range(lo, min(hi, END - 4), 2):
        bt = bl_target(o)
        if bt in TARGETS:
            hits.append((o, f"BL->{hex(bt)}"))
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) in (0xBF4, 0xBF5, 0xBF6):
            hits.append((o, f"STRB.W #{hex(hw2 & 0xFFF)}"))
    return hits


def dump(o, before=0x28, after=0x40):
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
        if note:
            lines.append(f"  {hex(p)}: " + " | ".join(note))
    return "\n".join(lines)


# Find unique log ids near WAIT_FOR_INIT / PRESENT send by scanning A-string region
# Heuristic: Shannon packs log id as halfword immediately before format in some builds —
# try adjacent bytes; also search MOVW of small ids in USIM with string-like context 0x4107

print("=== Strings proving boot order ===")
for needle in (
    b"Waiting for SIM_INIT_REQ",
    b"Wait For SIM_INIT_REQ",
    b"USIM_WAIT_FOR_INIT_REQ",
    b"USIM_CARD_PRESENT",
    b"Sending SIM_PRESENT_IND",
    b"PrepareSimStartIndParameters",
    b"SIM_PIN_STATUS_IND",
    b"SIM_START_IND",
    b"START_STACK_SERVICES",
):
    for off, s in find_str(needle):
        if len(s) < 120:
            print(f"  {s}")

# Probe: are there PIN_STATUS-specific log sites? Search movw near string "PIN_STATUS" in code comments
# Use log ids from dense USIM table — scan 0x1910000-0x1950000 for MOVW that appear with
# helpers 0x189bcf6 / 0x19a2b54 (same as START emit)
print("\n=== Other IND emits via same helpers as 0x105a (USIM band) ===")
helpers = {0x189BCF6, 0x19A2B54, 0x18D2258}
for o in range(0x1910000, 0x1950000, 2):
    bt = bl_target(o)
    if bt not in helpers:
        continue
    # find nearby log movw
    logs = []
    for p in range(o - 0x20, o + 4, 2):
        r = movw(p)
        if r and r[0] < 0x2000:
            ctx = None
            for q in range(p - 16, p + 20, 2):
                t = movt(q)
                if t and t[1] == r[1] and 0x4000 <= t[0] <= 0x45FF:
                    ctx = t[0]
            logs.append((hex(p), hex(r[0]), hex(ctx) if ctx else None))
    if logs:
        h = scan(o - 0x80, o + 0x40)
        mark = " PRESENT_HIT" if h else ""
        print(f"  BL {hex(bt)} @{hex(o)} logs={logs}{mark}")

# WRAP_SIM sole caller context
print("\n=== WRAP_SIM / FN_A caller context (not START) ===")
for label, addr in (("FN_A via WRAP", 0x14C3872), ("FN_A via SIMWRAP", 0x14F6D7A), ("FN_B", 0x14C5FE6)):
    print(f"\n{label} @{hex(addr)}")
    print(dump(addr, 0x40, 0x20))

# PRESENT IND related: log search  — any site with PRESENT and FN_A?
print("\n=== Scan 0x18f0000..0x1960000 for FN_A/Present (INIT/PRESENT band guess) ===")
print("hits", [(hex(a), b) for a, b in scan(0x18F0000, 0x1960000)])

print("\nDONE")
