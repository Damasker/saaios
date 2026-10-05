#!/usr/bin/env python3
"""Find ALL +0xBF6 STRB writers; FN_A callers; STATUS after VerifyPin path."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10


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


def dump(start, end, lab, max_lines=80):
    print(f"\n=== {lab} ===")
    o = start
    n = 0
    while o < end and n < max_lines:
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
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2
        n += 1


# STRB.W [rN, #0xBF6]
print("=== ALL STRB.W [*,#0xBF6] ===")
bf6_writes = []
o = MAIN
while o < len(img) - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0x0FFF) == 0xBF6:
        bf6_writes.append(o)
        print(f"  {hex(o)} STRB.W [r{hw & 0xf},#0xBF6]")
    o += 2
print(f"count={len(bf6_writes)}")

# LDRB.W [*,#0xBF6] for completeness
print("\n=== ALL LDRB.W [*,#0xBF6] ===")
bf6_reads = []
o = MAIN
while o < len(img) - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF890 and (hw2 & 0x0FFF) == 0xBF6:
        bf6_reads.append(o)
    o += 2
print(f"count={len(bf6_reads)} first={[hex(x) for x in bf6_reads[:20]]}")

# Direct BL to FN_A entry 0x14f692c (and nearby thunks)
FN_A = 0x14F692C
# Also search for address literal of FN_A as fptr
print("\n=== Direct BL->FN_A ===")
fn_a_bls = []
o = MAIN
while o < len(img) - 4:
    b = bl(o)
    if b == FN_A or b == (FN_A + 1):
        fn_a_bls.append(o)
        print(f"  {hex(o)} -> FN_A")
    o += 2
print(f"count={len(fn_a_bls)}")

# Absolute VA of FN_A in litpools: VA = 0x40010000 + (FN_A - MAIN) = 0x40010000 + 0x14DFD1C = 0x414efd1c?
fn_a_va = 0x40010000 + (FN_A - MAIN)
print(f"FN_A VA={hex(fn_a_va)}")
pat = struct.pack("<I", fn_a_va)
# also thumb bit set
pat1 = struct.pack("<I", fn_a_va | 1)
idxs = []
pos = 0
while True:
    i = img.find(pat, pos)
    if i < 0:
        break
    idxs.append(i)
    pos = i + 1
idxs1 = []
pos = 0
while True:
    i = img.find(pat1, pos)
    if i < 0:
        break
    idxs1.append(i)
    pos = i + 1
print(f"FN_A VA literals: {len(idxs)} {[hex(i) for i in idxs[:30]]}")
print(f"FN_A|1 literals: {len(idxs1)} {[hex(i) for i in idxs1[:30]]}")

# SIM-wrap / STATUS_WRAP callers via BL
for name, addr in [("SIMWRAP", 0x14C3986), ("STATUS_WRAP", 0x14C6626), ("STATUS_CFG", 0x14FB4F8), ("FN_A", FN_A)]:
    bls = []
    o = MAIN
    while o < len(img) - 4:
        b = bl(o)
        if b == addr:
            bls.append(o)
        o += 2
    print(f"BL->{name}({hex(addr)}): {len(bls)} {[hex(x) for x in bls[:25]]}")

# After VerifyPin Pin1Verified set — does anything call STATUS_CFG?
# Trace 0x1dcb028 (SimInfo) for BL targets of interest
print("\n=== BLs from 0x1dcb028 window ===")
for o in range(0x1DCB028, 0x1DCB400, 2):
    b = bl(o)
    if b:
        tag = ""
        if b in (FN_A, 0x14C3986, 0x14C6626, 0x14FB380, 0x19916D2, 0x20EA040):
            tag = " ***"
        # interesting ranges
        if tag or (0x14C0000 <= b <= 0x1500000) or (0x1990000 <= b <= 0x19A0000) or abs(b - o) > 0x100000:
            print(f"  {hex(o)} -> {hex(b)}{tag}")

dump(0x1DCB028, 0x1DCB120, "SimInfo 0x1dcb028")
dump(0x1F1458C, 0x1F14680, "post 0x1f1458c")

# Check PresentObj store via STRB.W [rN,#0] where rN loaded from getobj result stored globally
# Global PresentObj pointer? Search stores of getobj result with size 0x636c
print("\n=== getobj #0x636c call sites ===")
o = MAIN
sites = []
while o < len(img) - 4:
    r = movw(o)
    if r and r[0] == 0x636C:
        # look for BL getobj nearby
        for d in range(0, 32, 2):
            b = bl(o + d)
            if b == 0x20EA040:
                sites.append(o)
                break
    o += 2
print(f"getobj#636c sites: {len(sites)} {[hex(s) for s in sites]}")
for s in sites:
    dump(s - 0x10, s + 0x40, f"getobj636c@{hex(s)}", max_lines=40)
