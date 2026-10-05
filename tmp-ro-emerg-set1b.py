#!/usr/bin/env python3
"""Follow-up: 0x18e8348 path; SET1 vs Pin1Verified clear via callees; Emergency SIT."""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(o):
    if o + 4 > len(IMG):
        return None
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


def dump(lo, hi, label):
    print(f"=== {label} {hex(lo)}..{hex(hi)} ===")
    o = lo
    while o < hi:
        hw = u16(o)
        t = bl_target(o)
        bits = []
        if t is not None:
            bits.append(f"BL->{hex(t)}")
            print(f"  {hex(o)}: {hw:04x} {u16(o+2):04x} {' '.join(bits)}")
            o += 4
            continue
        if (hw & 0xFF00) == 0x2800:
            bits.append(f"CMP r0,#{hw & 0xFF}")
        if (hw & 0xFF00) == 0x2000:
            bits.append(f"MOVS r0,#{hw & 0xFF}")
        if (hw & 0xF000) == 0xD000:
            bits.append(f"Bcond {(hw>>8)&0xF}")
        if (hw & 0xF800) == 0xE000:
            bits.append("B")
        if bits:
            print(f"  {hex(o)}: {hw:04x} {' '.join(bits)}")
        o += 2


# Full START_NETWORK-ish function: find prologue back from 0x18e831a
dump(0x18E8280, 0x18E83C0, "START_NETWORK region")

# Who BL->0x18e8310-ish entry? Find function start by scanning BLs into 0x18e82xx
print("\n=== callers into 0x18e82c0..0x18e8360 ===")
entry_hits = []
for o in range(0x1800000, 0x1A00000, 2):
    t = bl_target(o)
    if t is not None and 0x18E82C0 <= t <= 0x18E8360:
        entry_hits.append((o, t))
print(f"count={len(entry_hits)}")
for o, t in entry_hits[:20]:
    print(f"  {hex(o)} -> {hex(t)}")

# Does SET1_FN or callees STRB Pin1Verified=0?
# Search STRB.W #+0x14 and known FirstPIN clear in 0x19a1000 and 0x20e1800
print("\n=== STRB.W imm12=0x14 / 0xBF5 in SET1 callee neighborhoods ===")


def find_strb(imm, lo, hi):
    hits = []
    o = lo
    while o + 4 <= hi:
        hw, hw2 = u16(o), u16(o + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0x0F00) == 0x0F00 and (hw2 & 0xFFF) == imm:
            hits.append(o)
        # also STRB Rt,[Rn,#imm5] with imm5==20 (0x14)
        if imm == 0x14 and (hw & 0xFE00) == 0x7000 and ((hw >> 6) & 0x1F) == 0x14:
            hits.append(o)
        o += 2
    return hits


for name, lo, hi in [
    ("SET1 body", 0x146A99C, 0x146AB20),
    ("callee 0x19a10fc nbhd", 0x19A1000, 0x19A1300),
    ("callee 0x19a1d90 nbhd", 0x19A1D00, 0x19A1F00),
    ("0x20e184e nbhd", 0x20E1800, 0x20E1A00),
    ("FirstPIN/verify band", 0x18E0000, 0x18F0000),
]:
    h14 = find_strb(0x14, lo, hi)
    hbf5 = find_strb(0xBF5, lo, hi)
    print(f"  {name}: +14={list(map(hex,h14))} +BF5={list(map(hex,hbf5))}")

# Xref EmergencyModeUpdateIndHandler / EMERGENCY_SCAN_REQ to code (string refs hard);
# look for SIT id patterns near 'Emergency' in AP factory if present in this IMG (CP only).
print("\n=== limited service monitor / SIM invalid ===")
for s in [
    b"limited service monitor: No need (SIM is invalid)",
    b"START_NETWORK Ignored: SIM is not ready",
    b"EmergencyMode",
]:
    i = IMG.find(s)
    print(f"  {s[:50]!r} -> {hex(i) if i>=0 else None}")

# Check 0x18e8484 (CMP 2,7) - what is it?
dump(0x18E8460, 0x18E8500, "GET_APP CMP2/7 site")

print("DONE")
