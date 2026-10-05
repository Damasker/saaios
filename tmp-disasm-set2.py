#!/usr/bin/env python3
"""Disasm STATUS SET#2 cluster; plan DISABLED→READY patch on B MAIN COPY."""
from __future__ import annotations
import struct
from pathlib import Path

p = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin")
data = bytearray(p.read_bytes())

main_b_off = main_size = main_crc_off = None
for i in range(16):
    e = i * 32
    name = data[e : e + 12].split(b"\0", 1)[0]
    if name == b"MAIN":
        b_off, m_off, size, crc = struct.unpack_from("<IIII", data, e + 12)
        # CRC field position: after name(12)+b_off+m_off+size = offset e+24
        main_b_off, main_size = b_off, size
        main_crc_off = e + 24
        print(f"MAIN b_off={hex(b_off)} m_off={hex(m_off)} size={hex(size)} crc={hex(crc)} crc_at={hex(main_crc_off)}")
        break

SET_APP = 0x19916D2


def bl_target(off: int, hw: int, hw2: int) -> int | None:
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= (1 << 31):
            imm32 -= 1 << 32
    return (off + 4 + imm32) & 0xFFFFFFFF


def dump(start: int, end: int) -> None:
    print(f"\n=== dump {hex(start)}-{hex(end)} ===")
    o = start
    while o < end:
        hw = data[o] | (data[o + 1] << 8)
        hw2 = data[o + 2] | (data[o + 3] << 8) if o + 3 < len(data) else 0
        extra = ""
        t = bl_target(o, hw, hw2)
        if t is not None:
            mark = " SET_APP" if t == SET_APP else ""
            extra += f" BL->{hex(t)}{mark}"
        if (hw & 0xF800) == 0x2000:
            extra += f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFFF0) == 0xF1B0 and (hw2 & 0xFF00) == 0x0F00:
            extra += f" CMP.W #{hw2&0xff}"
        if (hw & 0xFFF0) == 0xF890:
            extra += f" LDRB.W #+{hw2&0xfff:x}"
        if (hw & 0xF000) == 0xD000:
            cond = (hw >> 8) & 0xF
            imm = hw & 0xFF
            # signed
            if imm & 0x80:
                imm = imm - 256
            tgt = o + 4 + (imm << 1)
            extra += f" Bcond{cond}->{hex(tgt)}"
        if (hw & 0xF800) == 0xE000:
            imm = hw & 0x7FF
            if imm & 0x400:
                imm = imm - 0x800
            tgt = o + 4 + (imm << 1)
            extra += f" B->{hex(tgt)}"
        print(f"  {hex(o)}: {hw:04x}{extra}")
        o += 2


# Known sites from prior RO (file offsets in full modem.bin)
for lab, fo in [("SET2", 0x14FB406), ("SET5", 0x14FB5C6), ("BF6", 0x14FB380)]:
    print(lab, hex(fo), data[fo : fo + 8].hex())

dump(0x14FB380, 0x14FB430)
dump(0x14FB580, 0x14FB5E0)

# Find MOVS #2 immediately before BL SET_APP at 0x14fb406
print("\n=== bytes at SET#2 site ===")
print(data[0x14FB3F0:0x14FB410].hex())
# Look for MOVS Rd,#2 then BL
o = 0x14FB3E0
while o < 0x14FB410:
    hw = data[o] | (data[o + 1] << 8)
    if (hw & 0xF800) == 0x2000 and (hw & 0xFF) == 2:
        print(f"MOVS #2 at {hex(o)}: {hw:04x} rd={(hw>>8)&7}")
    if (hw & 0xF800) == 0x2000 and (hw & 0xFF) == 5:
        print(f"MOVS #5 at {hex(o)}: {hw:04x}")
    o += 2
PY
