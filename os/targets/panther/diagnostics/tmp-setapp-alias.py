#!/usr/bin/env python3
"""Exhaustive SET_APP callers; CDMA msg ID uniqueness vs LTE/NR aliases."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000
SET_APP = 0x19916D2


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


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


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# --- 1) ALL BL → SET_APP ---
print("=== ALL BL → SET_APP 0x19916d2 ===")
callers = []
o = 0x1000000
while o < 0x3C00000 - 4:
    if bl(o) == SET_APP:
        callers.append(o)
    o += 2
print(f"n={len(callers)}")

APP_NAMES = {
    0: "UNKNOWN?",
    1: "DETECTED?",
    2: "PIN",
    3: "PUK?",
    4: "SUBSCRIPTION?",
    5: "READY",
    6: "?",
    7: "?",
}


def infer_r0(site):
    """Walk back up to 0x40 looking for r0 value: MOVS r0,#imm, MOV r0,rx, LDRB, etc."""
    imm = None
    reg_src = None
    notes = []
    for p in range(site - 2, max(0, site - 0x60), -2):
        hw = u16(p)
        # MOVS r0, #imm8
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            notes.append(f"MOVS r0,#{imm} @{hex(p)}")
            break
        # MOVW r0, #imm
        r = movw(p)
        if r and r[1] == 0:
            imm = r[0]
            notes.append(f"MOVW r0,#{hex(imm)} @{hex(p)}")
            break
        # MOV r0, rN (Thumb T1 MOV): 0x4600 | rd | (rm<<3) when rd=0
        if (hw & 0xFFC7) == 0x4600:  # MOV r0, rm
            rm = (hw >> 3) & 7
            reg_src = rm
            notes.append(f"MOV r0,r{rm} @{hex(p)}")
            # look further for MOVS rm,#imm
            for q in range(p - 2, max(0, p - 0x40), -2):
                h2 = u16(q)
                if (h2 & 0xFF00) == (0x2000 | (rm << 8)):
                    imm = h2 & 0xFF
                    notes.append(f"  from MOVS r{rm},#{imm} @{hex(q)}")
                    break
            break
        # IT EQ / conditional MOVS before
        if hw in (0xBF08, 0xBF18, 0xBF28, 0xBF38):  # IT EQ etc
            notes.append(f"IT @{hex(p)}={hex(hw)}")
    return imm, notes


rows = []
for c in callers:
    imm, notes = infer_r0(c)
    # Also check for gate: LDRB #BF6 nearby
    has_bf6 = False
    cmp2 = False
    for p in range(max(0, c - 0x80), c, 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF6:
            has_bf6 = True
        if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) == 2:
            cmp2 = True
    rows.append((c, imm, has_bf6, cmp2, notes))
    name = APP_NAMES.get(imm, f"imm={imm}")
    print(f"  {hex(c)} imm={imm} ({name}) bf6={has_bf6} cmp2={cmp2} | {'; '.join(notes[:3])}")

ready5 = [r for r in rows if r[1] == 5]
print(f"\nREADY #5 callers: {len(ready5)} {[hex(r[0]) for r in ready5]}")
for c, imm, has_bf6, cmp2, notes in ready5:
    dump(c - 0x50, c + 0x10, f"READY5@{hex(c)}")

# Also check STRB +0xBF4 direct (bypass SET_APP)
print("\n=== ALL STRB.W #0xBF4 ===")
o = 0x1000000
bf4w = []
while o < 0x3C00000 - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF4:
        bf4w.append(o)
    o += 2
print([hex(x) for x in bf4w])

