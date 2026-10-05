#!/usr/bin/env python3
"""Trace FN_A dispatch table; arg0==3 sources; global PresentObj pointer stores."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


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


def dump(start, end, lab, max_lines=90):
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


# Dispatch caller of FN_A wrap @0x14b7766
dump(0x14B7700, 0x14B7800, "dispatch->FN_A_wrap")
dump(0x14B79A0, 0x14B7A40, "dispatch->0x14c388e")

# Find message ID / switch that routes to 0x14c3986 (SIMWRAP -> 0x14f6d02)
# Search for absolute pointers to 0x14c3986|1 or 0x14f6d02|1 in tables
for tgt_off, name in [
    (0x14C3986, "SIMWRAP"),
    (0x14F6D02, "MEAS_FN"),
    (0x14C380E, "LATCH_FN"),
    (0x14C3872, "BL_site"),
    (0x14F692C, "FN_A"),
]:
    va = VA + (tgt_off - MAIN)
    for thumb in (0, 1):
        pat = struct.pack("<I", (va | thumb))
        idxs = []
        pos = 0
        while len(idxs) < 30:
            i = img.find(pat, pos)
            if i < 0:
                break
            idxs.append(i)
            pos = i + 1
        if idxs:
            print(f"{name} VA={hex(va)}|{thumb} literals@{[hex(i) for i in idxs[:20]]}")

# CDMA_MEAS_RESULT_IND string -> find handler that sets arg0=3
j = img.find(b"MMC_LTEL1_CDMA_MEAS_RESULT_IND")
print(f"\nCDMA_MEAS_RESULT_IND str off={hex(j) if j>=0 else None}")
if j and j > 0:
    a = j
    while a > 0 and 32 <= img[a - 1] < 127:
        a -= 1
    b = j
    while b < len(img) and 32 <= img[b] < 127:
        b += 1
    print(" full:", img[a:b].decode("ascii", "replace")[:120])

# Search all strings containing CDMA_MEAS
pos = 0
while True:
    j = img.find(b"CDMA_MEAS", pos)
    if j < 0:
        break
    a = j
    while a > 0 and 32 <= img[a - 1] < 127:
        a -= 1
    b = j
    while b < len(img) and 32 <= img[b] < 127:
        b += 1
    s = img[a:b].decode("ascii", "replace")
    if "RESULT" in s or "IND" in s:
        print(f"  @{hex(a)}: {s[:110]}")
    pos = j + 1

# FN_A Present=2 gate: disasm around 0x14f6a0e CMP arg0,#3
dump(0x14F69E0, 0x14F6A50, "FN_A Present=2 CMP#3")

# Global PresentObj: after getobj in FN_A, is pointer stored to a global?
# Look for STR of r8/r0 to absolute address near FN_A Present write
# Also scan ALL STRB.W [rN,#0] preceded by LDR of a global that was written from getobj#636c
# Find stores of getobj result: after BL getobj at sites, STR.W [abs]

print("\n=== post-getobj pointer stores (possible global PresentObj) ===")
GETOBJ = 0x20EA040
o = MAIN
while o < len(img) - 8:
    r = movw(o)
    if r and r[0] == 0x636C:
        for d in range(0, 48, 2):
            if bl(o + d) == GETOBJ:
                # dump 80 bytes after getobj for STR.W to PC-relative / abs
                dump(o + d, o + d + 0x60, f"post_getobj@{hex(o)}", max_lines=40)
                break
    o += 2

# Broader: any MOVS #2 + STRB.W [rN,#0] where rN was loaded from literal pool
# pointing into L1LC IratController region — already did windows.
# Check FN_B Present=2 again and whether LTE_MEASURE_CNF can reach it
print("\n=== FN_B Present=2 reachability quick ===")
dump(0x14F9550, 0x14F95A0, "FN_B Present=2")

# Search for BL FN_A with MOVS r0,#3 in preceding 32 bytes (non-wrap producers)
print("\n=== sites with MOVS#3 near BL FN_A ===")
for call in (0x14C3872, 0x14F6D7A):
    dump(call - 0x40, call + 8, f"pre_FN_A@{hex(call)}", max_lines=50)

# Who calls 0x14b7766? Find function + its callers (msg switch)
print("\n=== callers of 0x14b7766 region entry ===")
# find push before 0x14b7700
entry = None
for o in range(0x14B7700, 0x14B7000, -2):
    hw = u16(o)
    if (hw & 0xFF00) == 0xB500 or hw == 0xE92D or (hw & 0xFFF0) == 0xE92D:
        # e92d is 32-bit
        entry = o if (hw & 0xF800) not in (0xE800, 0xF000, 0xF800) else o
        if u16(o) == 0xE92D or (u16(o) & 0xFFF0) == 0xE92D:
            entry = o
        break
# scan for BL targets into 0x14b76xx-0x14b78xx from switch tables via ADR
# Look for PC-relative stubs

# Message ID constants near CDMA_TIMING_LATCH_CNF handler registration
for needle in [
    b"Decode MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
    b"Decode MMC_LTEL1_CDMA_MEAS_RESULT_IND",
    b"CDMA_MEAS_RESULT_IND",
    b"rat_mode:%d",
]:
    j = img.find(needle)
    print(f"str {needle!r}: {hex(j) if j>=0 else None}")
