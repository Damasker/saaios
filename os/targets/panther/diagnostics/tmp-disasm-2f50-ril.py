#!/usr/bin/env python3
"""Disassemble libsitril MOVZ #0x2f50 send path for OEM framing evidence."""
from __future__ import annotations

import struct
from pathlib import Path

ril = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
).read_bytes()


def u32(o):
    return struct.unpack_from("<I", ril, o)[0]


def dump(o, n=96):
    return " ".join(f"{x:02x}" for x in ril[o : o + n])


def cstr(o, n=80):
    s = bytearray()
    for i in range(o, min(len(ril), o + n)):
        c = ril[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def decode(o):
    insn = u32(o)
    if (insn & 0xFC000000) == 0x94000000:
        imm26 = insn & 0x3FFFFFF
        if imm26 & (1 << 25):
            imm26 -= 1 << 26
        return f"BL {hex(o + imm26 * 4)}"
    if (insn & 0xFC000000) == 0x14000000:
        imm26 = insn & 0x3FFFFFF
        if imm26 & (1 << 25):
            imm26 -= 1 << 26
        return f"B {hex(o + imm26 * 4)}"
    if (insn & 0xFF800000) == 0x52800000:
        imm = (insn >> 5) & 0xFFFF
        rd = insn & 0x1F
        return f"MOVZ w{rd},#{hex(imm)}"
    if (insn & 0xFF800000) == 0xD2800000:
        imm = (insn >> 5) & 0xFFFF
        rd = insn & 0x1F
        return f"MOVZ x{rd},#{hex(imm)}"
    if (insn & 0xFF800000) == 0x72800000:
        imm = (insn >> 5) & 0xFFFF
        rd = insn & 0x1F
        hw = (insn >> 21) & 3
        return f"MOVK w{rd},#{hex(imm)},LSL#{hw * 16}"
    if (insn & 0xFF000000) == 0x91000000:
        rd = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = (insn >> 10) & 0xFFF
        sh = (insn >> 22) & 1
        shift = "<<12" if sh else ""
        return f"ADD x{rd},x{rn},#{imm}{shift}"
    if (insn & 0xFF000000) == 0x11000000:
        rd = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = (insn >> 10) & 0xFFF
        return f"ADD w{rd},w{rn},#{imm}"
    if (insn & 0xFFE00C00) == 0x39000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = (insn >> 10) & 0xFFF
        return f"STRB w{rt},[x{rn},#{imm}]"
    if (insn & 0xFFE00C00) == 0x79000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = ((insn >> 10) & 0xFFF) << 1
        return f"STRH w{rt},[x{rn},#{imm}]"
    if (insn & 0xFFE00C00) == 0xB9000000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = ((insn >> 10) & 0xFFF) << 2
        return f"STR w{rt},[x{rn},#{imm}]"
    if (insn & 0xFFE00C00) == 0xB9400000:
        rt = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        imm = ((insn >> 10) & 0xFFF) << 2
        return f"LDR w{rt},[x{rn},#{imm}]"
    if (insn & 0x7F800000) == 0x2A000000:
        rd = insn & 0x1F
        rn = (insn >> 5) & 0x1F
        rm = (insn >> 16) & 0x1F
        if rn == 31:
            return f"MOV w{rd},w{rm}"
        return f"ORR w{rd},w{rn},w{rm}"
    if insn == 0xD65F03C0:
        return "RET"
    if (insn & 0x9F000000) == 0x90000000:
        rd = insn & 0x1F
        immlo = (insn >> 29) & 3
        immhi = (insn >> 5) & 0x7FFFF
        imm = (immhi << 2) | immlo
        if imm & (1 << 20):
            imm -= 1 << 21
        page = (o & ~0xFFF) + (imm << 12)
        return f"ADRP x{rd},#{hex(page)}"
    return f"? {hex(insn)}"


print("=== all MOVZ #0x2f50 sites ===")
for rd in range(32):
    needle = struct.pack("<I", 0x52800000 | (0x2F50 << 5) | rd)
    pos = 0
    while True:
        i = ril.find(needle, pos)
        if i < 0:
            break
        print(f"\nMOVZ w{rd},#0x2f50 @{hex(i)}")
        for o in range(i - 0x40, i + 0x80, 4):
            if 0 <= o < len(ril) - 4:
                print(f"  {hex(o)}: {decode(o)}")
        # resolve ADRP+ADD string refs in window
        for o in range(i - 0x40, i + 0x80, 4):
            d = decode(o)
            if d.startswith("ADRP"):
                # next ADD?
                if o + 4 < len(ril):
                    d2 = decode(o + 4)
                    if d2.startswith("ADD x"):
                        # parse pages roughly from text
                        try:
                            page = int(d.split("#")[1], 16)
                            # ADD xA,xB,#imm
                            imm_s = d2.split("#")[1]
                            imm = int(imm_s)
                            rd_a = int(d2.split("x")[1].split(",")[0])
                            addr = page + imm
                            if 0 <= addr < len(ril):
                                s = cstr(addr, 80)
                                if s:
                                    print(f"    str@{hex(addr)}: {s!r}")
                        except Exception:
                            pass
        pos = i + 1

# Also MOVZ #0x2f58 (START_STACK OEM?) and #0x2f57
print("\n=== MOVZ OEM neighbors 0x2f57/58/52 ===")
for imm in (0x2F52, 0x2F57, 0x2F58, 0x2F42, 0x2F40):
    hits = []
    for rd in range(32):
        needle = struct.pack("<I", 0x52800000 | (imm << 5) | rd)
        pos = 0
        while True:
            i = ril.find(needle, pos)
            if i < 0:
                break
            hits.append((i, rd))
            pos = i + 1
    print(f"MOVZ #{hex(imm)} count={len(hits)} {[hex(i) for i,_ in hits[:8]]}")

print("\nDONE")