# --- 2) CDMA message ID uniqueness ---
print("\n=== CDMA msg name uniqueness / switch binding ===")
msgs = {
    "CDMA_MEAS_RESULT_IND": b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    "CDMA_TIMING_LATCH_CNF": b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    "UMTS_MEASURE_CNF": b"MMC_LTEL1_UMTS_MEASURE_CNF",
    "UMTS_TDD_PARTIAL": b"MMC_LTEL1_UMTS_TDD_PARTIAL_SEARCH_CNF",
}
# Also find any LTE/NR MEAS that share the SAME string suffix or same VA low16 in switch
for label, n in msgs.items():
    j = img.find(n)
    sva = VA + (j - MAIN) if j >= 0 else None
    print(f"{label}: off={hex(j) if j>=0 else None} VA={hex(sva) if sva else None} lo16={hex(sva & 0xFFFF) if sva else None}")

# Count how many times each lo16 appears as MOVW r1,#lo in MMC switch 0x14b0000-0x14d0000
print("\n=== MMC switch: each lo16 binds to ONE case (BL target) ===")
bindings = {}
o = 0x14B7000
while o < 0x14B8000 - 8:
    r = movw(o)
    if r and r[1] == 1:
        t = movt(o + 4)
        if t and t[0] == 0x4106 and t[1] == 1:
            lo = r[0]
            # find following BL within 0x20
            bl_t = None
            for p in range(o + 8, min(o + 0x28, 0x14B8000 - 4), 2):
                bt = bl(p)
                if bt and 0x14C0000 <= bt <= 0x14D0000:
                    bl_t = bt
                    break
            bindings.setdefault(lo, []).append((o, bl_t))
    o += 2

for lo in [0xEC0B, 0xEFB3, 0xEEE0, 0xEF38]:
    ents = bindings.get(lo, [])
    print(f"  lo={hex(lo)}: {len(ents)} cases {[(hex(a), hex(b) if b else None) for a,b in ents]}")

# Check if any OTHER string VA shares lo16 with CDMA msgs (collision)
print("\n=== lo16 collision scan for CDMA msg lows ===")
for lo, label in [(0xEC0B, "MEAS"), (0xEFB3, "LATCH")]:
    # find all strings whose VA & 0xFFFF == lo and hi == 0x4106
    # already unique VA. Check if any OTHER string at VA 0x4106XXXX with same lo
    # Scan string table region for VA ending in lo
    count = 0
    examples = []
    # file offs for VA 0x41060000+lo
    fo = (0x41060000 + lo) - VA + MAIN
    if 0 <= fo < len(img) - 8:
        s = img[fo : fo + 80]
        end = 0
        while end < len(s) and 32 <= s[end] < 127:
            end += 1
        print(f"  VA 0x4106{lo:04x} -> '{s[:end].decode()}'")
    # Any second string with same lo in 0x4106xxxx? Only one VA per lo in that page.
    # Check MOVW r1,#lo with MOVT NOT 0x4106
    other = []
    o = 0x14B0000
    while o < 0x14E0000 - 8:
        r = movw(o)
        if r and r[0] == lo and r[1] == 1:
            t = movt(o + 4)
            hi = t[0] if t and t[1] == 1 else None
            t2 = movt(o - 4) if o >= 4 else None
            if t2 and t2[1] == 1:
                hi = t2[0]
            if hi and hi != 0x4106:
                other.append((o, hi))
        o += 2
    print(f"  {label} lo={hex(lo)} with non-0x4106 MOVT in MMC: {[(hex(a), hex(b)) for a,b in other[:6]]}")

# Confirm WRAP targets uniquely
print("\n=== WRAP targets ===")
print(f"MEAS case BL→WRAP_A 0x14c380e from 0x14b7766 (lo=0xec0b)")
print(f"LATCH case BL→0x14c388e from 0x14b79e2 (lo=0xefb3)")
# Are WRAP_A / LATCH body reached from any other BL?
for tgt, name in [(0x14C380E, "WRAP_A"), (0x14C388E, "LATCH_body"), (0x14F6D02, "WRAP_SIM")]:
    cs = []
    o = 0x1000000
    while o < 0x3C00000 - 4:
        if bl(o) == tgt:
            cs.append(o)
        o += 2
    print(f"  BL→{name}: {len(cs)} {[hex(c) for c in cs]}")
