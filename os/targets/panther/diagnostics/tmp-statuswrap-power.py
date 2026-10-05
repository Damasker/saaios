#!/usr/bin/env python3
"""STATUS_WRAP callers; FN_A internal; SetSimCardPower / SetUicc opcodes."""
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


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            b = bl(o)
            extra = f" ;BL->{hex(b)}" if b is not None else ""
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Callers of STATUS_WRAP
for addr in (0x14B7CA8, 0x1A3E6CC):
    dump(addr - 0x40, addr + 0x30, f"STATUS_WRAP_caller@{hex(addr)}")

# FN_A callers
for addr in (0x14C3872, 0x14F6D7A):
    dump(addr - 0x60, addr + 0x40, f"FN_A_caller@{hex(addr)}")

# sit-stream: disasm BuildSetSimCardPower / BuildSetUicc for opcode
stream = Path("sit-stream.so").read_bytes()

def arm64_or_arm32_find_mov_imm_near(data, sym_off, window=0x80):
    """Find 16-bit immediate stores that look like SIT ids near function."""
    # sit-stream is ARM32 typically for Android vendor
    # Look for patterns like MOVW rX, #0x02xx or STRH of constants
    hits = []
    end = min(len(data) - 4, sym_off + window)
    o = sym_off
    while o < end:
        hw = struct.unpack_from("<H", data, o)[0]
        # ARM32 MOVW encoding same as thumb? sit-stream may be ARM Thumb2
        hw2 = struct.unpack_from("<H", data, o + 2)[0]
        if (hw & 0xFBF0) == 0xF240 and (hw2 & 0x8000) == 0:
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            if 0x200 <= imm <= 0x900 or imm in (0x0800, 0x0801, 0x070A, 0x070B, 0x0704, 0x0701):
                hits.append((o, imm))
        # also plain movs-like in ARM: rarely
        o += 2
    return hits

# From syms: BuildSetSimCardPower 0x80640, BuildSetUicc 0x805b0, BuildSimGetStatus 0x7dbb0
for name, off in [
    ("BuildSimGetStatus", 0x7DBB0),
    ("BuildSetSimCardPower", 0x80640),
    ("BuildSetUicc", 0x805B0),
    ("BuildSimGetSlotStatus", 0x7D480),
    ("BuildSimVerifyPin", 0x7DC30),
]:
    hits = arm64_or_arm32_find_mov_imm_near(stream, off, 0x100)
    print(f"\n{name}@{hex(off)} imm16 near: {[(hex(a), hex(i)) for a,i in hits[:20]]}")
    # dump first 32 halfwords
    print(" raw:", " ".join(f"{struct.unpack_from('<H', stream, off+i)[0]:04x}" for i in range(0, 64, 2)))

# Search strings for SetSimCardPower / card power in libsitril
ril = Path("libsitril.so").read_bytes()
for needle in [
    b"SetSimCardPower",
    b"BuildSetSimCardPower",
    b"SetUicc",
    b"SIM_CARD_POWER",
    b"card power",
    b"DoSetSimCardPower",
    b"REQUEST_SET_SIM_CARD_POWER",
]:
    j = ril.find(needle)
    print(f"ril {needle}: {hex(j) if j>=0 else None}")
