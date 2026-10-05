#!/usr/bin/env python3
"""Fully decode SetEngMode 0x908 layout; CP handler vs START_NETWORK GET_APP gate."""
import struct
from pathlib import Path

STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()
IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

# ProtocolMiscDebugBuilder::SetEngMode(unsigned char) @ 0x740f0
FN = 0x740F0


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


print("=== SetEngModeEh raw insn dump ===")
for i in range(0, 0x80, 4):
    insn = u32(STREAM, FN + i)
    # decode common
    notes = []
    # MOVZ
    if (insn & 0xFF800000) == 0x52800000:
        rd = insn & 0x1F
        imm = (insn >> 5) & 0xFFFF
        hw = (insn >> 21) & 0x3
        notes.append(f"MOVZ w{rd},#{hex(imm<<(hw*16))}")
    # STRB Wt,[Xn,#imm]
    if (insn & 0xFFC00000) == 0x39000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = (insn >> 10) & 0xFFF
        notes.append(f"STRB w{rt},[x{rn},#{imm}]")
    # STRH
    if (insn & 0xFFC00000) == 0x79000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = ((insn >> 10) & 0xFFF) * 2
        notes.append(f"STRH w{rt},[x{rn},#{imm}]")
    # STR W
    if (insn & 0xFFC00000) == 0xB9000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = ((insn >> 10) & 0xFFF) * 4
        notes.append(f"STR w{rt},[x{rn},#{imm}]")
    # BL
    if (insn & 0xFC000000) == 0x94000000:
        imm26 = insn & 0x3FFFFFF
        if imm26 & (1 << 25):
            imm26 -= 1 << 26
        tgt = FN + i + (imm26 << 2)
        notes.append(f"BL->{hex(tgt)}")
    print(f"  {hex(FN+i)}: {insn:08x} {' '.join(notes)}")

# SetEngModeEhh @ 0x74170
FN2 = 0x74170
print("\n=== SetEngModeEhh ===")
for i in range(0, 0x90, 4):
    insn = u32(STREAM, FN2 + i)
    notes = []
    if (insn & 0xFF800000) == 0x52800000:
        rd = insn & 0x1F
        imm = (insn >> 5) & 0xFFFF
        notes.append(f"MOVZ w{rd},#{hex(imm)}")
    if (insn & 0xFFC00000) == 0x39000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = (insn >> 10) & 0xFFF
        notes.append(f"STRB w{rt},[x{rn},#{imm}]")
    if (insn & 0xFFC00000) == 0x79000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = ((insn >> 10) & 0xFFF) * 2
        notes.append(f"STRH w{rt},[x{rn},#{imm}]")
    print(f"  {hex(FN2+i)}: {insn:08x} {' '.join(notes)}")

# GetEngMode @ 0x70cd0
print("\n=== GetEngModeEh @0x70cd0 ===")
FN3 = 0x70CD0
for i in range(0, 0x90, 4):
    insn = u32(STREAM, FN3 + i)
    notes = []
    if (insn & 0xFF800000) == 0x52800000:
        rd = insn & 0x1F
        imm = (insn >> 5) & 0xFFFF
        notes.append(f"MOVZ w{rd},#{hex(imm)}")
    if notes:
        print(f"  {hex(FN3+i)}: {notes}")

# CP: find SIT dispatch for 0x908
# Search MOVW #0x908 near SIT handlers, or string SIT_SET_ENG_MODE used in handler
print("\n=== CP SIT_SET_ENG_MODE log/handler strings ===")
for n in [
    b"SIT_SET_ENG_MODE",
    b"SET_ENG_MODE",
    b"EngMode",
    b"ENG MODE",
    b"isn't supported",
]:
    off = 0
    c = 0
    while c < 8:
        j = IMG.find(n, off)
        if j < 0:
            break
        s = IMG[j : j + 90].split(b"\x00")[0]
        if b"ENG" in s.upper() or b"Eng" in s:
            print(hex(j), s)
            c += 1
        off = j + 1

# Thumb MOVW Rd,#0x908
print("\n=== Thumb MOVW #0x908 sites ===")


def movw_imm(o):
    hw = struct.unpack_from("<H", IMG, o)[0]
    hw2 = struct.unpack_from("<H", IMG, o + 2)[0]
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return rd, (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8


sites = []
for o in range(0x1400000, 0x2200000, 2):
    m = movw_imm(o)
    if m and m[1] == 0x908:
        sites.append((o, m[0]))
print(f"count={len(sites)}")
for o, rd in sites[:30]:
    # nearby strings / BL
    print(f"  {hex(o)} r{rd}")

# Compare: does any 0x908 handler call into START_NETWORK region 0x18e83xx?
# Look at sites and dump nearby for GET_APP / START_NETWORK / SIM ready
print("\n=== context around first MOVW #0x908 in SIT range ===")
for o, rd in sites:
    if 0x1800000 <= o <= 0x1C00000 or 0x1400000 <= o <= 0x1600000:
        print(f"candidate {hex(o)}")
        # dump nearby movw/strings via searching for ASCII nearby in ±0x40 of lit pools - skip
        # instead print 0x40 bytes hex
        print(IMG[o : o + 0x40].hex())

# BuildSetImsTestMode 0x614 — also lab-ish; check if NET start
print("\n=== BuildSetImsTestMode @0x72b60 ===")
FN4 = 0x72B60
for i in range(0, 0x50, 4):
    insn = u32(STREAM, FN4 + i)
    if (insn & 0xFF800000) == 0x52800000:
        rd = insn & 0x1F
        imm = (insn >> 5) & 0xFFFF
        print(f"  +{hex(i)} MOVZ w{rd},#{hex(imm)}")

print("DONE")
