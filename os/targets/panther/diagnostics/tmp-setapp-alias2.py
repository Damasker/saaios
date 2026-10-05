#!/usr/bin/env python3
"""Harden CDMA lo16 MMC binding + compact SET_APP table."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000
SET_APP = 0x19916D2


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def bl(o):
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
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


print("=== MMC lo16 binding (MOVT +/- MOVW) 0x14b7000..0x14b8000 ===")
for lo, name in [
    (0xEC0B, "MEAS"),
    (0xEFB3, "LATCH"),
    (0xEEE0, "UMTS"),
    (0xEF38, "UMTS_TDD"),
]:
    hits = []
    o = 0x14B7000
    while o < 0x14B8000 - 8:
        r = movw(o)
        if r and r[0] == lo and r[1] == 1:
            hi = None
            for d in (4, -4, 8, -8, 12, -12):
                if o + d < 0:
                    continue
                t = movt(o + d)
                if t and t[1] == 1:
                    hi = t[0]
                    break
            blt = None
            for p in range(max(0, o - 8), min(o + 0x30, 0x14B8000 - 4), 2):
                bt = bl(p)
                if bt and 0x14C0000 <= bt <= 0x14F8000:
                    blt = bt
                    break
            hits.append((o, hi, blt))
        o += 2
    print(
        f"{name} lo={hex(lo)} hits="
        f"{[(hex(a), hex(b) if b is not None else None, hex(c) if c else None) for a, b, c in hits]}"
    )

print("\n=== string uniqueness ===")
for n in [
    b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    b"MMC_LTEL1_LTE_MEAS_RESULT_IND",
    b"MMC_LTEL1_NR_MEAS_RESULT_IND",
    b"MMC_LTEL1_UMTS_MEASURE_CNF",
]:
    j = img.find(n)
    c = img.count(n)
    if j >= 0:
        print(f"{n.decode()}: VA={hex(VA+(j-MAIN))} count={c}")
    else:
        print(f"{n.decode()}: ABSENT count={c}")

print("\n=== WRAP BL uniqueness ===")
for tgt, name in [(0x14C380E, "WRAP_A"), (0x14C388E, "LATCH_body"), (0x14F6D02, "WRAP_SIM")]:
    cs = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        if bl(o) == tgt:
            cs.append(o)
        o += 2
    print(f"BL->{name}: {len(cs)} {[hex(c) for c in cs]}")

# Dump MOVW/MOVT around known sites
print("\n=== around known MEAS/LATCH case sites ===")
for site in (0x14B7752, 0x14B7766, 0x14B79CE, 0x14B79E2):
    print(f"-- {hex(site)}")
    for p in range(site - 0x10, site + 0x18, 2):
        r, t, b = movw(p), movt(p), bl(p)
        extra = ""
        if r:
            extra = f" MOVW r{r[1]},#{hex(r[0])}"
        if t:
            extra = f" MOVT r{t[1]},#{hex(t[0])}"
        if b is not None:
            extra = f" BL->{hex(b)}"
        hw = u16(p)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(p)}:{hw:04x} {u16(p+2):04x}{extra}")
        else:
            print(f" {hex(p)}:{hw:04x}{extra}")

print("\n=== SET_APP imm table ===")
names = {0: "UNK0", 1: "DET1", 2: "PIN", 3: "PUK3", 4: "SUB4", 5: "READY", 6: "S6", 7: "S7"}
callers = []
o = 0x1000000
while o < 0x3C00000 - 4:
    if bl(o) == SET_APP:
        callers.append(o)
    o += 2
ready5 = []
for c in callers:
    imm = None
    bf6 = cmp2 = False
    for p in range(c - 2, max(0, c - 0x60), -2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            break
    for p in range(max(0, c - 0x80), c, 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF6:
            bf6 = True
        if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) == 2:
            cmp2 = True
    if imm == 5:
        ready5.append(c)
    print(f"  {hex(c)} imm={imm} ({names.get(imm, '?')}) PresentGate(bf6&cmp2)={bf6 and cmp2}")
print(f"n={len(callers)} READY#5={len(ready5)} {[hex(x) for x in ready5]}")
print("STRB #BF4 sole @0x1991734 in SET_APP body (prior scan)")
