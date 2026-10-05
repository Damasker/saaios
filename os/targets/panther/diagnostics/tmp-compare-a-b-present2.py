#!/usr/bin/env python3
"""Compare modem_a vs B for Present=2 / SET_APP#5 / FN_A CDMA coupling."""
import struct
from pathlib import Path

A = Path("fw/saaios-probe-a-modem.bin").read_bytes()
B = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
assert len(A) == len(B) == 98265168
print(f"sizes equal {len(A)}; identical={A==B}")


def u16(img, o):
    return struct.unpack_from("<H", img, o)[0]


def bl(img, o):
    hw, hw2 = u16(img, o), u16(img, o + 2)
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


def movw(img, o):
    hw, hw2 = u16(img, o), u16(img, o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def find_str(img, n):
    j = img.find(n)
    return j


def count_strb_bf6(img):
    out = []
    o = MAIN
    while o < len(img) - 4:
        hw, hw2 = u16(img, o), u16(img, o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            out.append(o)
        o += 2
    return out


def count_set_app5(img, set_app=0x19916D2):
    # B address may differ on A — find by MOVS#5 + BL pattern near known, or find SET_APP by string
    out = []
    o = MAIN
    while o < len(img) - 4:
        hw = u16(img, o)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 5:
            b = bl(img, o + 2)
            if b is None and o + 4 < len(img):
                b = bl(img, o + 4)
            # also check o itself isn't bl
            for p in (o + 2, o + 4):
                b = bl(img, p)
                if b and abs(b - p) > 0x1000:
                    # verify nearby LDRB #0xBF6
                    window = img[max(MAIN, p - 0x30) : p]
                    if b"\x90\x0b\xf6" in window or b"\xf8\x90" in window:
                        out.append((p, b))
        o += 2
    return out


def getobj_636c_sites(img):
    GETOBJ = None
    # find getobj by scanning MOVW #636c + BL nearby; record BL targets
    sites = []
    o = MAIN
    while o < len(img) - 8:
        r = movw(img, o)
        if r and r[0] == 0x636C:
            for d in range(0, 40, 2):
                b = bl(img, o + d)
                if b:
                    sites.append((o, b))
                    break
        o += 2
    return sites


def present2_near_getobj(img, sites):
    """MOVS #2 + STRB.W [rN,#0] within +0x280 of getobj#636c site."""
    hits = []
    for s, gob in sites:
        end = min(s + 0x280, len(img) - 4)
        o = s
        while o < end:
            hw = u16(img, o)
            if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
                for p in range(o + 2, min(o + 16, end), 2):
                    h2 = u16(img, p)
                    if (h2 & 0xF800) in (0xE800, 0xF000, 0xF800):
                        h3 = u16(img, p + 2)
                        if (h2 & 0xFFF0) == 0xF880 and (h3 & 0xFFF) == 0:
                            hits.append((s, o, p, gob))
                        break
            o += 2
    return hits


def bl_to(img, tgt):
    cs = []
    o = MAIN
    while o < len(img) - 4:
        if bl(img, o) == tgt:
            cs.append(o)
        o += 2
    return cs


print("=== strings ===")
for n in [
    b"No CDMA in SupportedRatMap",
    b"CDMA_TIMING_LATCH",
    b"CDMA_MEAS_RESULT",
    b"SIM STATUS update: Present",
    b"g5300q-",
]:
    print(f"  A {n!r}: {hex(find_str(A,n)) if find_str(A,n)>=0 else None}")
    print(f"  B {n!r}: {hex(find_str(B,n)) if find_str(B,n)>=0 else None}")

print("\n=== +0xBF6 STRB ===")
print("A", [hex(x) for x in count_strb_bf6(A)])
print("B", [hex(x) for x in count_strb_bf6(B)])

print("\n=== getobj#636c ===")
sa, sb = getobj_636c_sites(A), getobj_636c_sites(B)
print(f"A sites={len(sa)} {[hex(s) for s,_ in sa]}")
print(f"B sites={len(sb)} {[hex(s) for s,_ in sb]}")

print("\n=== Present=2 near getobj ===")
pa, pb = present2_near_getobj(A, sa), present2_near_getobj(B, sb)
print(f"A Present2 hits={len(pa)} {[(hex(s),hex(o)) for s,o,_,_ in pa]}")
print(f"B Present2 hits={len(pb)} {[(hex(s),hex(o)) for s,o,_,_ in pb]}")

# Compare FN_A region bytes around known B Present=2 store
for off in (0x14F6A14, 0x14FB380, 0x14FB5C6, 0x14C3986, 0x14F692C):
    same = A[off : off + 8] == B[off : off + 8]
    print(f"region {hex(off)} same8={same} A={A[off:off+8].hex()} B={B[off:off+8].hex()}")

# Diff density in L1 SIM wrap / STATUS / FN_A windows
windows = [
    (0x14F0000, 0x1500000, "L1_STATUS"),
    (0x14C3000, 0x14C7000, "SIMWRAP"),
    (0x1F00000, 0x1F10000, "FirstPIN"),
]
for lo, hi, lab in windows:
    diff = sum(1 for i in range(lo, hi) if A[i] != B[i])
    print(f"diff bytes {lab}: {diff}/{hi-lo}")

# On A: find BL targets of Present=2 sites (callers of FN_A-like)
print("\n=== A: CDMA latch strings near code? ===")
for n in [b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF", b"MMC_LTEL1_CDMA_MEAS_RESULT_IND", b"No CDMA in SupportedRatMap"]:
    j = find_str(A, n)
    print(f"  {n.decode(errors='replace')}: {hex(j) if j>=0 else None}")

# Search A for any MOVS#2 STRB.W #0 in USIM range that has getobj 636c in -0x400
print("\n=== A broader MOVS#2+STRB#0 in 0x14f0000-0x1505000 ===")
hits = []
o = 0x14F0000
while o < 0x1505000:
    if u16(A, o) == 0x2002:  # MOVS r0,#2 — also other regs
        pass
    hw = u16(A, o)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        for p in range(o + 2, min(o + 12, 0x1505000), 2):
            h2 = u16(A, p)
            if (h2 & 0xF800) in (0xE800, 0xF000, 0xF800):
                h3 = u16(A, p + 2)
                if (h2 & 0xFFF0) == 0xF880 and (h3 & 0xFFF) == 0:
                    hits.append(o)
                break
    o += 2
print(f"L1 window hits: {len(hits)} {[hex(h) for h in hits[:20]]}")

# Does A have SET_APP READY at same place?
print("\nA vs B at STATUS READY gate 32 bytes:")
print("A", A[0x14FB5BC : 0x14FB5D0].hex())
print("B", B[0x14FB5BC : 0x14FB5D0].hex())
print("equal", A[0x14FB5BC : 0x14FB5D0] == B[0x14FB5BC : 0x14FB5D0])
